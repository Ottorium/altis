use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[derive(Deserialize, Debug)]
pub struct ProxyResponse {
    pub headers: HashMap<String, Vec<String>>,
    pub body: String,
}

#[derive(Serialize)]
struct ProxyArgs<'a> {
    method: &'a str,
    url: &'a str,
    pub headers: &'a HashMap<String, Vec<String>>,
    pub body: &'a str,
}

#[wasm_bindgen]
extern "C" {
    // `catch`, because a command that returns an Err rejects the promise, and a rejection on a
    // binding without it takes the whole module down instead of coming back as one. Every caller
    // already handles the error this hands back.
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

// CORS disallows the requests to other domains, so we need a proxy
pub async fn request_proxy(
    method: &str,
    url: &str,
    headers: HashMap<String, Vec<String>>,
    body: String,
) -> Result<ProxyResponse, String> {
    let args = serde_wasm_bindgen::to_value(&ProxyArgs {
        method,
        url,
        headers: &headers,
        body: &body,
    })
        .map_err(|e| e.to_string())?;

    let response_js = invoke("proxy", args).await.map_err(crate::native::error_message)?;
    serde_wasm_bindgen::from_value::<ProxyResponse>(response_js).map_err(|e| e.to_string())
}