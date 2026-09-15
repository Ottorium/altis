mod background;
mod http;

use std::collections::HashMap;
use std::io::{Read, Write as _};
use tauri_plugin_dialog::{DialogExt, FilePath};
use tauri_plugin_fs::{FsExt, OpenOptions};

#[derive(serde::Serialize)]
struct ProxyResponse {
    headers: HashMap<String, Vec<String>>,
    body: String,
}

/// CORS won't let the webview talk to Untis directly, so every request goes through here
#[tauri::command]
async fn proxy(
    method: String,
    url: String,
    headers: HashMap<String, Vec<String>>,
    body: String,
) -> Result<ProxyResponse, String> {
    let response = http::request(&method, &url, headers, body).await?;
    Ok(ProxyResponse { headers: response.headers, body: response.body })
}

/// Hands the Android background poller the settings and session it needs to keep checking on its
/// own. A no-op on every other platform, where the poller runs in the webview.
#[tauri::command]
fn sync_store(entries: HashMap<String, String>) -> Result<(), String> {
    background::sync_store(entries)
}

/// Switches Android background polling between "off", "periodic" and "continuous", returning
/// whether polling now happens natively. Always `false` elsewhere, where the frontend has to keep
/// doing it itself.
#[tauri::command]
fn set_background_polling(mode: String) -> Result<bool, String> {
    background::set_mode(&mode)
}

fn write_file(app: &tauri::AppHandle, path: FilePath, contents: &[u8]) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    let mut file = app.fs().open(path, options).map_err(|e| e.to_string())?;
    file.write_all(contents).map_err(|e| e.to_string())
}

/// Asks where to save the file, `false` if the user cancelled
#[tauri::command]
async fn save_file(app: tauri::AppHandle, file_name: String, contents: String) -> Result<bool, String> {
    let Some(path) = app.dialog().file()
        .set_file_name(file_name)
        .add_filter("JSON", &["json"])
        .blocking_save_file() else {
        return Ok(false);
    };

    write_file(&app, path, contents.as_bytes())?;
    Ok(true)
}

/// Downloads the file, then asks where to save it, `false` if the user cancelled.
/// Files are binary, so unlike the proxy's text body they can't be passed to the webview
#[tauri::command]
async fn download_file(
    app: tauri::AppHandle,
    url: String,
    headers: HashMap<String, Vec<String>>,
    file_name: String,
) -> Result<bool, String> {
    let contents = http::client()?.get(&url)
        .headers(http::header_map(headers))
        .send()
        .await
        .and_then(|res| res.error_for_status())
        .map_err(|e| http::report(&e))?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    let Some(path) = app.dialog().file()
        .set_file_name(file_name)
        .blocking_save_file() else {
        return Ok(false);
    };

    write_file(&app, path, &contents)?;
    Ok(true)
}

/// Asks for a file and reads it, `None` if the user cancelled
#[tauri::command]
async fn open_file(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let Some(path) = app.dialog().file().blocking_pick_file() else {
        return Ok(None);
    };

    let mut options = OpenOptions::new();
    options.read(true);
    let mut contents = String::new();
    app.fs().open(path, options)
        .and_then(|mut file| file.read_to_string(&mut contents))
        .map_err(|e| e.to_string())?;
    Ok(Some(contents))
}

/// Keeps the app running in the background after the window is closed, so the notification poller
/// (which lives in the frontend's WASM, i.e. only runs while the webview is alive) keeps checking
/// for timetable changes. The app only fully quits via the tray menu's "Quit" item.
#[cfg(desktop)]
fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::Manager;
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let quit_item = MenuItem::with_id(app, "quit", "Quit Altis", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&quit_item])?;

    let mut tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Altis")
        .on_menu_event(|app, event| {
            if event.id.as_ref() == "quit" {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

/// WebKitGTK denies camera access unless it's allowed explicitly, the webcam is used to scan QR codes
#[cfg(target_os = "linux")]
fn allow_camera(app: &tauri::App) {
    use tauri::Manager;
    use webkit2gtk::glib::object::Cast;
    use webkit2gtk::{PermissionRequestExt, SettingsExt, UserMediaPermissionRequest, WebViewExt};

    let Some(window) = app.get_webview_window("main") else { return };
    let _ = window.with_webview(|webview| {
        let webview = webview.inner();
        if let Some(settings) = webview.settings() {
            settings.set_enable_media_stream(true);
        }
        webview.connect_permission_request(|_, request| {
            if request.downcast_ref::<UserMediaPermissionRequest>().is_some() {
                request.allow();
                true
            } else {
                false
            }
        });
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    http::install_crypto_provider();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init());
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_barcode_scanner::init());

    #[cfg(desktop)]
    let builder = builder.on_window_event(|window, event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window.hide();
        }
    });

    builder
        .setup(|_app| {
            #[cfg(target_os = "linux")]
            allow_camera(_app);
            #[cfg(desktop)]
            setup_tray(_app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![proxy, save_file, download_file, open_file, sync_store, set_background_polling])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
