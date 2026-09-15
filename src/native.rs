use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn try_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

async fn call<A: Serialize, T: DeserializeOwned>(cmd: &str, args: &A) -> Result<T, String> {
    let args = serde_wasm_bindgen::to_value(args).map_err(|e| e.to_string())?;
    let res = try_invoke(cmd, args).await.map_err(error_message)?;
    serde_wasm_bindgen::from_value(res).map_err(|e| e.to_string())
}

pub fn error_message(err: JsValue) -> String {
    err.as_string()
        .or_else(|| js_sys::Reflect::get(&err, &"message".into()).ok().and_then(|m| m.as_string()))
        .unwrap_or_else(|| format!("{err:?}"))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveFileArgs<'a> {
    file_name: &'a str,
    contents: &'a str,
}

/// Lets the user pick where to save the file, `false` if they cancelled
pub async fn save_file(file_name: &str, contents: &str) -> Result<bool, String> {
    call("save_file", &SaveFileArgs { file_name, contents }).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadFileArgs<'a> {
    url: &'a str,
    headers: &'a HashMap<String, Vec<String>>,
    file_name: &'a str,
}

/// Downloads the file and lets the user pick where to save it, `false` if they cancelled
pub async fn download_file(url: &str, headers: &HashMap<String, Vec<String>>, file_name: &str) -> Result<bool, String> {
    call("download_file", &DownloadFileArgs { url, headers, file_name }).await
}

/// Lets the user pick a file and reads it, `None` if they cancelled
pub async fn open_file() -> Result<Option<String>, String> {
    call("open_file", &()).await
}

fn user_agent() -> String {
    web_sys::window()
        .and_then(|w| w.navigator().user_agent().ok())
        .unwrap_or_default()
}

/// The native QR scanner only exists in the mobile app
pub fn has_native_scanner() -> bool {
    let ua = user_agent();
    ua.contains("Android") || ua.contains("iPhone") || ua.contains("iPad")
}

/// Whether this is the Android app, which is the only platform where notifications are polled
/// outside the webview (see `background_polling`)
pub fn is_android() -> bool {
    user_agent().contains("Android")
}

#[derive(Deserialize)]
struct Permissions {
    camera: String,
}

pub async fn request_camera_permission() -> Result<(), String> {
    let permissions: Permissions = call("plugin:barcode-scanner|request_permissions", &()).await?;
    if permissions.camera == "granted" {
        Ok(())
    } else {
        Err("Camera access was denied".to_string())
    }
}

#[derive(Serialize)]
struct ScanArgs {
    windowed: bool,
    formats: [&'static str; 1],
}

#[derive(Deserialize)]
struct Scanned {
    content: String,
}

/// Needs the camera permission. The camera is shown behind the webview, so the page has to be see-through meanwhile.
/// A cancelled scan never finishes.
pub async fn scan_qr_code() -> Result<String, String> {
    let scanned: Scanned = call("plugin:barcode-scanner|scan", &ScanArgs { windowed: true, formats: ["QR_CODE"] }).await?;
    Ok(scanned.content)
}

pub async fn cancel_scan() -> Result<(), String> {
    call("plugin:barcode-scanner|cancel", &()).await
}

/// Checks whether the OS has already granted permission to show notifications
pub async fn is_notification_permission_granted() -> bool {
    call::<(), Option<bool>>("plugin:notification|is_permission_granted", &())
        .await
        .ok()
        .flatten()
        .unwrap_or(false)
}

/// Asks the user for permission to show notifications, unless it was already granted.
/// Returns whether notifications may be shown.
pub async fn ensure_notification_permission() -> bool {
    if is_notification_permission_granted().await {
        return true;
    }
    let state: Result<String, String> = call("plugin:notification|request_permission", &()).await;
    state.as_deref() == Ok("granted")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NotifyOptions<'a> {
    title: &'a str,
    body: &'a str,
}

#[derive(Serialize)]
struct NotifyArgs<'a> {
    options: NotifyOptions<'a>,
}

/// Shows a native OS notification. `ensure_notification_permission` should be called first.
pub async fn send_notification(title: &str, body: &str) -> Result<(), String> {
    call("plugin:notification|notify", &NotifyArgs { options: NotifyOptions { title, body } }).await
}

#[derive(Serialize)]
struct SyncStoreArgs<'a> {
    entries: &'a HashMap<String, String>,
}

/// Copies the settings and the current Untis session into the native store, where the Android
/// background poller can read them without a webview
pub async fn sync_store(entries: &HashMap<String, String>) -> Result<(), String> {
    call("sync_store", &SyncStoreArgs { entries }).await
}

#[derive(Serialize)]
struct BackgroundPollingArgs<'a> {
    mode: &'a str,
}

/// Picks how Android keeps polling once the app is closed ("off", "periodic" or "continuous").
/// Returns whether polling now runs natively; `false` on every other platform, where the webview
/// has to stay alive for notifications to happen at all.
pub async fn set_background_polling(mode: &str) -> Result<bool, String> {
    call("set_background_polling", &BackgroundPollingArgs { mode }).await
}
