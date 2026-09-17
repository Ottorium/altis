//! The app's HTTP client. Used both by the `proxy` command, which exists because CORS won't let
//! the webview talk to Untis directly, and by the Android background poller, which has no webview
//! to proxy for in the first place.

use altis_core::env::HttpResponse;
use reqwest::{header::{HeaderMap, HeaderName, HeaderValue}, Method};
use rustls::ClientConfig;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::OnceLock;

/// Installs the TLS backend. `run()` does this for the app, but on Android the background service
/// can start the process on its own, without ever going through it.
pub fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

pub fn client() -> Result<reqwest::Client, String> {
    static CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();

    CLIENT
        .get_or_init(|| {
            let mut root_store = rustls::RootCertStore::empty();
            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

            let config = ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();

            reqwest::Client::builder()
                .use_preconfigured_tls(config)
                // Untis going quiet must not wedge a whole poll: without these, a request that
                // never comes back hangs until Android kills the background job, which takes the
                // poll's bookkeeping down with it (see `persist` in `altis_core::notifications`)
                .connect_timeout(std::time::Duration::from_secs(15))
                .timeout(std::time::Duration::from_secs(45))
                .build()
                .map_err(|e| e.to_string())
        })
        .clone()
}

pub fn header_map(headers: HashMap<String, Vec<String>>) -> HeaderMap {
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

pub async fn request(
    method: &str,
    url: &str,
    headers: HashMap<String, Vec<String>>,
    body: String,
) -> Result<HttpResponse, String> {
    let http_method = Method::from_bytes(method.to_uppercase().as_bytes())
        .map_err(|_| format!("Invalid HTTP method: {}", method))?;

    let res = client()?
        .request(http_method, url)
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

    Ok(HttpResponse {
        headers: resp_headers,
        body: res.text().await.map_err(|e| e.to_string())?,
    })
}

/// reqwest's own `Display` stops at the outermost error, which for a failed request is usually
/// just "error sending request" without the DNS or TLS cause anyone actually needs
pub fn report(err: &dyn std::error::Error) -> String {
    let mut s = format!("{}", err);
    let mut current = err.source();
    while let Some(src) = current {
        let _ = write!(s, "\n\nCaused by: {}", src);
        current = src.source();
    }
    s
}
