use crate::native;
use altis_core::env::{Env, Headers, HttpResponse};
use web_sys::Storage;

/// The browser implementation of the shared platform trait: localStorage for storage, the
/// shell's proxy for requests and its notification plugin for notifications
#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub struct WebEnv;

impl WebEnv {
    pub fn storage() -> Result<Storage, String> {
        let window = web_sys::window().ok_or("No global window found")?;
        window
            .local_storage()
            .map_err(|_| "LocalStorage access denied (check permissions)")?
            .ok_or("LocalStorage is not available in this environment".to_string())
    }
}

impl Env for WebEnv {
    fn get(&self, key: &str) -> Option<String> {
        Self::storage().ok()?.get_item(key).ok()?
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        Self::storage()?.set_item(key, value).map_err(|_| format!("Failed to write {key} to localStorage"))
    }

    fn remove(&self, key: &str) {
        if let Ok(storage) = Self::storage() {
            let _ = storage.remove_item(key);
        }
    }

    fn now_ms(&self) -> u64 {
        js_sys::Date::now() as u64
    }

    async fn request(&self, method: &str, url: &str, headers: Headers, body: String) -> Result<HttpResponse, String> {
        native::proxy_request(method, url, &headers, &body).await
    }

    async fn notify(&self, title: &str, body: &str) {
        let _ = native::send_notification(title, body).await;
    }
}
