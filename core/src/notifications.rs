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

/// Writes the bookkeeping out mid-poll.
///
/// Whatever stops a notification from being sent a second time has to be on disk *before* it is
/// sent, never after: a poll that dies in between - a background job Android kills for taking too
/// long, a request that never comes back, a process that goes away - would otherwise leave the old
/// state behind and send the very same notifications again on every later poll, for as long as the
/// change is still there.
fn persist<E: Env>(state: &NotificationState) {
    let _ = Store::<E>::save_notification_state(state);
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
            // the fresh week becomes the baseline and is written out first, and only then is
            // anything sent: see `persist`
            let previous = state.last_my_timetable.insert(week.start.clone(), fresh.clone());
            persist::<E>(state);

            if let Some(previous) = previous {
                diff_and_notify::<E>(&previous, &fresh, changes).await;
            }
        }

        fetched.push(fresh);
    }

    // a week that has rolled out of the tracked range is never diffed against again
    state.last_my_timetable.retain(|start, _| weeks.iter().any(|week| &week.start == start));

    if exams.enabled {
        check_exam_reminders::<E>(&fetched, state, exams).await;
    }
}

/// Pairs a day's lessons up across two polls, returning the pairs along with what was only in the
/// fresh day (added) and what was only in the previous one (removed).
///
/// Two passes, because a single map keyed on anything less than the whole lesson collapses the
/// duplicates an ordinary school day is full of - the same subject with the same teacher twice in
/// a day, or a double period split into two blocks. Collapsed, the morning block ends up compared
/// against the afternoon one, and every poll reports the same time and room change that never
/// happened, for as long as both lessons stand.
///
/// The first pass takes the lessons that are identical anyway and consumes both sides, so only
/// what actually differs is left to guess about. The second pairs the rest off by subject in
/// start-time order. Teacher is deliberately not part of the second key: a substitution changes
/// the teacher, which is the very thing worth reporting, so matching on it would stop the two
/// halves of one lesson ever meeting.
fn pair_lessons<'a>(
    previous: &'a [LessonBlock],
    fresh: &'a [LessonBlock],
) -> (Vec<(&'a LessonBlock, &'a LessonBlock)>, Vec<&'a LessonBlock>, Vec<&'a LessonBlock>) {
    let mut old_left: Vec<&LessonBlock> = previous.iter().collect();
    let mut new_left: Vec<&LessonBlock> = fresh.iter().collect();
    old_left.sort_by_key(|lesson| lesson.time_range.start);
    new_left.sort_by_key(|lesson| lesson.time_range.start);

    let mut pairs = Vec::new();
    let mut changed = Vec::new();

    for new_lesson in new_left {
        let unchanged = old_left.iter().position(|old| {
            old.time_range == new_lesson.time_range
                && subject_name(old) == subject_name(new_lesson)
                && teacher_names(old) == teacher_names(new_lesson)
        });

        match unchanged {
            Some(index) => pairs.push((old_left.remove(index), new_lesson)),
            None => changed.push(new_lesson),
        }
    }

    let mut added = Vec::new();
    for new_lesson in changed {
        match old_left.iter().position(|old| subject_name(old) == subject_name(new_lesson)) {
            Some(index) => pairs.push((old_left.remove(index), new_lesson)),
            None => added.push(new_lesson),
        }
    }

    (pairs, added, old_left)
}

async fn diff_and_notify<E: Env>(previous: &MyTimeTable, fresh: &MyTimeTable, cfg: &TimetableChangeSettings) {
    for (prev_day, new_day) in previous.timetable.days.iter().zip(fresh.timetable.days.iter()) {
        if prev_day.date != new_day.date {
            continue;
        }

        let (pairs, added, removed) = pair_lessons(&prev_day.lessons, &new_day.lessons);

        for (old_lesson, new_lesson) in pairs {
            notify_lesson_diff::<E>(old_lesson, new_lesson, new_day.date, cfg).await;
        }

        if cfg.notify_substitution {
            for new_lesson in added.iter().filter(|lesson| lesson.status != "CANCELLED") {
                notify_new_lesson::<E>(new_lesson, new_day.date).await;
            }
        }

        if cfg.notify_cancelled {
            // one that was already cancelled was reported as such when it happened, and dropping
            // out of the timetable afterwards is not a second thing to hear about
            for old_lesson in removed.iter().filter(|lesson| lesson.status != "CANCELLED") {
                notify_removed_lesson::<E>(old_lesson, new_day.date).await;
            }
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

async fn notify_removed_lesson<E: Env>(lesson: &LessonBlock, date: NaiveDate) {
    let body = format!("{} on {} at {}", subject_name(lesson), date.format("%a %d %b"), format_time_range(lesson));
    notify::<E>("Lesson removed", &body).await;
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
    let mut due = Vec::new();

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
                    if now >= trigger_at && now < lesson.time_range.start && state.sent_exam_reminders.insert(key) {
                        let body = format!(
                            "{} exam at {} on {}",
                            subject_name(lesson),
                            lesson.time_range.start.format("%H:%M"),
                            day.date.format("%a %d %b"),
                        );
                        due.push((format!("Exam in {}", describe_lead_time_minutes(lead)), body));
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

    persist::<E>(state);
    for (title, body) in &due {
        notify::<E>(title, body).await;
    }
}

/// Notifies about any inbox message not seen in a previous poll. On the very first run every
/// existing message is just recorded as seen, so opening the app for the first time doesn't fire
/// a notification for the entire inbox history.
async fn check_messages<E: Env>(client: &UntisClient<E>, state: &mut NotificationState) {
    let Ok(messages) = client.get_messages().await else { return };
    let first_run = state.known_message_ids.is_empty();

    let mut fresh = Vec::new();
    for message in &messages {
        if state.known_message_ids.insert(message.id) && !first_run {
            let sender = if message.sender.display_name.is_empty() { "Untis".to_string() } else { message.sender.display_name.clone() };
            fresh.push((format!("New message from {sender}"), message.subject.clone()));
        }
    }

    // keep the known-id set bounded rather than growing forever
    if state.known_message_ids.len() > 500 {
        let current_ids: std::collections::BTreeSet<i32> = messages.iter().map(|m| m.id).collect();
        state.known_message_ids.retain(|id| current_ids.contains(id));
    }

    persist::<E>(state);
    for (title, body) in &fresh {
        notify::<E>(title, body).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_models::clean_models::untis::{Room, Subject, Teacher, TimeRange, Tracked};
    use chrono::NaiveDate;

    fn tracked(inner: Entity) -> Tracked<Entity> {
        Tracked { inner, status: ChangeStatus::Regular }
    }

    fn lesson(subject: &str, teacher: &str, room: &str, start: (u32, u32), end: (u32, u32)) -> LessonBlock {
        let date = NaiveDate::from_ymd_opt(2026, 9, 14).unwrap();
        LessonBlock {
            time_range: TimeRange {
                start: date.and_hms_opt(start.0, start.1, 0).unwrap(),
                end: date.and_hms_opt(end.0, end.1, 0).unwrap(),
            },
            entities: vec![
                tracked(Entity::Subject(Subject { short_name: subject.to_string(), ..Default::default() })),
                tracked(Entity::Teacher(Teacher { short_name: teacher.to_string(), ..Default::default() })),
                tracked(Entity::Room(Room { name: room.to_string() })),
            ],
            ..Default::default()
        }
    }

    /// The bug that made the poller repeat itself every 15 minutes: two lessons sharing a subject
    /// and a teacher on one day collapsed into a single entry, so the morning block was compared
    /// against the afternoon one and every poll reported the same time and room change, forever.
    #[test]
    fn pairs_duplicate_subjects_with_themselves() {
        let previous = [
            lesson("Math", "SMI", "R1", (8, 0), (8, 45)),
            lesson("Math", "SMI", "R2", (14, 0), (14, 45)),
        ];
        let fresh = previous.clone();

        let (pairs, added, removed) = pair_lessons(&previous, &fresh);

        assert_eq!(pairs.len(), 2);
        assert!(added.is_empty() && removed.is_empty());
        for (old, new) in pairs {
            assert_eq!(old.time_range, new.time_range);
            assert_eq!(room_names(old), room_names(new));
        }
    }

    /// A substitution changes the teacher, so keying on the teacher stopped the two halves of the
    /// lesson ever meeting and the change went unreported
    #[test]
    fn pairs_a_substituted_lesson() {
        let previous = [lesson("Math", "SMI", "R1", (8, 0), (8, 45))];
        let fresh = [lesson("Math", "JON", "R1", (8, 0), (8, 45))];

        let (pairs, added, removed) = pair_lessons(&previous, &fresh);

        assert_eq!(pairs.len(), 1);
        assert!(added.is_empty() && removed.is_empty());
        let (old, new) = pairs[0];
        assert_eq!(teacher_names(old), vec!["SMI".to_string()]);
        assert_eq!(teacher_names(new), vec!["JON".to_string()]);
    }

    #[test]
    fn pairs_a_moved_lesson_with_its_old_self() {
        let previous = [lesson("Math", "SMI", "R1", (8, 0), (8, 45))];
        let fresh = [lesson("Math", "SMI", "R2", (10, 0), (10, 45))];

        let (pairs, added, removed) = pair_lessons(&previous, &fresh);

        assert_eq!(pairs.len(), 1);
        assert!(added.is_empty() && removed.is_empty());
    }

    #[test]
    fn reports_lessons_that_appeared_and_disappeared() {
        let previous = [lesson("Math", "SMI", "R1", (8, 0), (8, 45))];
        let fresh = [lesson("Bio", "GRA", "R3", (8, 0), (8, 45))];

        let (pairs, added, removed) = pair_lessons(&previous, &fresh);

        assert!(pairs.is_empty());
        assert_eq!(added.len(), 1);
        assert_eq!(subject_name(added[0]), "Bio");
        assert_eq!(removed.len(), 1);
        assert_eq!(subject_name(removed[0]), "Math");
    }
}
