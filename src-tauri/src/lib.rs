use reqwest::{header::{HeaderMap, HeaderName, HeaderValue}, Method};
use rustls::ClientConfig;
use std::collections::HashMap;
use std::fmt::Write;
use std::io::{Read, Write as _};
use tauri_plugin_dialog::{DialogExt, FilePath};
use tauri_plugin_fs::{FsExt, OpenOptions};

#[derive(serde::Serialize)]
struct ProxyResponse {
    headers: HashMap<String, Vec<String>>,
    body: String,
}

fn http_client() -> Result<reqwest::Client, String> {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    reqwest::Client::builder()
        .use_preconfigured_tls(config)
        .build()
        .map_err(|e| e.to_string())
}

fn header_map(headers: HashMap<String, Vec<String>>) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    for (key, values) in headers {
        if let Ok(name) = HeaderName::from_bytes(key.as_bytes()) {
            for value_str in values {
                if let Ok(val) = HeaderValue::from_str(&value_str) {
                    header_map.append(name.clone(), val);
                }
            }
        }
    }
    header_map
}

#[tauri::command]
async fn proxy(
    method: String,
    url: String,
    headers: HashMap<String, Vec<String>>,
    body: String,
) -> Result<ProxyResponse, String> {
    let http_method = Method::from_bytes(method.to_uppercase().as_bytes())
        .map_err(|_| format!("Invalid HTTP method: {}", method))?;

    let res = http_client()?.request(http_method, &url)
        .headers(header_map(headers))
        .body(body)
        .send()
        .await
        .map_err(|e| report(&e))?;

    let mut resp_headers: HashMap<String, Vec<String>> = HashMap::new();
    for (name, value) in res.headers().iter() {
        if let Ok(val_str) = value.to_str() {
            resp_headers
                .entry(name.to_string())
                .or_default()
                .push(val_str.to_string());
        }
    }

    Ok(ProxyResponse {
        headers: resp_headers,
        body: res.text().await.map_err(|e| e.to_string())?,
    })
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
    let contents = http_client()?.get(&url)
        .headers(header_map(headers))
        .send()
        .await
        .and_then(|res| res.error_for_status())
        .map_err(|e| report(&e))?
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

fn report(err: &dyn std::error::Error) -> String {
    let mut s = format!("{}", err);
    let mut current = err.source();
    while let Some(src) = current {
        let _ = write!(s, "\n\nCaused by: {}", src);
        current = src.source();
    }
    s
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init());
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_barcode_scanner::init());

    builder
        .setup(|_app| {
            #[cfg(target_os = "linux")]
            allow_camera(_app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![proxy, save_file, download_file, open_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
