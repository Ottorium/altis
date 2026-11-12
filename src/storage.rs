//! What the frontend keeps in localStorage on top of the shared [`Store`]: the timetable caches,
//! which are compressed and never leave the webview, and the timetables picked for comparing.

use crate::components::timetable::Compared;
use crate::native;
use crate::web_env::WebEnv;
use altis_core::env::Env;
use altis_core::models::timetable::{Class, Entity, MyTimeTable, WeekTimeTable};
use altis_core::settings::Settings;
use altis_core::store::{Store, JSESSIONID_KEY, SCHOOLNAME_KEY, SHARED_KEYS, TENANT_ID_KEY};
use altis_core::untis::week::Week;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::NaiveDateTime;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::io::Read;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlDocument;

const COMPARED_KEY: &str = "compared_timetables";
const CLASS_TIMETABLES_KEY: &str = "cached_timetables";
const MY_TIMETABLES_KEY: &str = "cached_my_timetables";

const ZSTD_LEVEL: i32 = 3;

/// The timetable of every class of a week, and the id of the class the student is in
pub type ClassTimeTables = (HashMap<Class, WeekTimeTable>, Option<i32>);

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ClassTimeTableCache {
    /// the timetables along with the time they expire
    pub tables: HashMap<Week, (Option<NaiveDateTime>, ClassTimeTables)>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MyTimeTableCache {
    /// the timetables along with the time they expire
    pub tables: HashMap<Week, (NaiveDateTime, MyTimeTable)>,
}

pub fn settings() -> Result<Option<Settings>, String> {
    Store::<WebEnv>::get_settings()
}

/// The settings, or the defaults where there are none or they don't parse
pub fn settings_or_default() -> Settings {
    Store::<WebEnv>::settings_or_default()
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    // a session belongs to the login it was made with
    if let Ok(Some(existing)) = self::settings()
        && existing.untis_auth != settings.untis_auth {
        clear_cookies();
    }

    Store::<WebEnv>::save_settings(settings)?;
    sync_to_background();
    Ok(())
}

/// Ends the Untis session, both the stored one and whatever the webview itself holds of it
pub fn clear_cookies() {
    Store::<WebEnv>::clear_cookies();

    if let Some(document) = web_sys::window().and_then(|window| window.document())
        && let Ok(document) = document.dyn_into::<HtmlDocument>() {
        for name in [JSESSIONID_KEY, TENANT_ID_KEY, SCHOOLNAME_KEY] {
            let _ = document.set_cookie(&format!("{name}=; Max-Age=0; path=/; SameSite=Lax"));
        }
    }

    sync_to_background();
}

/// The settings the Android background poller needs to keep checking on its own
pub fn background_store_entries() -> HashMap<String, String> {
    SHARED_KEYS.iter().filter_map(|key| WebEnv.get(key).map(|value| (key.to_string(), value))).collect()
}

/// Hands the Android background poller a copy of those, so it can keep checking once the
/// webview is gone. A no-op everywhere else, where the poller runs in this very webview and
/// reads localStorage directly.
pub fn sync_to_background() {
    if !native::is_android() {
        return;
    }

    let entries = background_store_entries();
    spawn_local(async move {
        let _ = native::sync_store(&entries).await;
    });
}

pub fn class_timetables() -> Result<Option<ClassTimeTableCache>, String> {
    load_compressed(CLASS_TIMETABLES_KEY)
}

pub fn save_class_timetables(cache: &ClassTimeTableCache) -> Result<(), String> {
    save_compressed(CLASS_TIMETABLES_KEY, cache)
}

pub fn my_timetables() -> Result<Option<MyTimeTableCache>, String> {
    load_compressed(MY_TIMETABLES_KEY)
}

pub fn save_my_timetables(cache: &MyTimeTableCache) -> Result<(), String> {
    save_compressed(MY_TIMETABLES_KEY, cache)
}

/// A week of every class's timetable is far too large for localStorage as JSON, so the caches
/// are stored as compressed postcard
fn save_compressed<T: Serialize>(key: &str, value: &T) -> Result<(), String> {
    let bytes = postcard::to_allocvec(value).map_err(|e| format!("Postcard failed: {e}"))?;
    let compressed = zstd::encode_all(&bytes[..], ZSTD_LEVEL).map_err(|e| format!("Compression failed: {e}"))?;

    WebEnv.set(key, &STANDARD.encode(compressed))
}

fn load_compressed<T: DeserializeOwned>(key: &str) -> Result<Option<T>, String> {
    let Some(value) = WebEnv.get(key) else { return Ok(None) };

    let compressed = STANDARD.decode(value.trim()).map_err(|e| format!("Base64 decode failed: {e}"))?;
    let mut bytes = Vec::new();
    zstd::Decoder::new(&compressed[..])
        .map_err(|e| e.to_string())?
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Decompression failed: {e}"))?;

    postcard::from_bytes(&bytes).map(Some).map_err(|e| format!("Postcard failed: {e}"))
}

/// The timetables picked for comparing last time, so they don't have to be picked again
pub fn compared() -> Vec<Compared> {
    WebEnv.get(COMPARED_KEY).and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or_default()
}

pub fn save_compared(compared: &[Compared]) -> Result<(), String> {
    let serialized = serde_json::to_string(compared).map_err(|e| format!("Serialization failed: {e}"))?;
    WebEnv.set(COMPARED_KEY, &serialized)
}

/// Every subject the cached timetables mention, to suggest when a colour is set for one
pub fn known_subjects() -> Vec<String> {
    let Ok(Some(cache)) = class_timetables() else { return Vec::new() };

    let subjects: BTreeSet<String> = cache
        .tables
        .values()
        .flat_map(|(_, (timetables, _))| timetables.values())
        .flat_map(|timetable| &timetable.days)
        .flat_map(|day| &day.lessons)
        .flat_map(|lesson| &lesson.entities)
        .filter_map(|entity| match &entity.inner {
            Entity::Subject(subject) => Some(subject.short_name.trim().to_string()),
            _ => None,
        })
        .filter(|name| !name.is_empty())
        .collect();
    subjects.into_iter().collect()
}

/// Forgets everything: the settings, the session and the caches
pub fn clear_all() -> Result<(), String> {
    clear_cookies();
    WebEnv::storage()?.clear().map_err(|_| "Failed to clear localStorage".to_string())?;
    sync_to_background();
    Ok(())
}
