//! Background poller that watches for timetable changes, upcoming exams and new messages, and
//! shows a native notification for each. This all lives in the frontend because that's where the
//! Untis client and its cached login live; it only runs for as long as the app's webview is alive
//! (see `setup_tray` in the Tauri backend for how the desktop app is kept running in the
//! background instead of quitting when the window is closed).

use crate::data_models::clean_models::untis::{ChangeStatus, Entity, LessonBlock, MyTimeTable};
use crate::native;
use crate::persistence_manager::{
    describe_lead_time_minutes, ExamReminderSettings, NotificationState, PersistenceManager, TimetableChangeSettings,
};
use crate::untis::cached_untis_client::CachedUntisClient;
use crate::untis::untis_client::UntisClient;
use crate::untis::untis_week::Week;
use chrono::{Local, NaiveDate, TimeDelta};
use gloo_timers::future::TimeoutFuture;
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;

/// Starts the background poller. Re-reads the settings on every cycle, so changes made in the
/// Settings screen (including the poll interval itself) take effect on the next poll.
pub fn start() {
    spawn_local(async {
        // ask for the permission up front, so the first real notification isn't silently dropped
        let _ = native::ensure_notification_permission().await;

        loop {
            run_once().await;

            let settings = PersistenceManager::get_settings().ok().flatten().unwrap_or_default();
            let minutes = settings.notification_settings.poll_interval_minutes.max(1);
            TimeoutFuture::new(minutes * 60 * 1000).await;
        }
    });
}

async fn run_once() {
    let Ok(Some(settings)) = PersistenceManager::get_settings() else { return };
    let cfg = &settings.notification_settings;
    if !cfg.enabled || settings.untis_auth.school_identifier.is_empty() {
        return;
    }

    let mut state = PersistenceManager::get_notification_state().ok().flatten().unwrap_or_default();

    if cfg.timetable_changes.enabled
        && let Ok(client) = UntisClient::new() {
        check_timetable_changes(&client, &cfg.timetable_changes, &mut state).await;
    }

    if cfg.exam_reminders.enabled {
        check_exam_reminders(&mut state, &cfg.exam_reminders).await;
    }

    if cfg.message_notifications.enabled
        && let Ok(client) = UntisClient::new() {
        check_messages(&client, &mut state).await;
    }

    let _ = PersistenceManager::save_notification_state(&state);
}

/// Fetches the current and next week's personal timetable fresh (bypassing the hour-long cache,
/// since this is what should be near-real-time) and diffs it against what was seen last poll
async fn check_timetable_changes(client: &UntisClient, cfg: &TimetableChangeSettings, state: &mut NotificationState) {
    let tracked = [Week::current(), Week::current().next()];

    for week in tracked.clone() {
        let Ok(fresh) = client.get_my_timetable(week.clone()).await else { continue };
        if let Some(previous) = state.last_my_timetable.get(&week) {
            diff_and_notify(previous, &fresh, cfg).await;
        }
        state.last_my_timetable.insert(week, fresh);
    }

    state.last_my_timetable.retain(|w, _| tracked.contains(w));
}

async fn diff_and_notify(previous: &MyTimeTable, fresh: &MyTimeTable, cfg: &TimetableChangeSettings) {
    for (prev_day, new_day) in previous.timetable.days.iter().zip(fresh.timetable.days.iter()) {
        if prev_day.date != new_day.date {
            continue;
        }

        let mut prev_by_identity: HashMap<(String, Vec<String>), &LessonBlock> = HashMap::new();
        for lesson in &prev_day.lessons {
            prev_by_identity.insert(lesson_identity(lesson), lesson);
        }

        for new_lesson in &new_day.lessons {
            let Some(old_lesson) = prev_by_identity.remove(&lesson_identity(new_lesson)) else {
                if cfg.notify_substitution && new_lesson.status == "ADDITIONAL" {
                    notify_new_lesson(new_lesson, new_day.date).await;
                }
                continue;
            };

            notify_lesson_diff(old_lesson, new_lesson, new_day.date, cfg).await;
        }
    }
}

async fn notify_lesson_diff(old_lesson: &LessonBlock, new_lesson: &LessonBlock, date: NaiveDate, cfg: &TimetableChangeSettings) {
    let subject = subject_name(new_lesson);
    let day_str = date.format("%a %d %b").to_string();
    let time_str = format_time_range(new_lesson);

    if cfg.notify_cancelled && new_lesson.status == "CANCELLED" && old_lesson.status != "CANCELLED" {
        let body = format!("{subject} on {day_str} at {time_str} was cancelled");
        let _ = native::send_notification("Lesson cancelled", &body).await;
        return;
    }
    if cfg.notify_cancelled && old_lesson.status == "CANCELLED" && new_lesson.status != "CANCELLED" {
        let body = format!("{subject} on {day_str} at {time_str} is back on");
        let _ = native::send_notification("Lesson reinstated", &body).await;
        return;
    }
    if new_lesson.status == "CANCELLED" {
        return;
    }

    if cfg.notify_time_change && old_lesson.time_range != new_lesson.time_range {
        let old_time = format_time_range(old_lesson);
        let body = format!("{subject} on {day_str} is now at {time_str} (was {old_time})");
        let _ = native::send_notification("Time changed", &body).await;
    }

    if cfg.notify_room_change {
        let (old_rooms, new_rooms) = (room_names(old_lesson), room_names(new_lesson));
        if old_rooms != new_rooms {
            let body = format!(
                "{subject} on {day_str} at {time_str} is now in {} (was {})",
                format_names(&new_rooms),
                format_names(&old_rooms),
            );
            let _ = native::send_notification("Room changed", &body).await;
        }
    }

    if cfg.notify_substitution {
        let (old_teachers, new_teachers) = (teacher_names(old_lesson), teacher_names(new_lesson));
        if old_teachers != new_teachers {
            let body = format!(
                "{subject} on {day_str} at {time_str} is now with {} (was {})",
                format_names(&new_teachers),
                format_names(&old_teachers),
            );
            let _ = native::send_notification("Teacher changed", &body).await;
        } else if old_lesson.status != new_lesson.status && new_lesson.status == "CHANGED" {
            let body = format!("{subject} on {day_str} at {time_str} was changed");
            let _ = native::send_notification("Lesson changed", &body).await;
        }
    }
}

async fn notify_new_lesson(lesson: &LessonBlock, date: NaiveDate) {
    let body = format!("{} on {} at {}", subject_name(lesson), date.format("%a %d %b"), format_time_range(lesson));
    let _ = native::send_notification("New lesson added", &body).await;
}

/// Identity used to match the same lesson across polls even if its time or room changed: the
/// subject plus the teacher(s) are the parts of a lesson that stay stable across those changes.
/// Two distinct lessons with the same subject and teacher on the same day are treated as one,
/// which is a rare enough case to accept for a best-effort notifier.
fn lesson_identity(lesson: &LessonBlock) -> (String, Vec<String>) {
    (subject_name(lesson), teacher_names(lesson))
}

fn subject_name(lesson: &LessonBlock) -> String {
    entity_names(lesson, |e| matches!(e, Entity::Subject(_))).into_iter().next().unwrap_or_else(|| "Lesson".to_string())
}

fn teacher_names(lesson: &LessonBlock) -> Vec<String> {
    entity_names(lesson, |e| matches!(e, Entity::Teacher(_)))
}

fn room_names(lesson: &LessonBlock) -> Vec<String> {
    entity_names(lesson, |e| matches!(e, Entity::Room(_)))
}

fn entity_names(lesson: &LessonBlock, filter: fn(&Entity) -> bool) -> Vec<String> {
    let mut names: Vec<String> = lesson.entities.iter()
        .filter(|t| t.status != ChangeStatus::Removed)
        .filter(|t| filter(&t.inner))
        .map(|t| t.inner.name())
        .collect();
    names.sort();
    names
}

fn format_names(names: &[String]) -> String {
    if names.is_empty() { "none".to_string() } else { names.join(", ") }
}

fn format_time_range(lesson: &LessonBlock) -> String {
    format!("{}-{}", lesson.time_range.start.format("%H:%M"), lesson.time_range.end.format("%H:%M"))
}

/// Scans the current and next week's personal timetable (via the hourly cache, exam reminders
/// don't need minute-level freshness) for exams, and notifies once per configured lead time
async fn check_exam_reminders(state: &mut NotificationState, cfg: &ExamReminderSettings) {
    if cfg.lead_times_minutes.is_empty() {
        return;
    }
    let Ok(client) = CachedUntisClient::new() else { return };
    let now = Local::now().naive_local();

    for week in [Week::current(), Week::current().next()] {
        let Ok((_, tt)) = client.get_my_timetable(week).await else { continue };
        for day in &tt.timetable.days {
            for lesson in &day.lessons {
                if lesson.r#type != "EXAM" || lesson.status == "CANCELLED" {
                    continue;
                }

                let identity = format!("{}|{}|{}", day.date, lesson.time_range.start.format("%H:%M"), subject_name(lesson));
                for &lead in &cfg.lead_times_minutes {
                    let trigger_at = lesson.time_range.start - TimeDelta::minutes(lead);
                    let key = (identity.clone(), lead);
                    if now >= trigger_at && now < lesson.time_range.start && !state.sent_exam_reminders.contains(&key) {
                        let body = format!(
                            "{} exam at {} on {}",
                            subject_name(lesson),
                            lesson.time_range.start.format("%H:%M"),
                            day.date.format("%a %d %b"),
                        );
                        let _ = native::send_notification(&format!("Exam in {}", describe_lead_time_minutes(lead)), &body).await;
                        state.sent_exam_reminders.insert(key);
                    }
                }
            }
        }
    }

    // drop dedupe entries for exams that are in the past, so the set doesn't grow forever
    let today = now.date();
    state.sent_exam_reminders.retain(|(identity, _)| {
        identity.split('|').next()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .is_none_or(|d| d >= today)
    });
}

/// Notifies about any inbox message not seen in a previous poll. On the very first run every
/// existing message is just recorded as seen, so opening the app for the first time doesn't fire
/// a notification for the entire inbox history.
async fn check_messages(client: &UntisClient, state: &mut NotificationState) {
    let Ok(messages) = client.get_messages().await else { return };
    let first_run = state.known_message_ids.is_empty();

    for message in &messages {
        if state.known_message_ids.insert(message.id) && !first_run {
            let sender = if message.sender.display_name.is_empty() { "Untis".to_string() } else { message.sender.display_name.clone() };
            let _ = native::send_notification(&format!("New message from {sender}"), &message.subject).await;
        }
    }

    // keep the known-id set bounded rather than growing forever
    if state.known_message_ids.len() > 500 {
        let current_ids: std::collections::BTreeSet<i32> = messages.iter().map(|m| m.id).collect();
        state.known_message_ids.retain(|id| current_ids.contains(id));
    }
}
