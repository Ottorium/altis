use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct ProxyResponse {
    pub headers: HashMap<String, Vec<String>>,
    pub body: Value,
}

#[derive(Serialize)]
#[allow(dead_code)]
struct ProxyArgs<'a> {
    url: &'a str,
    pub headers: &'a HashMap<String, Vec<String>>,
    pub body: &'a Value,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

pub async fn request_proxy(
    url: &str,
    headers: HashMap<String, Vec<String>>,
    body: Value,
) -> Result<ProxyResponse, String> {
    let args = serde_wasm_bindgen::to_value(&ProxyArgs {
        url,
        headers: &headers,
        body: &body,
    })
    .map_err(|e| e.to_string())?;
    let response_js = invoke("proxy", args).await;
    serde_wasm_bindgen::from_value::<ProxyResponse>(response_js).map_err(|e| e.to_string())
}
