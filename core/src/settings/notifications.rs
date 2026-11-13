use super::default_true;
use serde::{Deserialize, Serialize};

/// Roughly how often Android's WorkManager runs a periodic job. 15 minutes is its hard floor,
/// and it is a floor rather than a promise: the system batches jobs and Doze can stretch the gap
/// considerably while the phone is idle.
pub const PERIODIC_INTERVAL_MINUTES: u32 = 15;

/// How many weeks beyond the running one the poller watches at most. Every week costs two
/// requests a poll, and a timetable that far out is rarely final anyway.
pub const MAX_WEEKS_AHEAD: u32 = 4;

const MINUTES_PER_HOUR: i64 = 60;
const MINUTES_PER_DAY: i64 = 24 * MINUTES_PER_HOUR;

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
    /// how many weeks after the running one are watched for changes and exams
    #[serde(default = "default_weeks_ahead")]
    pub weeks_ahead: u32,
    #[serde(default)]
    pub timetable_changes: TimetableChangeSettings,
    #[serde(default)]
    pub exam_reminders: ExamReminderSettings,
    #[serde(default)]
    pub message_notifications: MessageNotificationSettings,
    #[serde(default)]
    pub absence_notifications: AbsenceNotificationSettings,
}

fn default_poll_interval_minutes() -> u32 {
    5
}

/// Next week matters as much as this one: a lesson dropped on Friday for the Monday after is
/// exactly the change worth knowing about early
fn default_weeks_ahead() -> u32 {
    1
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_minutes: default_poll_interval_minutes(),
            background_mode: BackgroundMode::default(),
            weeks_ahead: default_weeks_ahead(),
            timetable_changes: TimetableChangeSettings::default(),
            exam_reminders: ExamReminderSettings::default(),
            message_notifications: MessageNotificationSettings::default(),
            absence_notifications: AbsenceNotificationSettings::default(),
        }
    }
}

impl NotificationSettings {
    /// The mode Android should actually be in: turning notifications off altogether has to stop
    /// the background polling too, whatever mode is selected under it
    pub fn effective_background_mode(&self) -> BackgroundMode {
        match self.enabled {
            true => self.background_mode,
            false => BackgroundMode::Off,
        }
    }

    /// How many weeks after the running one are watched, whatever was stored
    pub fn weeks_ahead(&self) -> u32 {
        self.weeks_ahead.min(MAX_WEEKS_AHEAD)
    }

    /// The pause between two polls. Never less than a minute, whatever was stored.
    pub fn poll_minutes(&self) -> u32 {
        self.poll_interval_minutes.max(1)
    }
}

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
    pub const ALL: [BackgroundMode; 3] = [BackgroundMode::Off, BackgroundMode::Periodic, BackgroundMode::Continuous];

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

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(from = "StoredTimetableChangeSettings")]
pub struct TimetableChangeSettings {
    pub enabled: bool,
    /// lessons that were cancelled or reinstated
    pub notify_cancelled: bool,
    pub notify_room_change: bool,
    pub notify_time_change: bool,
    /// another teacher, or a lesson Untis marks as changed
    pub notify_substitution: bool,
    /// lessons that weren't in the timetable before
    pub notify_added: bool,
    /// lessons that are no longer in the timetable
    pub notify_removed: bool,
}

impl Default for TimetableChangeSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            notify_cancelled: true,
            notify_room_change: true,
            notify_time_change: true,
            notify_substitution: true,
            notify_added: true,
            notify_removed: true,
        }
    }
}

/// The settings as they are stored. Added and removed lessons used to be reported along with
/// substitutions and cancellations, so settings from before they had a switch of their own take
/// over what those were set to - nobody starts getting notifications they had switched off.
#[derive(Deserialize)]
struct StoredTimetableChangeSettings {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default = "default_true")]
    notify_cancelled: bool,
    #[serde(default = "default_true")]
    notify_room_change: bool,
    #[serde(default = "default_true")]
    notify_time_change: bool,
    #[serde(default = "default_true")]
    notify_substitution: bool,
    notify_added: Option<bool>,
    notify_removed: Option<bool>,
}

impl From<StoredTimetableChangeSettings> for TimetableChangeSettings {
    fn from(stored: StoredTimetableChangeSettings) -> Self {
        Self {
            enabled: stored.enabled,
            notify_cancelled: stored.notify_cancelled,
            notify_room_change: stored.notify_room_change,
            notify_time_change: stored.notify_time_change,
            notify_substitution: stored.notify_substitution,
            notify_added: stored.notify_added.unwrap_or(stored.notify_substitution),
            notify_removed: stored.notify_removed.unwrap_or(stored.notify_cancelled),
        }
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

fn default_exam_lead_times() -> Vec<i64> {
    vec![MINUTES_PER_DAY, MINUTES_PER_HOUR]
}

impl Default for ExamReminderSettings {
    fn default() -> Self {
        Self { enabled: true, lead_times_minutes: default_exam_lead_times() }
    }
}

impl ExamReminderSettings {
    pub fn has_lead_time(&self, minutes: i64) -> bool {
        self.lead_times_minutes.contains(&minutes)
    }

    /// Adds the lead time, keeping the list in order. Does nothing if it is in there already.
    pub fn add_lead_time(&mut self, minutes: i64) {
        if !self.has_lead_time(minutes) {
            self.lead_times_minutes.push(minutes);
            self.lead_times_minutes.sort();
        }
    }

    pub fn remove_lead_time(&mut self, minutes: i64) {
        self.lead_times_minutes.retain(|lead| *lead != minutes);
    }

    pub fn toggle_lead_time(&mut self, minutes: i64) {
        match self.has_lead_time(minutes) {
            true => self.remove_lead_time(minutes),
            false => self.add_lead_time(minutes),
        }
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

/// Absences a teacher enters for the student are the ones worth hearing about, the student's own
/// are not - they typed them in a minute ago - so those stay quiet unless asked for
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AbsenceNotificationSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// notify about the absences the student reported themselves as well
    #[serde(default)]
    pub include_own: bool,
}

impl Default for AbsenceNotificationSettings {
    fn default() -> Self {
        Self { enabled: true, include_own: false }
    }
}

/// A stretch of time in words, in its largest whole unit: "1 day", "3 hours", "45 minutes"
pub fn describe_minutes(minutes: i64) -> String {
    let plural = |count: i64| if count == 1 { "" } else { "s" };

    if minutes != 0 && minutes % MINUTES_PER_DAY == 0 {
        let days = minutes / MINUTES_PER_DAY;
        format!("{days} day{}", plural(days))
    } else if minutes != 0 && minutes % MINUTES_PER_HOUR == 0 {
        let hours = minutes / MINUTES_PER_HOUR;
        format!("{hours} hour{}", plural(hours))
    } else {
        format!("{minutes} minutes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describes_minutes_in_their_largest_whole_unit() {
        assert_eq!(describe_minutes(1440), "1 day");
        assert_eq!(describe_minutes(2880), "2 days");
        assert_eq!(describe_minutes(60), "1 hour");
        assert_eq!(describe_minutes(180), "3 hours");
        assert_eq!(describe_minutes(45), "45 minutes");
        assert_eq!(describe_minutes(90), "90 minutes");
        assert_eq!(describe_minutes(0), "0 minutes");
    }

    #[test]
    fn switching_notifications_off_switches_the_background_polling_off() {
        let on = NotificationSettings { background_mode: BackgroundMode::Continuous, ..Default::default() };
        let off = NotificationSettings { enabled: false, ..on.clone() };

        assert_eq!(on.effective_background_mode(), BackgroundMode::Continuous);
        assert_eq!(off.effective_background_mode(), BackgroundMode::Off);
    }

    #[test]
    fn lead_times_stay_sorted_and_unique() {
        let mut exams = ExamReminderSettings::default();

        exams.toggle_lead_time(15);
        exams.add_lead_time(60);
        assert_eq!(exams.lead_times_minutes, vec![15, 60, 1440]);

        exams.toggle_lead_time(60);
        assert_eq!(exams.lead_times_minutes, vec![15, 1440]);
    }

    /// settings stored before a field existed have to keep loading, with that field at its default
    #[test]
    fn reads_settings_that_predate_most_of_their_fields() {
        let settings: NotificationSettings = serde_json::from_str(r#"{"poll_interval_minutes": 30}"#).unwrap();

        assert_eq!(settings, NotificationSettings { poll_interval_minutes: 30, ..Default::default() });
        assert_eq!(serde_json::to_value(BackgroundMode::Continuous).unwrap(), "continuous");
    }

    /// before added and removed lessons had a switch of their own they went with substitutions
    /// and cancellations, and stored settings have to keep meaning what they meant
    #[test]
    fn settings_without_the_newer_switches_take_them_from_the_ones_they_were_part_of() {
        let stored = r#"{"notify_cancelled": false, "notify_substitution": true}"#;
        let settings: TimetableChangeSettings = serde_json::from_str(stored).unwrap();
        assert!(settings.notify_added && !settings.notify_removed);

        let stored = r#"{"notify_cancelled": true, "notify_substitution": false}"#;
        let settings: TimetableChangeSettings = serde_json::from_str(stored).unwrap();
        assert!(!settings.notify_added && settings.notify_removed);

        assert_eq!(serde_json::from_str::<TimetableChangeSettings>("{}").unwrap(), TimetableChangeSettings::default());
    }

    #[test]
    fn the_newer_switches_are_their_own_once_they_were_stored() {
        let settings = TimetableChangeSettings { notify_substitution: false, notify_added: true, ..Default::default() };

        let stored = serde_json::to_string(&settings).unwrap();
        assert_eq!(serde_json::from_str::<TimetableChangeSettings>(&stored).unwrap(), settings);
    }

    #[test]
    fn watches_the_next_week_unless_told_otherwise_and_never_more_than_a_few() {
        assert_eq!(NotificationSettings::default().weeks_ahead(), 1);
        assert_eq!(NotificationSettings { weeks_ahead: 0, ..Default::default() }.weeks_ahead(), 0);
        assert_eq!(NotificationSettings { weeks_ahead: 99, ..Default::default() }.weeks_ahead(), MAX_WEEKS_AHEAD);
    }

    #[test]
    fn the_students_own_absences_stay_quiet_by_default() {
        let settings: AbsenceNotificationSettings = serde_json::from_str(r#"{"enabled": true}"#).unwrap();
        assert!(!settings.include_own);
    }
}
