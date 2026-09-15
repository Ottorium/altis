//! The browser side of the shared storage: `WebEnv` plugs localStorage, the Tauri `proxy` command
//! and the notification plugin into [`altis_core::env::Env`], and `PersistenceManager` keeps the
//! app's own timetable caches, which are compressed and never leave the webview.

use crate::request_proxy::request_proxy;
use altis_core::data_models::clean_models::untis::{Class, Entity, MyTimeTable, WeekTimeTable};
use altis_core::env::{Env, HttpResponse};
use altis_core::settings::Settings;
use altis_core::store::{Store, SHARED_KEYS};
use altis_core::untis::untis_week::Week;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::NaiveDateTime;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::io::Read;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlDocument, Storage};

pub type TimeTables = (HashMap<Class, WeekTimeTable>, Option<i32>);

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TimeTableCache {
    pub tables: HashMap<Week, (Option<NaiveDateTime>, TimeTables)>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MyTimeTableCache {
    /// the timetables along with the time they expire
    pub tables: HashMap<Week, (NaiveDateTime, MyTimeTable)>,
}

/// The browser implementation of the shared platform trait
#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub struct WebEnv;

impl WebEnv {
    fn storage() -> Result<Storage, String> {
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
        Self::storage()?
            .set_item(key, value)
            .map_err(|_| format!("Failed to write {key} to localStorage"))
    }

    fn remove(&self, key: &str) {
        if let Ok(storage) = Self::storage() {
            let _ = storage.remove_item(key);
        }
    }

    fn now_ms(&self) -> u64 {
        js_sys::Date::now() as u64
    }

    async fn request(
        &self,
        method: &str,
        url: &str,
        headers: HashMap<String, Vec<String>>,
        body: String,
    ) -> Result<HttpResponse, String> {
        let response = request_proxy(method, url, headers, body).await?;
        Ok(HttpResponse { headers: response.headers, body: response.body })
    }

    async fn notify(&self, title: &str, body: &str) {
        let _ = crate::native::send_notification(title, body).await;
    }
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct PersistenceManager {}

impl PersistenceManager {
    pub fn clear_cookies() {
        Store::<WebEnv>::clear_cookies();

        if let Some(window) = web_sys::window()
            && let Some(document) = window.document()
            && let Ok(html_doc) = document.dyn_into::<HtmlDocument>() {
            for name in ["JSESSIONID", "Tenant-Id", "schoolname"] {
                let _ = html_doc.set_cookie(&format!("{}=; Max-Age=0; path=/; SameSite=Lax", name));
            }
        }

        Self::sync_to_background();
    }

    pub fn save_settings(settings: &Settings) -> Result<(), String> {
        if let Ok(Some(existing)) = Self::get_settings()
            && existing.untis_auth != settings.untis_auth {
            Self::clear_cookies();
        }

        Store::<WebEnv>::save_settings(settings)?;
        Self::sync_to_background();
        Ok(())
    }

    pub fn get_settings() -> Result<Option<Settings>, String> {
        Store::<WebEnv>::get_settings()
    }

    /// The settings the Android background poller needs to keep checking on its own
    pub fn background_store_entries() -> HashMap<String, String> {
        let env = WebEnv;
        SHARED_KEYS
            .iter()
            .filter_map(|key| env.get(key).map(|value| (key.to_string(), value)))
            .collect()
    }

    /// Hands the Android background poller a copy of those, so it can keep checking once the
    /// webview is gone. A no-op everywhere else, where the poller runs in this very webview and
    /// reads localStorage directly.
    pub fn sync_to_background() {
        if !crate::native::is_android() {
            return;
        }

        let entries = Self::background_store_entries();
        spawn_local(async move {
            let _ = crate::native::sync_store(&entries).await;
        });
    }

    pub fn save_timetables(tt: &TimeTableCache) -> Result<(), String> {
        Self::save_compressed("cached_timetables", tt)
    }

    pub fn get_timetables() -> Result<Option<TimeTableCache>, String> {
        Self::get_compressed("cached_timetables")
    }

    pub fn save_my_timetables(tt: &MyTimeTableCache) -> Result<(), String> {
        Self::save_compressed("cached_my_timetables", tt)
    }

    pub fn get_my_timetables() -> Result<Option<MyTimeTableCache>, String> {
        Self::get_compressed("cached_my_timetables")
    }

    fn save_compressed<T: Serialize>(key: &str, value: &T) -> Result<(), String> {
        let bytes = postcard::to_allocvec(value)
            .map_err(|e| format!("Postcard failed: {}", e))?;

        let compressed = zstd::encode_all(&bytes[..], 3)
            .map_err(|e| format!("Compression failed: {}", e))?;

        WebEnv.set(key, &STANDARD.encode(compressed))
    }

    fn get_compressed<T: DeserializeOwned>(key: &str) -> Result<Option<T>, String> {
        let Some(value) = WebEnv.get(key) else { return Ok(None) };

        let compressed_bytes = STANDARD.decode(value.trim())
            .map_err(|e| format!("Base64 decode failed: {}", e))?;

        let mut decompressed = Vec::new();
        zstd::Decoder::new(&compressed_bytes[..])
            .map_err(|e| e.to_string())?
            .read_to_end(&mut decompressed)
            .map_err(|e| format!("Decompression failed: {}", e))?;

        postcard::from_bytes(&decompressed)
            .map(Some)
            .map_err(|e| format!("Postcard failed: {}", e))
    }

    pub fn get_known_subjects() -> Vec<String> {
        let mut subjects = BTreeSet::new();
        if let Ok(Some(cache)) = Self::get_timetables() {
            for (_, tables) in cache.tables.values() {
                for week_table in tables.0.values() {
                    for day in &week_table.days {
                        for lesson in &day.lessons {
                            for entity in &lesson.entities {
                                if let Entity::Subject(sub) = &entity.inner {
                                    let name = sub.short_name.trim();
                                    if !name.is_empty() {
                                        subjects.insert(name.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        subjects.into_iter().collect()
    }

    pub fn clear_storage() -> Result<(), String> {
        Self::clear_cookies();
        WebEnv::storage()?
            .clear()
            .map_err(|_| "Failed to clear localStorage".to_string())?;
        Self::sync_to_background();
        Ok(())
    }
}
