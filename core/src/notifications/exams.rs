use super::wording::{format_day, format_time, subject_name};
use super::{NotificationState, Notification};
use crate::models::timetable::MyTimeTable;
use crate::settings::{describe_lead_time_minutes, ExamReminderSettings};
use chrono::{NaiveDate, NaiveDateTime, TimeDelta};

const IDENTITY_DATE_FORMAT: &str = "%Y-%m-%d";

/// The reminders that are due: one per exam and configured lead time, as soon as the exam is no
/// further away than that, and never twice. Recorded as sent in the state.
pub(super) fn due_reminders(
    timetables: &[MyTimeTable],
    state: &mut NotificationState,
    settings: &ExamReminderSettings,
    now: NaiveDateTime,
) -> Vec<Notification> {
    let mut due = Vec::new();

    for day in timetables.iter().flat_map(|timetable| &timetable.timetable.days) {
        for exam in day.lessons.iter().filter(|lesson| lesson.is_exam() && !lesson.is_cancelled()) {
            let start = exam.time_range.start;
            let subject = subject_name(exam);
            let identity = format!("{}|{}|{subject}", day.date.format(IDENTITY_DATE_FORMAT), format_time(start));

            for &lead in &settings.lead_times_minutes {
                let within_lead_time = now >= start - TimeDelta::minutes(lead) && now < start;
                if within_lead_time && state.sent_exam_reminders.insert((identity.clone(), lead)) {
                    due.push(Notification {
                        title: format!("Exam in {}", describe_lead_time_minutes(lead)),
                        body: format!("{subject} exam at {} on {}", format_time(start), format_day(day.date)),
                    });
                }
            }
        }
    }

    forget_past_exams(state, now.date());
    due
}

/// Drops the reminders of exams that are over, so the set doesn't grow forever
fn forget_past_exams(state: &mut NotificationState, today: NaiveDate) {
    state.sent_exam_reminders.retain(|(identity, _)| {
        identity
            .split('|')
            .next()
            .and_then(|date| NaiveDate::parse_from_str(date, IDENTITY_DATE_FORMAT).ok())
            .is_none_or(|date| date >= today)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::timetable::{DayTimeTable, LessonBlock, LessonKind, LessonStatus, TimeRange, WeekTimeTable};

    fn date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 16).unwrap()
    }

    fn at(hour: u32, minute: u32) -> NaiveDateTime {
        date().and_hms_opt(hour, minute, 0).unwrap()
    }

    fn exam_at_ten(status: LessonStatus) -> Vec<MyTimeTable> {
        let exam = LessonBlock { time_range: TimeRange { start: at(10, 0), end: at(11, 40) }, kind: LessonKind::Exam, status, ..Default::default() };
        let timetable = WeekTimeTable { days: vec![DayTimeTable { date: date(), lessons: vec![exam] }] };
        vec![MyTimeTable { name: String::new(), timetable }]
    }

    fn titles(reminders: &[Notification]) -> Vec<&str> {
        reminders.iter().map(|reminder| reminder.title.as_str()).collect()
    }

    #[test]
    fn reminds_once_per_lead_time_when_it_is_reached() {
        let timetables = exam_at_ten(LessonStatus::default());
        let settings = ExamReminderSettings::default();
        let mut state = NotificationState::default();

        // two days before neither the day nor the hour is reached
        let early = at(10, 0) - TimeDelta::days(2);
        assert!(due_reminders(&timetables, &mut state, &settings, early).is_empty());

        let day_before = due_reminders(&timetables, &mut state, &settings, at(9, 0) - TimeDelta::hours(20));
        assert_eq!(titles(&day_before), vec!["Exam in 1 day"]);
        assert_eq!(day_before[0].body, "Lesson exam at 10:00 on Wed 16 Sep");

        assert_eq!(titles(&due_reminders(&timetables, &mut state, &settings, at(9, 30))), vec!["Exam in 1 hour"]);
        assert!(due_reminders(&timetables, &mut state, &settings, at(9, 35)).is_empty());
    }

    #[test]
    fn an_exam_that_has_started_or_was_cancelled_is_not_reminded_of() {
        let settings = ExamReminderSettings::default();

        let started = due_reminders(&exam_at_ten(LessonStatus::default()), &mut NotificationState::default(), &settings, at(10, 0));
        assert!(started.is_empty());

        let cancelled = due_reminders(&exam_at_ten(LessonStatus::Cancelled), &mut NotificationState::default(), &settings, at(9, 30));
        assert!(cancelled.is_empty());
    }

    #[test]
    fn forgets_the_reminders_of_exams_that_are_over() {
        let mut state = NotificationState::default();
        state.sent_exam_reminders.insert(("2026-09-15|10:00|M".to_string(), 60));
        state.sent_exam_reminders.insert(("2026-09-16|10:00|M".to_string(), 60));

        forget_past_exams(&mut state, date());
        assert_eq!(state.sent_exam_reminders.len(), 1);
    }
}
