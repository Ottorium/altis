use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
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

/// Lets the user pick a file and reads it, `None` if they cancelled
pub async fn open_file() -> Result<Option<String>, String> {
    call("open_file", &()).await
}

/// The native QR scanner only exists in the mobile app
pub fn has_native_scanner() -> bool {
    web_sys::window()
        .and_then(|w| w.navigator().user_agent().ok())
        .is_some_and(|ua| ua.contains("Android") || ua.contains("iPhone") || ua.contains("iPad"))
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
