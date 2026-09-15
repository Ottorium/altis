//! Watches for timetable changes, upcoming exams and new messages, and shows a native
//! notification for each.
//!
//! One poll fetches the current and the next week's personal timetable once and uses it for both
//! the change diff and the exam reminders. Everything it needs from the platform (the store, HTTP
//! and the notification itself) comes through [`Env`], so the same poll runs in the frontend's
//! WASM on desktop and natively in the Android background service, where there is no webview to
//! run it in (see `altis::background` in the Tauri backend).

use crate::data_models::clean_models::untis::{ChangeStatus, Entity, LessonBlock, MyTimeTable};
use crate::env::Env;
use crate::settings::{
    describe_lead_time_minutes, ExamReminderSettings, NotificationState, TimetableChangeSettings,
};
use crate::store::Store;
use crate::untis::untis_client::UntisClient;
use crate::untis::untis_week::Week;
use chrono::{Local, NaiveDate, TimeDelta};
use std::collections::HashMap;

/// The weeks a poll looks at. Next week matters as much as this one: a lesson dropped on Friday
/// for the Monday after is exactly the change worth knowing about early.
pub fn tracked_weeks() -> [Week; 2] {
    let current = Week::current();
    let next = current.next();
    [current, next]
}

async fn notify<E: Env>(title: &str, body: &str) {
    E::default().notify(title, body).await;
}

/// Runs a single poll. Re-reads the settings every time, so changes made in the Settings screen
/// take effect on the next poll without anything having to be restarted.
pub async fn run_once<E: Env>() {
    let settings = Store::<E>::settings_or_default();
    let cfg = &settings.notification_settings;
    if !cfg.enabled || !settings.untis_auth.is_complete() {
        return;
    }

    let Ok(client) = UntisClient::<E>::new() else { return };
    let mut state = Store::<E>::get_notification_state().ok().flatten().unwrap_or_default();

    if cfg.timetable_changes.enabled || cfg.exam_reminders.enabled {
        check_timetable::<E>(&client, &mut state, &cfg.timetable_changes, &cfg.exam_reminders).await;
    }

    if cfg.message_notifications.enabled {
        check_messages::<E>(&client, &mut state).await;
    }

    let _ = Store::<E>::save_notification_state(&state);
}

/// Fetches the tracked weeks' personal timetable fresh (deliberately not through the frontend's
/// hour-long cache, this is what should be near-real-time), diffs each against what was seen last
/// poll, and scans the same data for exams that are now within a reminder's lead time.
async fn check_timetable<E: Env>(
    client: &UntisClient<E>,
    state: &mut NotificationState,
    changes: &TimetableChangeSettings,
    exams: &ExamReminderSettings,
) {
    let weeks = tracked_weeks();
    let mut fetched: Vec<MyTimeTable> = Vec::with_capacity(weeks.len());

    for week in &weeks {
        let Ok(fresh) = client.get_my_timetable(week.clone()).await else { continue };

        if changes.enabled {
            if let Some(previous) = state.last_my_timetable.get(&week.start) {
                diff_and_notify::<E>(previous, &fresh, changes).await;
            }
            state.last_my_timetable.insert(week.start.clone(), fresh.clone());
        }

        fetched.push(fresh);
    }

    // a week that has rolled out of the tracked range is never diffed against again
    state.last_my_timetable.retain(|start, _| weeks.iter().any(|week| &week.start == start));

    if exams.enabled {
        check_exam_reminders::<E>(&fetched, state, exams).await;
    }
}

async fn diff_and_notify<E: Env>(previous: &MyTimeTable, fresh: &MyTimeTable, cfg: &TimetableChangeSettings) {
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
                    notify_new_lesson::<E>(new_lesson, new_day.date).await;
                }
                continue;
            };

            notify_lesson_diff::<E>(old_lesson, new_lesson, new_day.date, cfg).await;
        }
    }
}

async fn notify_lesson_diff<E: Env>(old_lesson: &LessonBlock, new_lesson: &LessonBlock, date: NaiveDate, cfg: &TimetableChangeSettings) {
    let subject = subject_name(new_lesson);
    let day_str = date.format("%a %d %b").to_string();
    let time_str = format_time_range(new_lesson);

    if cfg.notify_cancelled && new_lesson.status == "CANCELLED" && old_lesson.status != "CANCELLED" {
        let body = format!("{subject} on {day_str} at {time_str} was cancelled");
        notify::<E>("Lesson cancelled", &body).await;
        return;
    }
    if cfg.notify_cancelled && old_lesson.status == "CANCELLED" && new_lesson.status != "CANCELLED" {
        let body = format!("{subject} on {day_str} at {time_str} is back on");
        notify::<E>("Lesson reinstated", &body).await;
        return;
    }
    if new_lesson.status == "CANCELLED" {
        return;
    }

    if cfg.notify_time_change && old_lesson.time_range != new_lesson.time_range {
        let old_time = format_time_range(old_lesson);
        let body = format!("{subject} on {day_str} is now at {time_str} (was {old_time})");
        notify::<E>("Time changed", &body).await;
    }

    if cfg.notify_room_change {
        let (old_rooms, new_rooms) = (room_names(old_lesson), room_names(new_lesson));
        if old_rooms != new_rooms {
            let body = format!(
                "{subject} on {day_str} at {time_str} is now in {} (was {})",
                format_names(&new_rooms),
                format_names(&old_rooms),
            );
            notify::<E>("Room changed", &body).await;
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
            notify::<E>("Teacher changed", &body).await;
        } else if old_lesson.status != new_lesson.status && new_lesson.status == "CHANGED" {
            let body = format!("{subject} on {day_str} at {time_str} was changed");
            notify::<E>("Lesson changed", &body).await;
        }
    }
}

async fn notify_new_lesson<E: Env>(lesson: &LessonBlock, date: NaiveDate) {
    let body = format!("{} on {} at {}", subject_name(lesson), date.format("%a %d %b"), format_time_range(lesson));
    notify::<E>("New lesson added", &body).await;
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

/// Scans the already fetched timetables for exams and notifies once per configured lead time
async fn check_exam_reminders<E: Env>(timetables: &[MyTimeTable], state: &mut NotificationState, cfg: &ExamReminderSettings) {
    if cfg.lead_times_minutes.is_empty() {
        return;
    }
    let now = Local::now().naive_local();

    for timetable in timetables {
        for day in &timetable.timetable.days {
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
                        notify::<E>(&format!("Exam in {}", describe_lead_time_minutes(lead)), &body).await;
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
async fn check_messages<E: Env>(client: &UntisClient<E>, state: &mut NotificationState) {
    let Ok(messages) = client.get_messages().await else { return };
    let first_run = state.known_message_ids.is_empty();

    for message in &messages {
        if state.known_message_ids.insert(message.id) && !first_run {
            let sender = if message.sender.display_name.is_empty() { "Untis".to_string() } else { message.sender.display_name.clone() };
            notify::<E>(&format!("New message from {sender}"), &message.subject).await;
        }
    }

    // keep the known-id set bounded rather than growing forever
    if state.known_message_ids.len() > 500 {
        let current_ids: std::collections::BTreeSet<i32> = messages.iter().map(|m| m.id).collect();
        state.known_message_ids.retain(|id| current_ids.contains(id));
    }
}
