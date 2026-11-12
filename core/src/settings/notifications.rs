use super::default_true;
use serde::{Deserialize, Serialize};

/// Roughly how often Android's WorkManager runs a periodic job. 15 minutes is its hard floor,
/// and it is a floor rather than a promise: the system batches jobs and Doze can stretch the gap
/// considerably while the phone is idle.
pub const PERIODIC_INTERVAL_MINUTES: u32 = 15;

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

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            poll_interval_minutes: default_poll_interval_minutes(),
            background_mode: BackgroundMode::default(),
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
pub struct TimetableChangeSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// lessons that were cancelled, reinstated or dropped from the timetable
    #[serde(default = "default_true")]
    pub notify_cancelled: bool,
    #[serde(default = "default_true")]
    pub notify_room_change: bool,
    #[serde(default = "default_true")]
    pub notify_time_change: bool,
    /// another teacher, a lesson marked as changed, or one that was added
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
/// are not, so only the former are notified about (see `crate::notifications`)
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AbsenceNotificationSettings {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for AbsenceNotificationSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// A short human label for a lead time in minutes, e.g. "1 day", "3 hours", "45 minutes"
pub fn describe_lead_time_minutes(minutes: i64) -> String {
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
    fn describes_a_lead_time_in_its_largest_whole_unit() {
        assert_eq!(describe_lead_time_minutes(1440), "1 day");
        assert_eq!(describe_lead_time_minutes(2880), "2 days");
        assert_eq!(describe_lead_time_minutes(60), "1 hour");
        assert_eq!(describe_lead_time_minutes(180), "3 hours");
        assert_eq!(describe_lead_time_minutes(45), "45 minutes");
        assert_eq!(describe_lead_time_minutes(90), "90 minutes");
        assert_eq!(describe_lead_time_minutes(0), "0 minutes");
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
}
