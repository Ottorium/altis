use crate::data_models::clean_models::untis::{ChangeStatus, Class, Entity, LessonBlock, MyTimeTable, WeekTimeTable};
use crate::untis::untis_week::Week;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{NaiveDateTime, Weekday};
use serde::de::DeserializeOwned;
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
pub struct MyTimeTableCache {
    /// the timetables along with the time they expire
    pub tables: HashMap<Week, (NaiveDateTime, MyTimeTable)>,
}

/// Bookkeeping for the background notification poller (see `crate::notifications`), so it knows
/// what has changed since the last check and doesn't repeat itself
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct NotificationState {
    /// the personal timetable as of the last poll, per week, diffed against on the next one
    pub last_my_timetable: HashMap<Week, MyTimeTable>,
    /// ids of messages that have already been seen, so only new ones trigger a notification
    pub known_message_ids: BTreeSet<i32>,
    /// (lesson identity, lead time in minutes) pairs an exam reminder was already sent for
    pub sent_exam_reminders: BTreeSet<(String, i64)>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub untis_auth: AuthSettings,
    pub b2e_auth: AuthSettings,
    pub visual_settings: VisualSettings,
    #[serde(default)]
    pub notification_settings: NotificationSettings,
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

fn default_true() -> bool { true }
fn default_poll_interval_minutes() -> u32 { 5 }
fn default_exam_lead_times() -> Vec<i64> { vec![1440, 60] }

/// Settings for the background notification poller, see `crate::notifications`. Fully user
/// customizable from the Settings screen.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct NotificationSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// how often the poller checks Untis for changes
    #[serde(default = "default_poll_interval_minutes")]
    pub poll_interval_minutes: u32,
    #[serde(default)]
    pub timetable_changes: TimetableChangeSettings,
    #[serde(default)]
    pub exam_reminders: ExamReminderSettings,
    #[serde(default)]
    pub message_notifications: MessageNotificationSettings,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_minutes: default_poll_interval_minutes(),
            timetable_changes: TimetableChangeSettings::default(),
            exam_reminders: ExamReminderSettings::default(),
            message_notifications: MessageNotificationSettings::default(),
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TimetableChangeSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub notify_cancelled: bool,
    #[serde(default = "default_true")]
    pub notify_room_change: bool,
    #[serde(default = "default_true")]
    pub notify_time_change: bool,
    #[serde(default = "default_true")]
    pub notify_substitution: bool,
}

impl Default for TimetableChangeSettings {
    fn default() -> Self {
        Self { enabled: true, notify_cancelled: true, notify_room_change: true, notify_time_change: true, notify_substitution: true }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ExamReminderSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// how long before an exam to notify, e.g. `[1440, 60]` for one day and one hour before
    #[serde(default = "default_exam_lead_times")]
    pub lead_times_minutes: Vec<i64>,
}

impl Default for ExamReminderSettings {
    fn default() -> Self {
        Self { enabled: true, lead_times_minutes: default_exam_lead_times() }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MessageNotificationSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for MessageNotificationSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// A short human label for a lead time in minutes, e.g. "1 day", "3 hours", "45 minutes"
pub fn describe_lead_time_minutes(minutes: i64) -> String {
    if minutes != 0 && minutes % 1440 == 0 {
        let days = minutes / 1440;
        format!("{days} day{}", if days == 1 { "" } else { "s" })
    } else if minutes != 0 && minutes % 60 == 0 {
        let hours = minutes / 60;
        format!("{hours} hour{}", if hours == 1 { "" } else { "s" })
    } else {
        format!("{minutes} minutes")
    }
}

/// Settings as they are shared with other devices, the logins are only included when asked for
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SettingsExport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub untis_auth: Option<AuthSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b2e_auth: Option<AuthSettings>,
    pub visual_settings: VisualSettings,
    #[serde(default)]
    pub notification_settings: NotificationSettings,
}

impl SettingsExport {
    pub fn new(settings: &Settings, include_credentials: bool) -> Self {
        Self {
            untis_auth: include_credentials.then(|| settings.untis_auth.clone()),
            b2e_auth: include_credentials.then(|| settings.b2e_auth.clone()),
            visual_settings: settings.visual_settings.clone(),
            notification_settings: settings.notification_settings.clone(),
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text.trim()).map_err(|_| "This isn't an Altis settings export".to_string())
    }

    pub fn has_credentials(&self) -> bool {
        self.untis_auth.is_some() || self.b2e_auth.is_some()
    }

    /// Overwrites the given settings with the imported ones, logins that weren't exported are kept
    pub fn apply_to(self, settings: &mut Settings) {
        if let Some(auth) = self.untis_auth {
            settings.untis_auth = auth;
        }
        if let Some(auth) = self.b2e_auth {
            settings.b2e_auth = auth;
        }
        settings.visual_settings = self.visual_settings;
        settings.notification_settings = self.notification_settings;
    }
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

    pub fn save_notification_state(state: &NotificationState) -> Result<(), String> {
        Self::save_compressed("notification_state", state)
    }

    pub fn get_notification_state() -> Result<Option<NotificationState>, String> {
        Self::get_compressed("notification_state")
    }

    fn save_compressed<T: Serialize>(key: &str, value: &T) -> Result<(), String> {
        let bytes = postcard::to_allocvec(value)
            .map_err(|e| format!("Postcard failed: {}", e))?;

        let compressed = zstd::encode_all(&bytes[..], 3)
            .map_err(|e| format!("Compression failed: {}", e))?;

        let encoded = STANDARD.encode(compressed);

        Self::get_storage()?
            .set_item(key, &encoded)
            .map_err(|_| "Failed to write to localStorage".to_string())?;

        Ok(())
    }

    fn get_compressed<T: DeserializeOwned>(key: &str) -> Result<Option<T>, String> {
        let storage = Self::get_storage()?;
        let value = storage.get_item(key)
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
