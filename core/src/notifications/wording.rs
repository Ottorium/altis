//! How a lesson is put into the words of a notification.

use crate::models::timetable::{Entity, LessonBlock};
use chrono::NaiveDate;

const DAY_FORMAT: &str = "%a %d %b";
const TIME_FORMAT: &str = "%H:%M";

pub(super) fn format_day(date: NaiveDate) -> String {
    date.format(DAY_FORMAT).to_string()
}

pub(super) fn format_time(time: impl chrono::Timelike) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

pub(super) fn format_time_range(lesson: &LessonBlock) -> String {
    format!("{}-{}", lesson.time_range.start.format(TIME_FORMAT), lesson.time_range.end.format(TIME_FORMAT))
}

/// The lesson's subject, or just "Lesson" for one that has none
pub(super) fn subject_name(lesson: &LessonBlock) -> String {
    sorted_names(lesson, Entity::is_subject).into_iter().next().unwrap_or_else(|| "Lesson".to_string())
}

pub(super) fn teacher_names(lesson: &LessonBlock) -> Vec<String> {
    sorted_names(lesson, Entity::is_teacher)
}

pub(super) fn room_names(lesson: &LessonBlock) -> Vec<String> {
    sorted_names(lesson, Entity::is_room)
}

/// Sorted, so the same teachers or rooms in another order are no change
fn sorted_names(lesson: &LessonBlock, wanted: fn(&Entity) -> bool) -> Vec<String> {
    let mut names: Vec<String> = lesson.current_names(wanted).collect();
    names.sort();
    names
}

pub(super) fn format_names(names: &[String]) -> String {
    if names.is_empty() { "none".to_string() } else { names.join(", ") }
}

/// `Math on Mon 14 Sep at 08:00-08:45`
pub(super) fn describe_lesson(lesson: &LessonBlock, date: NaiveDate) -> String {
    format!("{} on {} at {}", subject_name(lesson), format_day(date), format_time_range(lesson))
}
