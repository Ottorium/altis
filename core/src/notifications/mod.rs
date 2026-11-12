//! Watches for timetable changes, upcoming exams, new messages and new absences, and shows a
//! native notification for each.
//!
//! One poll fetches the current and the next week's personal timetable once and uses it for both
//! the change diff and the exam reminders. Everything it needs from the platform (the store, HTTP
//! and the notification itself) comes through [`Env`], so the same poll runs in the frontend's
//! WASM on desktop and natively in the Android background service, where there is no webview to
//! run it in (see `altis::background` in the Tauri backend).

mod changes;
mod exams;
mod state;
mod wording;

pub use state::NotificationState;

use crate::env::Env;
use crate::models::absences::Absence;
use crate::models::messages::MessagePreview;
use crate::models::timetable::MyTimeTable;
use crate::settings::NotificationSettings;
use crate::store::Store;
use crate::untis::client::UntisClient;
use crate::untis::week::{school_year, Week};
use chrono::Local;
use wording::{format_day, format_time};

/// What a notification says
#[derive(Clone, PartialEq, Debug)]
struct Notification {
    title: String,
    body: String,
}

/// The weeks a poll looks at. Next week matters as much as this one: a lesson dropped on Friday
/// for the Monday after is exactly the change worth knowing about early.
fn tracked_weeks() -> [Week; 2] {
    let current = Week::current();
    let next = current.next();
    [current, next]
}

/// Writes the bookkeeping out, then shows the notifications - in that order.
///
/// Whatever stops a notification from being sent a second time has to be on disk *before* it is
/// sent, never after: a poll that dies in between - a background job Android kills for taking too
/// long, a request that never comes back, a process that goes away - would otherwise leave the old
/// state behind and send the very same notifications again on every later poll, for as long as the
/// change is still there.
async fn record_then_notify<E: Env>(state: &NotificationState, notifications: &[Notification]) {
    let _ = Store::<E>::save_notification_state(state);

    let env = E::default();
    for notification in notifications {
        env.notify(&notification.title, &notification.body).await;
    }
}

/// Runs a single poll. Re-reads the settings every time, so changes made in the Settings screen
/// take effect on the next poll without anything having to be restarted.
pub async fn run_once<E: Env>() {
    let settings = Store::<E>::settings_or_default();
    let notifications = &settings.notification_settings;
    if !notifications.enabled || !settings.untis_auth.is_complete() {
        return;
    }

    let Ok(client) = UntisClient::<E>::new() else { return };
    let mut state = Store::<E>::notification_state_or_default();

    if notifications.timetable_changes.enabled || notifications.exam_reminders.enabled {
        check_timetable(&client, &mut state, notifications).await;
    }

    if notifications.message_notifications.enabled {
        check_messages(&client, &mut state).await;
    }

    if notifications.absence_notifications.enabled {
        check_absences(&client, &mut state, &settings.untis_auth.user_identifier).await;
    }

    let _ = Store::<E>::save_notification_state(&state);
}

/// Fetches the tracked weeks' personal timetable fresh (deliberately not through the frontend's
/// hour-long cache, this is what should be near-real-time), diffs each against what was seen last
/// poll, and scans the same data for exams that are now within a reminder's lead time.
async fn check_timetable<E: Env>(client: &UntisClient<E>, state: &mut NotificationState, settings: &NotificationSettings) {
    let weeks = tracked_weeks();
    let mut fetched: Vec<MyTimeTable> = Vec::with_capacity(weeks.len());

    for week in &weeks {
        let Ok(fresh) = client.get_my_timetable(week).await else { continue };

        if settings.timetable_changes.enabled {
            // the fresh week becomes what the next poll is compared against
            let previous = state.last_my_timetable.insert(week.start.to_string(), fresh.clone());
            let due = match previous {
                Some(previous) => changes::unsent(state, changes::timetable_changes(&previous, &fresh, &settings.timetable_changes)),
                None => Vec::new(),
            };
            record_then_notify::<E>(state, &due).await;
        }

        fetched.push(fresh);
    }

    // a week that has rolled out of the tracked range is never diffed against again, and a day
    // that is over can't have its notifications repeated any more
    state.last_my_timetable.retain(|start, _| weeks.iter().any(|week| week.start.to_string() == *start));
    let now = Local::now().naive_local();
    state.sent_change_notifications.retain(|(date, _, _)| *date >= now.date());

    let exams = &settings.exam_reminders;
    if exams.enabled && !exams.lead_times_minutes.is_empty() {
        let due = exams::due_reminders(&fetched, state, exams, now);
        record_then_notify::<E>(state, &due).await;
    }
}

/// Notifies about any inbox message not seen in a previous poll
async fn check_messages<E: Env>(client: &UntisClient<E>, state: &mut NotificationState) {
    let Ok(messages) = client.get_messages().await else { return };

    let ids: Vec<i32> = messages.iter().map(|message| message.id).collect();
    let unseen = state::record_seen(&mut state.known_message_ids, &ids);
    let due: Vec<Notification> = messages.iter().filter(|message| unseen.contains(&message.id)).map(message_notification).collect();

    record_then_notify::<E>(state, &due).await;
}

fn message_notification(message: &MessagePreview) -> Notification {
    let sender = match message.sender.display_name.is_empty() {
        true => "Untis",
        false => &message.sender.display_name,
    };
    Notification { title: format!("New message from {sender}"), body: message.subject.clone() }
}

/// Notifies about absences that weren't in a previous poll.
///
/// An absence the student entered themselves is recorded but never notified about - they were the
/// one who typed it in a minute ago. What's worth hearing about is a teacher marking them absent.
async fn check_absences<E: Env>(client: &UntisClient<E>, state: &mut NotificationState, own_user: &str) {
    let (start, end) = school_year(0);
    let Ok(data) = client.get_absences(start, end).await else { return };

    let ids: Vec<i32> = data.absences.iter().map(|absence| absence.id).collect();
    let unseen = state::record_seen(&mut state.known_absence_ids, &ids);
    let is_own = |absence: &Absence| !own_user.is_empty() && absence.created_user.eq_ignore_ascii_case(own_user);
    let due: Vec<Notification> = data
        .absences
        .iter()
        .filter(|absence| unseen.contains(&absence.id) && !is_own(absence))
        .map(|absence| Notification { title: "New absence".to_string(), body: describe_absence(absence) })
        .collect();

    record_then_notify::<E>(state, &due).await;
}

/// `Wed 07 Oct at 07:50-08:40 (Arzt)`
fn describe_absence(absence: &Absence) -> String {
    let day = match absence.start_date == absence.end_date {
        true => format_day(absence.start_date),
        false => format!("{} - {}", format_day(absence.start_date), format_day(absence.end_date)),
    };
    let times = format!("{}-{}", format_time(absence.start_time), format_time(absence.end_time));

    match absence.reason.is_empty() {
        true => format!("{day} at {times}"),
        false => format!("{day} at {times} ({})", absence.reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{Headers, HttpResponse};
    use crate::settings::{AuthSettings, Settings};
    use crate::test_env::TestEnv;
    use chrono::NaiveDate;
    use futures::executor::block_on;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn absence(start: (i32, u32, u32), end: (i32, u32, u32), reason: &str) -> Absence {
        Absence {
            id: 1,
            start_date: NaiveDate::from_ymd_opt(start.0, start.1, start.2).unwrap(),
            end_date: NaiveDate::from_ymd_opt(end.0, end.1, end.2).unwrap(),
            start_time: chrono::NaiveTime::from_hms_opt(7, 50, 0).unwrap(),
            end_time: chrono::NaiveTime::from_hms_opt(8, 40, 0).unwrap(),
            reason: reason.to_string(),
            reason_id: 0,
            text: String::new(),
            created_user: "fimi".to_string(),
            can_edit: false,
            excuse_status: String::new(),
            is_excused: false,
        }
    }

    #[test]
    fn describes_an_absence_by_when_it_is() {
        let one_day = absence((2026, 10, 7), (2026, 10, 7), "Arzt");
        assert_eq!(describe_absence(&one_day), "Wed 07 Oct at 07:50-08:40 (Arzt)");

        let no_reason = absence((2026, 10, 7), (2026, 10, 7), "");
        assert_eq!(describe_absence(&no_reason), "Wed 07 Oct at 07:50-08:40");

        let several_days = absence((2026, 10, 7), (2026, 10, 9), "");
        assert_eq!(describe_absence(&several_days), "Wed 07 Oct - Fri 09 Oct at 07:50-08:40");
    }

    /// What the pretend school answers with. A test changes it between two polls.
    #[derive(Clone, Default)]
    struct School {
        /// the room of the one lesson of every week, on its Monday at 08:00
        room: String,
        /// (id, sender, subject)
        messages: Vec<(i32, &'static str, &'static str)>,
        /// (id, the user who entered it)
        absences: Vec<(i32, &'static str)>,
    }

    impl School {
        fn respond(&self, url: &str) -> String {
            let query = |name: &str| url.split(['?', '&']).find_map(|pair| pair.strip_prefix(&format!("{name}="))).unwrap_or_default().to_string();

            if url.contains("jsonrpc_intern.do") || url.contains("?school=") {
                return "{}".to_string();
            }
            if url.ends_with("api/token/new") {
                // expires in 2100
                return "eyJhbGciOiJIUzI1NiJ9.eyJleHAiOjQxMDI0NDQ4MDB9.c2ln".to_string();
            }
            if url.contains("/timetable/filter") {
                return r#"{"preSelected": {"id": 555, "displayName": "Max"}}"#.to_string();
            }
            if url.contains("/timetable/entries") {
                let monday = query("start");
                return format!(
                    r#"{{"days": [{{"date": "{monday}", "gridEntries": [{{
                        "duration": {{"start": "{monday}T08:00", "end": "{monday}T08:50"}}, "status": "REGULAR",
                        "position1": [{{"current": {{"type": "SUBJECT", "shortName": "M"}}}}],
                        "position2": [{{"current": {{"type": "ROOM", "shortName": "{}"}}}}]
                    }}]}}]}}"#,
                    self.room,
                );
            }
            if url.ends_with("/messages") {
                let messages: Vec<String> = self.messages.iter().map(|(id, sender, subject)| {
                    format!(r#"{{"id": {id}, "subject": "{subject}", "sender": {{"displayName": "{sender}"}}, "sentDateTime": "2026-09-14T08:00:00"}}"#)
                }).collect();
                return format!(r#"{{"incomingMessages": [{}]}}"#, messages.join(","));
            }
            if url.contains("/absences/students") {
                let absences: Vec<String> = self.absences.iter().map(|(id, user)| {
                    format!(r#"{{"id": {id}, "startDate": 20261007, "endDate": 20261007, "startTime": 750, "endTime": 840, "createdUser": "{user}"}}"#)
                }).collect();
                return format!(r#"{{"data": {{"absences": [{}]}}}}"#, absences.join(","));
            }
            panic!("The poll asked for something the pretend school doesn't have: {url}");
        }
    }

    /// Logs the student in and returns the handle the school's answers are changed through
    fn school(initial: School) -> Rc<RefCell<School>> {
        TestEnv::reset();
        let auth = AuthSettings { school_identifier: "school".to_string(), user_identifier: "max".to_string(), secret: "JBSWY3DPEHPK3PXP".to_string() };
        Store::<TestEnv>::save_settings(&Settings { untis_auth: auth, ..Default::default() }).unwrap();

        let school = Rc::new(RefCell::new(initial));
        let answering = school.clone();
        TestEnv::respond_with(move |request| {
            let cookies = ["JSESSIONID=ABC", "Tenant-Id=42", "schoolname=_c2Nob29s"].map(String::from).to_vec();
            Ok(HttpResponse { headers: Headers::from([("set-cookie".to_string(), cookies)]), body: answering.borrow().respond(&request.url) })
        });
        school
    }

    fn poll() -> Vec<(String, String)> {
        block_on(run_once::<TestEnv>());
        TestEnv::take_notifications()
    }

    fn change_settings(change: impl FnOnce(&mut Settings)) {
        let mut settings = Store::<TestEnv>::settings_or_default();
        change(&mut settings);
        Store::<TestEnv>::save_settings(&settings).unwrap();
    }

    fn initial() -> School {
        School { room: "R1".to_string(), messages: vec![(1, "Smith", "Welcome")], absences: vec![(1, "SMI")] }
    }

    #[test]
    fn the_first_poll_only_takes_stock() {
        school(initial());

        assert_eq!(poll(), vec![]);

        let state = Store::<TestEnv>::notification_state_or_default();
        assert_eq!(state.last_my_timetable.len(), 2);
        assert!(state.known_message_ids.contains(&1) && state.known_absence_ids.contains(&1));
    }

    #[test]
    fn a_poll_that_finds_nothing_new_says_nothing() {
        school(initial());
        poll();

        assert_eq!(poll(), vec![]);
    }

    #[test]
    fn reports_a_room_change_in_both_tracked_weeks_once() {
        let school = school(initial());
        poll();

        school.borrow_mut().room = "R2".to_string();
        let notifications = poll();

        assert_eq!(notifications.len(), 2);
        for (title, body) in &notifications {
            assert_eq!(title, "Room changed");
            assert!(body.starts_with("M on Mon ") && body.ends_with(" at 08:00-08:50 is now in R2 (was R1)"), "{body}");
        }
        assert_ne!(notifications[0].1, notifications[1].1);

        assert_eq!(poll(), vec![]);
    }

    #[test]
    fn reports_new_messages_and_absences_but_not_the_students_own() {
        let school = school(initial());
        poll();

        school.borrow_mut().messages.insert(0, (2, "", "Timetable"));
        school.borrow_mut().absences.extend([(2, "Max"), (3, "JON")]);

        assert_eq!(poll(), vec![
            ("New message from Untis".to_string(), "Timetable".to_string()),
            ("New absence".to_string(), "Wed 07 Oct at 07:50-08:40".to_string()),
        ]);
        // the student's own absence is known now all the same
        assert!(Store::<TestEnv>::notification_state_or_default().known_absence_ids.contains(&2));
    }

    #[test]
    fn says_nothing_about_what_is_switched_off() {
        let school = school(initial());
        poll();
        change_settings(|settings| {
            settings.notification_settings.timetable_changes.enabled = false;
            settings.notification_settings.message_notifications.enabled = false;
        });

        *school.borrow_mut() = School { room: "R2".to_string(), messages: vec![(2, "Smith", "New")], absences: vec![(1, "SMI"), (3, "JON")] };
        let notifications = poll();

        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].0, "New absence");
    }

    #[test]
    fn does_not_poll_at_all_when_notifications_are_off_or_there_is_no_login() {
        school(initial());
        change_settings(|settings| settings.notification_settings.enabled = false);
        poll();
        assert!(TestEnv::requests().is_empty());

        change_settings(|settings| {
            settings.notification_settings.enabled = true;
            settings.untis_auth.secret.clear();
        });
        poll();
        assert!(TestEnv::requests().is_empty());
    }
}
