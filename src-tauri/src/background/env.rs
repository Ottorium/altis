use super::{bridge, store};
use altis_core::env::{Env, HttpResponse};
use std::collections::HashMap;

/// The Android background poller's platform: its own JSON file for storage, the app's reqwest
/// client for HTTP and a JNI call into the foreground service for notifications. Deliberately
/// does not go through Tauri, because the whole point is to keep working once the activity (and
/// with it the Tauri app handle and the webview) is gone.
#[derive(Default, Clone, Copy)]
pub struct NativeEnv;

impl Env for NativeEnv {
    fn get(&self, key: &str) -> Option<String> {
        store::get(key)
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        store::set(key, value)
    }

    fn remove(&self, key: &str) {
        store::remove(key);
    }

    fn now_ms(&self) -> u64 {
        chrono::Utc::now().timestamp_millis().max(0) as u64
    }

    async fn request(
        &self,
        method: &str,
        url: &str,
        headers: HashMap<String, Vec<String>>,
        body: String,
    ) -> Result<HttpResponse, String> {
        crate::http::request(method, url, headers, body).await
    }

    async fn notify(&self, title: &str, body: &str) {
        let _ = bridge::post_notification(title, body);
    }
}
