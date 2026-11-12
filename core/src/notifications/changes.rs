//! What changed in a week's timetable between two polls.

use super::wording::*;
use super::{Notification, NotificationState};
use crate::models::timetable::{LessonBlock, LessonStatus, MyTimeTable};
use crate::settings::TimetableChangeSettings;
use chrono::NaiveDate;

/// A notification about a lesson on the given day
pub(super) struct Change {
    date: NaiveDate,
    notification: Notification,
}

impl Change {
    fn new(date: NaiveDate, title: &str, body: String) -> Self {
        Self { date, notification: Notification { title: title.to_string(), body } }
    }
}

/// The changes that haven't been notified about yet, recorded as notified. However a poll pairs
/// the lessons up, the very same notification for the same day is never sent twice.
pub(super) fn unsent(state: &mut NotificationState, changes: Vec<Change>) -> Vec<Notification> {
    changes
        .into_iter()
        .filter(|change| {
            let Notification { title, body } = &change.notification;
            state.sent_change_notifications.insert((change.date, title.clone(), body.clone()))
        })
        .map(|change| change.notification)
        .collect()
}

/// A day's lessons paired up across two polls
struct PairedLessons<'a> {
    /// a lesson as it was, and as it is now
    pairs: Vec<(&'a LessonBlock, &'a LessonBlock)>,
    /// only in the fresh day
    added: Vec<&'a LessonBlock>,
    /// only in the previous one
    removed: Vec<&'a LessonBlock>,
}

/// Whether two lessons are the same in everything a notification is about
fn is_unchanged(old: &LessonBlock, new: &LessonBlock) -> bool {
    old.time_range == new.time_range
        && old.status == new.status
        && subject_name(old) == subject_name(new)
        && teacher_names(old) == teacher_names(new)
        && room_names(old) == room_names(new)
}

/// Pairs a day's lessons up across two polls, along with what was only in the fresh day (added)
/// and what was only in the previous one (removed).
///
/// Two passes, because a single map keyed on anything less than the whole lesson collapses the
/// duplicates an ordinary school day is full of - the same subject with the same teacher twice in
/// a day, or a double period split into two blocks. Collapsed, the morning block ends up compared
/// against the afternoon one, and every poll reports the same time and room change that never
/// happened, for as long as both lessons stand.
///
/// The first pass takes the lessons that are identical anyway and consumes both sides, so only
/// what actually differs is left to guess about. Identical means everything a notification is
/// about, rooms and status included: Untis lists a cancelled lesson next to its replacement, often
/// with the same subject, teacher and time, and in no fixed order. Matched on less, the two were
/// paired the wrong way round whenever the order flipped, and every such poll reported the same
/// cancellation again.
///
/// The second pairs the rest off by subject, preferring one at the same time, in start-time order.
/// Teacher is deliberately not part of that key: a substitution changes the teacher, which is the
/// very thing worth reporting, so matching on it would stop the two halves of one lesson ever
/// meeting.
fn pair_lessons<'a>(previous: &'a [LessonBlock], fresh: &'a [LessonBlock]) -> PairedLessons<'a> {
    let mut old_left: Vec<&LessonBlock> = previous.iter().collect();
    let mut new_left: Vec<&LessonBlock> = fresh.iter().collect();
    old_left.sort_by_key(|lesson| lesson.time_range.start);
    new_left.sort_by_key(|lesson| lesson.time_range.start);

    let mut pairs = Vec::new();
    let mut changed = Vec::new();

    for new_lesson in new_left {
        match old_left.iter().position(|old| is_unchanged(old, new_lesson)) {
            Some(index) => pairs.push((old_left.remove(index), new_lesson)),
            None => changed.push(new_lesson),
        }
    }

    // a lesson Untis marks as additional is new by its own account, so it only gets an old lesson
    // nothing else wants: a cancelled lesson keeps its own old self rather than its replacement
    // taking it, whichever order the two came in
    changed.sort_by_key(|lesson| lesson.status == LessonStatus::Additional);

    let mut added = Vec::new();
    for new_lesson in changed {
        let same_subject = |old: &&LessonBlock| subject_name(old) == subject_name(new_lesson);
        let counterpart = old_left
            .iter()
            .position(|old| same_subject(old) && old.time_range == new_lesson.time_range)
            .or_else(|| old_left.iter().position(same_subject));

        match counterpart {
            Some(index) => pairs.push((old_left.remove(index), new_lesson)),
            None => added.push(new_lesson),
        }
    }

    PairedLessons { pairs, added, removed: old_left }
}

/// Everything that changed between two polls of a week, worth a notification
pub(super) fn timetable_changes(previous: &MyTimeTable, fresh: &MyTimeTable, settings: &TimetableChangeSettings) -> Vec<Change> {
    let mut changes = Vec::new();

    for (previous_day, fresh_day) in previous.timetable.days.iter().zip(&fresh.timetable.days) {
        if previous_day.date != fresh_day.date {
            continue;
        }
        let date = fresh_day.date;
        let PairedLessons { pairs, added, removed } = pair_lessons(&previous_day.lessons, &fresh_day.lessons);

        for (old_lesson, new_lesson) in pairs {
            changes.extend(lesson_changes(old_lesson, new_lesson, date, settings));
        }

        if settings.notify_substitution {
            let appeared = added.iter().filter(|lesson| !lesson.is_cancelled());
            changes.extend(appeared.map(|lesson| Change::new(date, "New lesson added", describe_lesson(lesson, date))));
        }

        if settings.notify_cancelled {
            // one that was already cancelled was reported as such when it happened, and dropping
            // out of the timetable afterwards is not a second thing to hear about
            let dropped = removed.iter().filter(|lesson| !lesson.is_cancelled());
            changes.extend(dropped.map(|lesson| Change::new(date, "Lesson removed", describe_lesson(lesson, date))));
        }
    }
    changes
}

/// What changed about one lesson
fn lesson_changes(old_lesson: &LessonBlock, new_lesson: &LessonBlock, date: NaiveDate, settings: &TimetableChangeSettings) -> Vec<Change> {
    let subject = subject_name(new_lesson);
    let day = format_day(date);
    let time = format_time_range(new_lesson);
    let change = |title: &str, body: String| Change::new(date, title, body);

    match (old_lesson.is_cancelled(), new_lesson.is_cancelled()) {
        (false, true) if settings.notify_cancelled => {
            return vec![change("Lesson cancelled", format!("{subject} on {day} at {time} was cancelled"))];
        }
        (true, false) if settings.notify_cancelled => {
            return vec![change("Lesson reinstated", format!("{subject} on {day} at {time} is back on"))];
        }
        // whatever else changed about a lesson that doesn't take place is of no interest
        (_, true) => return Vec::new(),
        _ => {}
    }

    let mut changes = Vec::new();

    if settings.notify_time_change && old_lesson.time_range != new_lesson.time_range {
        let old_time = format_time_range(old_lesson);
        changes.push(change("Time changed", format!("{subject} on {day} is now at {time} (was {old_time})")));
    }

    if settings.notify_room_change {
        let (old_rooms, new_rooms) = (room_names(old_lesson), room_names(new_lesson));
        if old_rooms != new_rooms {
            let body = format!("{subject} on {day} at {time} is now in {} (was {})", format_names(&new_rooms), format_names(&old_rooms));
            changes.push(change("Room changed", body));
        }
    }

    if settings.notify_substitution {
        let (old_teachers, new_teachers) = (teacher_names(old_lesson), teacher_names(new_lesson));
        if old_teachers != new_teachers {
            let body = format!("{subject} on {day} at {time} is now with {} (was {})", format_names(&new_teachers), format_names(&old_teachers));
            changes.push(change("Teacher changed", body));
        } else if old_lesson.status != new_lesson.status && new_lesson.status == LessonStatus::Changed {
            changes.push(change("Lesson changed", format!("{subject} on {day} at {time} was changed")));
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::timetable::{ChangeStatus, DayTimeTable, Entity, Room, Subject, Teacher, TimeRange, Tracked, WeekTimeTable};

    fn tracked(inner: Entity) -> Tracked<Entity> {
        Tracked { inner, status: ChangeStatus::Regular }
    }

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 14).unwrap()
    }

    fn lesson(subject: &str, teacher: &str, room: &str, start: (u32, u32), end: (u32, u32)) -> LessonBlock {
        LessonBlock {
            time_range: TimeRange {
                start: date().and_hms_opt(start.0, start.1, 0).unwrap(),
                end: date().and_hms_opt(end.0, end.1, 0).unwrap(),
            },
            entities: vec![
                tracked(Entity::Subject(Subject { short_name: subject.to_string(), ..Default::default() })),
                tracked(Entity::Teacher(Teacher { short_name: teacher.to_string(), ..Default::default() })),
                tracked(Entity::Room(Room { name: room.to_string() })),
            ],
            ..Default::default()
        }
    }

    fn with_status(mut lesson: LessonBlock, status: LessonStatus) -> LessonBlock {
        lesson.status = status;
        lesson
    }

    fn week_of(lessons: Vec<LessonBlock>) -> MyTimeTable {
        MyTimeTable {
            timetable: WeekTimeTable { days: vec![DayTimeTable { date: date(), lessons }] },
            ..Default::default()
        }
    }

    fn changes(previous: Vec<LessonBlock>, fresh: Vec<LessonBlock>, settings: &TimetableChangeSettings) -> Vec<(String, String)> {
        timetable_changes(&week_of(previous), &week_of(fresh), settings)
            .into_iter()
            .map(|change| (change.notification.title, change.notification.body))
            .collect()
    }

    fn titles(previous: Vec<LessonBlock>, fresh: Vec<LessonBlock>) -> Vec<String> {
        changes(previous, fresh, &TimetableChangeSettings::default()).into_iter().map(|(title, _)| title).collect()
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

        let paired = pair_lessons(&previous, &fresh);

        assert_eq!(paired.pairs.len(), 2);
        assert!(paired.added.is_empty() && paired.removed.is_empty());
        for (old, new) in paired.pairs {
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

        let paired = pair_lessons(&previous, &fresh);

        assert_eq!(paired.pairs.len(), 1);
        assert!(paired.added.is_empty() && paired.removed.is_empty());
        let (old, new) = paired.pairs[0];
        assert_eq!(teacher_names(old), vec!["SMI".to_string()]);
        assert_eq!(teacher_names(new), vec!["JON".to_string()]);
    }

    #[test]
    fn pairs_a_moved_lesson_with_its_old_self() {
        let previous = [lesson("Math", "SMI", "R1", (8, 0), (8, 45))];
        let fresh = [lesson("Math", "SMI", "R2", (10, 0), (10, 45))];

        let paired = pair_lessons(&previous, &fresh);

        assert_eq!(paired.pairs.len(), 1);
        assert!(paired.added.is_empty() && paired.removed.is_empty());
    }

    #[test]
    fn reports_lessons_that_appeared_and_disappeared() {
        let previous = [lesson("Math", "SMI", "R1", (8, 0), (8, 45))];
        let fresh = [lesson("Bio", "GRA", "R3", (8, 0), (8, 45))];

        let paired = pair_lessons(&previous, &fresh);

        assert!(paired.pairs.is_empty());
        assert_eq!(paired.added.len(), 1);
        assert_eq!(subject_name(paired.added[0]), "Bio");
        assert_eq!(paired.removed.len(), 1);
        assert_eq!(subject_name(paired.removed[0]), "Math");
    }

    /// The bug that kept repeating the same cancellation: Untis lists a cancelled lesson next to
    /// its replacement with the same subject, teacher and time, in no fixed order, and pairing them
    /// up on those alone swapped the two whenever the order flipped
    #[test]
    fn a_cancelled_lesson_and_its_replacement_in_another_order_are_no_change() {
        let cancelled = with_status(lesson("D", "SMI", "R1", (8, 0), (8, 50)), LessonStatus::Cancelled);
        let replacement = with_status(lesson("D", "SMI", "R2", (8, 0), (8, 50)), LessonStatus::Additional);

        let previous = vec![cancelled.clone(), replacement.clone()];
        let fresh = vec![replacement, cancelled];

        assert_eq!(titles(previous, fresh), Vec::<String>::new());
    }

    #[test]
    fn a_lesson_that_gets_cancelled_and_replaced_is_reported_once() {
        let regular = lesson("D", "SMI", "R1", (8, 0), (8, 50));
        let fresh = vec![
            with_status(lesson("D", "SMI", "R2", (8, 0), (8, 50)), LessonStatus::Additional),
            with_status(regular.clone(), LessonStatus::Cancelled),
        ];

        assert_eq!(titles(vec![regular], fresh.clone()), vec!["Lesson cancelled", "New lesson added"]);

        // the next poll sees the same timetable and has nothing to say about it
        assert_eq!(titles(fresh.clone(), fresh), Vec::<String>::new());
    }

    #[test]
    fn words_each_kind_of_change() {
        let settings = TimetableChangeSettings::default();
        let old = lesson("Math", "SMI", "R1", (8, 0), (8, 45));

        let cancelled = with_status(old.clone(), LessonStatus::Cancelled);
        assert_eq!(changes(vec![old.clone()], vec![cancelled.clone()], &settings), vec![
            ("Lesson cancelled".to_string(), "Math on Mon 14 Sep at 08:00-08:45 was cancelled".to_string()),
        ]);
        assert_eq!(changes(vec![cancelled], vec![old.clone()], &settings), vec![
            ("Lesson reinstated".to_string(), "Math on Mon 14 Sep at 08:00-08:45 is back on".to_string()),
        ]);

        let moved = lesson("Math", "JON", "R2", (10, 0), (10, 45));
        assert_eq!(changes(vec![old.clone()], vec![moved], &settings), vec![
            ("Time changed".to_string(), "Math on Mon 14 Sep is now at 10:00-10:45 (was 08:00-08:45)".to_string()),
            ("Room changed".to_string(), "Math on Mon 14 Sep at 10:00-10:45 is now in R2 (was R1)".to_string()),
            ("Teacher changed".to_string(), "Math on Mon 14 Sep at 10:00-10:45 is now with JON (was SMI)".to_string()),
        ]);

        let marked = with_status(old.clone(), LessonStatus::Changed);
        assert_eq!(changes(vec![old.clone()], vec![marked], &settings), vec![
            ("Lesson changed".to_string(), "Math on Mon 14 Sep at 08:00-08:45 was changed".to_string()),
        ]);

        assert_eq!(changes(vec![old.clone()], vec![], &settings), vec![
            ("Lesson removed".to_string(), "Math on Mon 14 Sep at 08:00-08:45".to_string()),
        ]);
        assert_eq!(changes(vec![], vec![old], &settings), vec![
            ("New lesson added".to_string(), "Math on Mon 14 Sep at 08:00-08:45".to_string()),
        ]);
    }

    #[test]
    fn only_the_kinds_that_are_switched_on_are_reported() {
        let old = lesson("Math", "SMI", "R1", (8, 0), (8, 45));
        let moved = lesson("Math", "JON", "R2", (10, 0), (10, 45));
        let only = |change: fn(&mut TimetableChangeSettings)| {
            let mut settings = TimetableChangeSettings { enabled: true, notify_cancelled: false, notify_room_change: false, notify_time_change: false, notify_substitution: false };
            change(&mut settings);
            settings
        };
        let titles = |settings: &TimetableChangeSettings| -> Vec<String> {
            changes(vec![old.clone()], vec![moved.clone()], settings).into_iter().map(|(title, _)| title).collect()
        };

        assert_eq!(titles(&only(|settings| settings.notify_room_change = true)), vec!["Room changed"]);
        assert_eq!(titles(&only(|settings| settings.notify_time_change = true)), vec!["Time changed"]);
        assert_eq!(titles(&only(|settings| settings.notify_substitution = true)), vec!["Teacher changed"]);
        assert_eq!(titles(&only(|_| {})), Vec::<String>::new());

        // with cancellations off a cancelled lesson is not reported as anything else either
        let cancelled = with_status(moved.clone(), LessonStatus::Cancelled);
        let everything_else = TimetableChangeSettings { notify_cancelled: false, ..Default::default() };
        assert_eq!(changes(vec![old.clone()], vec![cancelled], &everything_else), vec![]);
    }

    #[test]
    fn the_same_notification_is_never_sent_twice() {
        let mut state = NotificationState::default();
        let change = |date| Change::new(date, "Lesson cancelled", "D on Mon 14 Sep at 08:00-08:50 was cancelled".to_string());

        assert_eq!(unsent(&mut state, vec![change(date())]).len(), 1);
        assert_eq!(unsent(&mut state, vec![change(date())]).len(), 0);
        // the same words about another day are another notification
        assert_eq!(unsent(&mut state, vec![change(date().succ_opt().unwrap())]).len(), 1);
    }
}
