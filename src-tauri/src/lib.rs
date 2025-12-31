use reqwest::{header::{HeaderMap, HeaderName, HeaderValue}, Method};
use std::collections::HashMap;

#[derive(serde::Serialize)]
struct ProxyResponse {
    headers: HashMap<String, Vec<String>>,
    body: String,
}

#[tauri::command]
async fn proxy(
    method: String,
    url: String,
    headers: HashMap<String, Vec<String>>,
    body: String,
) -> Result<ProxyResponse, String> {
    let client = reqwest::Client::new();

    let http_method = Method::from_bytes(method.to_uppercase().as_bytes())
        .map_err(|_| format!("Invalid HTTP method: {}", method))?;

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

    let res = client.request(http_method, &url)
        .headers(header_map)
        .body(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![proxy])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

