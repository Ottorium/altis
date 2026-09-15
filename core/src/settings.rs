use crate::data_models::clean_models::untis::{ChangeStatus, Entity, LessonBlock, MyTimeTable};
use chrono::Weekday;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Bookkeeping for the background notification poller (see `crate::notifications`), so it knows
/// what has changed since the last check and doesn't repeat itself
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct NotificationState {
    /// the personal timetable as of the last poll, keyed by the week's Monday (`Week::start`),
    /// diffed against on the next poll
    #[serde(default)]
    pub last_my_timetable: BTreeMap<String, MyTimeTable>,
    /// ids of messages that have already been seen, so only new ones trigger a notification
    #[serde(default)]
    pub known_message_ids: BTreeSet<i32>,
    /// (lesson identity, lead time in minutes) pairs an exam reminder was already sent for
    #[serde(default)]
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

impl AuthSettings {
    pub fn is_complete(&self) -> bool {
        !self.school_identifier.is_empty() && !self.user_identifier.is_empty() && !self.secret.is_empty()
    }
}

/// Roughly how often Android's WorkManager runs a periodic job. 15 minutes is its hard floor,
/// and it is a floor rather than a promise: the system batches jobs and Doze can stretch the gap
/// considerably while the phone is idle.
pub const PERIODIC_INTERVAL_MINUTES: u32 = 15;

/// How Android keeps checking once the app is closed. Ignored everywhere else, where the poll
/// runs in the webview and the only requirement is that the app is still running.
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackgroundMode {
    /// Doesn't. Notifications happen only while the app is open.
    Off,
    /// A WorkManager job, no notification, but no better than every
    /// [`PERIODIC_INTERVAL_MINUTES`] and often less while the phone is idle.
    #[default]
    Periodic,
    /// A foreground service on the configured interval. Exact, and Android charges a permanent
    /// notification for it.
    Continuous,
}

impl BackgroundMode {
    /// The wire form shared with the Android side
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Periodic => "periodic",
            Self::Continuous => "continuous",
        }
    }

    /// Whether the poll happens outside the webview, i.e. whether the frontend should stand down
    pub fn is_background(self) -> bool {
        self != Self::Off
    }
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
    /// on Android, how to keep polling after the app is closed
    #[serde(default)]
    pub background_mode: BackgroundMode,
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
            background_mode: BackgroundMode::default(),
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
