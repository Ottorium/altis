use crate::data_models::clean_models::untis::{ChangeStatus, Class, Entity, LessonBlock, WeekTimeTable};
use crate::untis::untis_week::Week;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{NaiveDateTime, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Read;
use wasm_bindgen::JsCast;
use web_sys::{HtmlDocument, Storage};


pub type TimeTables = (HashMap<Class, WeekTimeTable>, Option<i32>);

#[allow(dead_code)]
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TimeTableCache {
    pub tables: HashMap<Week, (Option<NaiveDateTime>, TimeTables)>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub untis_auth: AuthSettings,
    pub b2e_auth: AuthSettings,
    pub visual_settings: VisualSettings,
}

pub const ALL_WEEKDAYS: [Weekday; 7] = [Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri, Weekday::Sat, Weekday::Sun];
pub const WORK_WEEKDAYS: [Weekday; 5] = [Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri];

pub fn default_always_show_weekdays() -> Vec<Weekday> {
    WORK_WEEKDAYS.to_vec()
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct WeekdayOverride {
    #[serde(default)]
    pub only_lessons: bool,
    #[serde(default = "default_always_show_weekdays")]
    pub always_show: Vec<Weekday>,
}

impl Default for WeekdayOverride {
    fn default() -> Self {
        Self { only_lessons: false, always_show: default_always_show_weekdays() }
    }
}

impl WeekdayOverride {
    pub fn should_show(&self, weekday: Weekday, has_lessons: bool) -> bool {
        has_lessons || (!self.only_lessons && self.always_show.contains(&weekday))
    }

    pub fn is_weekday_always_shown(&self, weekday: Weekday) -> bool {
        self.always_show.contains(&weekday)
    }

    pub fn toggle_weekday(&mut self, weekday: Weekday) {
        if let Some(pos) = self.always_show.iter().position(|&w| w == weekday) {
            self.always_show.remove(pos);
        } else {
            self.always_show.push(weekday);
            self.always_show.sort_by_key(|w| w.num_days_from_monday());
        }
    }
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct VisualSettings {
    #[serde(default)]
    pub force_ascii_timetable: bool,
    #[serde(default)]
    pub weekday_override: WeekdayOverride,
    #[serde(default)]
    pub subject_color_overrides: BTreeMap<String, String>,
}

impl VisualSettings {
    /// Look up a color override for a given subject name (case-insensitive).
    pub fn get_subject_color_override(&self, subject_name: &str) -> Option<&String> {
        let trimmed = subject_name.trim();
        if trimmed.is_empty() {
            return None;
        }
        self.subject_color_overrides.iter().find_map(|(k, v)| {
            if k.trim().eq_ignore_ascii_case(trimmed) {
                Some(v)
            } else {
                None
            }
        })
    }

    /// Check if a lesson block has an entity that matches any subject color override.
    /// Prefers active (non-removed) subjects over removed ones.
    pub fn get_lesson_color_override(&self, lesson: &LessonBlock) -> Option<String> {
        let mut subjects: Vec<_> = lesson
            .entities
            .iter()
            .filter_map(|e| {
                if let Entity::Subject(sub) = &e.inner {
                    Some((&e.status, sub))
                } else {
                    None
                }
            })
            .collect();

        // Sort so non-removed subjects are checked first
        subjects.sort_by_key(|(status, _)| **status == ChangeStatus::Removed);

        for (_, sub) in subjects {
            if let Some(color) = self.get_subject_color_override(&sub.short_name) {
                return Some(color.clone());
            }
            if let Some(color) = self.get_subject_color_override(&sub.display_name) {
                return Some(color.clone());
            }
            if let Some(color) = self.get_subject_color_override(&sub.long_name) {
                return Some(color.clone());
            }
        }
        None
    }
}

pub fn is_dark_color(hex: &str) -> bool {
    let hex = hex.trim_start_matches('#');
    if hex.len() >= 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255) as f64;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255) as f64;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255) as f64;
        let luminance = 0.299 * r + 0.587 * g + 0.114 * b;
        luminance < 140.0
    } else {
        false
    }
}


#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AuthSettings {
    pub school_identifier: String,
    pub user_identifier: String,
    pub secret: String,
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct PersistenceManager {}

impl PersistenceManager {
    pub fn get_cookies() -> Option<Cookies> {
        let storage = Self::get_storage().ok()?;

        let jsessionid = storage.get_item("JSESSIONID").ok()??;
        let tenant_id = storage.get_item("Tenant-Id").ok()??;
        let school_name = storage.get_item("schoolname").ok()??;

        Some(Cookies {
            jsessionid,
            tenant_id,
            school_name_base32: school_name,
        })
    }

    pub fn save_cookies(cookies: &Cookies) -> Result<(), String> {
        let storage = Self::get_storage()?;

        storage
            .set_item("JSESSIONID", &cookies.jsessionid)
            .map_err(|_| "Failed to save JSESSIONID")?;

        storage
            .set_item("Tenant-Id", &cookies.tenant_id)
            .map_err(|_| "Failed to save Tenant-Id")?;

        storage
            .set_item("schoolname", &cookies.school_name_base32)
            .map_err(|_| "Failed to save schoolname")?;

        Ok(())
    }

    pub fn clear_cookies() {
        if let Ok(storage) = Self::get_storage() {
            let _ = storage.remove_item("JSESSIONID");
            let _ = storage.remove_item("Tenant-Id");
            let _ = storage.remove_item("schoolname");
        }

        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Ok(html_doc) = document.dyn_into::<HtmlDocument>() {
                    let cookie_names = ["JSESSIONID", "Tenant-Id", "schoolname"];
                    for name in cookie_names {
                        let _ = html_doc.set_cookie(&format!("{}=; Max-Age=0; path=/; SameSite=Lax", name));
                    }
                }
            }
        }
    }

    pub fn save_settings(settings: &Settings) -> Result<(), String> {
        if let Ok(Some(existing)) = Self::get_settings()
            && existing.untis_auth != settings.untis_auth {
            Self::clear_cookies();
        }

        let serialized =
            serde_json::to_string(settings).map_err(|e| format!("Serialization failed: {}", e))?;

        Self::get_storage()?
            .set_item("user_settings", &serialized)
            .map_err(|_| "Failed to write to localStorage")?;

        Ok(())
    }

    pub fn get_settings() -> Result<Option<Settings>, String> {
        let value = Self::get_storage()?
            .get_item("user_settings")
            .map_err(|_| "Error reading from localStorage")?;

        match value {
            Some(v) => Ok(Some(serde_json::from_str::<Settings>(&v).map_err(|e| format!("Failed to parse settings: {}", e))?)),
            None => Ok(None),
        }
    }

    pub fn save_timetables(tt: &TimeTableCache) -> Result<(), String> {
        let bytes = postcard::to_allocvec(tt)
            .map_err(|e| format!("Postcard failed: {}", e))?;

        let compressed = zstd::encode_all(&bytes[..], 3)
            .map_err(|e| format!("Compression failed: {}", e))?;

        let encoded = STANDARD.encode(compressed);

        Self::get_storage()?
            .set_item("cached_timetables", &encoded)
            .map_err(|_| "Failed to write to localStorage".to_string())?;

        Ok(())
    }

    pub fn get_timetables() -> Result<Option<TimeTableCache>, String> {
        let storage = Self::get_storage()?;
        let value = storage.get_item("cached_timetables")
            .map_err(|_| "Error reading from localStorage".to_string())?;

        match value {
            Some(v) => {
                let compressed_bytes = STANDARD.decode(v.trim())
                    .map_err(|e| format!("Base64 decode failed: {}", e))?;

                let mut decompressed = Vec::new();
                zstd::Decoder::new(&compressed_bytes[..])
                    .map_err(|e| e.to_string())?
                    .read_to_end(&mut decompressed)
                    .map_err(|e| format!("Decompression failed: {}", e))?;

                let decoded = postcard::from_bytes(&decompressed)
                    .map_err(|e| format!("Postcard failed: {}", e))?;

                Ok(Some(decoded))
            }
            None => Ok(None),
        }
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
        Self::get_storage()?
            .clear()
            .map_err(|_| "Failed to clear localStorage".to_string())
    }

    fn get_storage() -> Result<Storage, String> {
        let window = web_sys::window().ok_or("No global window found")?;
        let storage = window
            .local_storage()
            .map_err(|_| "LocalStorage access denied (check permissions)")?
            .ok_or("LocalStorage is not available in this environment")?;
        Ok(storage)
    }
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Cookies {
    pub jsessionid: String,
    pub tenant_id: String,
    pub school_name_base32: String,
}

impl Cookies {
    pub fn to_header_value(&self) -> String {
        format!(
            "JSESSIONID={}; Tenant-Id={}; schoolname={}",
            self.jsessionid, self.tenant_id, self.school_name_base32
        )
    }
}
