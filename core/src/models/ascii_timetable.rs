//! The week as a plain text table, for the "Force ASCII Timetable" setting.

use crate::models::timetable::{Entity, LessonBlock, WeekTimeTable};
use chrono::{NaiveDateTime, Timelike};
use std::collections::BTreeSet;
use std::fmt::Write;

const DAY_COLUMN_WIDTH: usize = 34;
const TIME_COLUMN_WIDTH: usize = 5;
/// what a column adds to its width: the `| ` before and the space after
const COLUMN_FRAME_WIDTH: usize = 3;

/// A stretch between two lessons this short is a break rather than a period, and only gets a row
/// if some lesson starts or ends inside it
const SHORT_SLOT_MINUTES: u32 = 35;

/// A time of day, in minutes since midnight
type Minutes = u32;

fn minutes(time: NaiveDateTime) -> Minutes {
    time.hour() * 60 + time.minute()
}

fn format_minutes(time: Minutes) -> String {
    format!("{:02}:{:02}", time / 60, time % 60)
}

/// A stretch of the day no lesson starts or ends inside
#[derive(Clone, Copy)]
struct Slot {
    start: Minutes,
    end: Minutes,
}

impl Slot {
    fn intersects(self, lesson: &LessonBlock) -> bool {
        minutes(lesson.time_range.start) < self.end && minutes(lesson.time_range.end) > self.start
    }

    /// whether the lesson both starts before and ends after the slot
    fn is_spanned_by(self, lesson: &LessonBlock) -> bool {
        minutes(lesson.time_range.start) < self.start && minutes(lesson.time_range.end) > self.end
    }

    fn is_short(self) -> bool {
        self.end - self.start < SHORT_SLOT_MINUTES
    }
}

/// The lines a slot is written in, one per sort of information about its lessons
#[derive(Clone, Copy)]
enum Line {
    Subject,
    Teachers,
    Classes,
    Rooms,
    Status,
}

const LINES: [Line; 5] = [Line::Subject, Line::Teachers, Line::Classes, Line::Rooms, Line::Status];

impl Line {
    fn text(self, lesson: &LessonBlock) -> String {
        let names = |wanted: fn(&Entity) -> bool| {
            lesson.entities.iter().filter(move |entity| wanted(&entity.inner)).map(|entity| entity.inner.name())
        };

        match self {
            Line::Subject => names(Entity::is_subject).next().unwrap_or_default(),
            Line::Teachers => names(Entity::is_teacher).collect::<Vec<_>>().join(","),
            Line::Classes => names(Entity::is_class).collect::<Vec<_>>().join(","),
            Line::Rooms => names(Entity::is_room).collect::<Vec<_>>().join(","),
            Line::Status => lesson.status.as_str().to_string(),
        }
    }
}

/// Splits the day at every time a lesson of the week starts or ends
fn slots(timetable: &WeekTimeTable) -> Vec<Slot> {
    let bounds: BTreeSet<Minutes> = lessons(timetable)
        .flat_map(|lesson| [minutes(lesson.time_range.start), minutes(lesson.time_range.end)])
        .collect();
    let bounds: Vec<Minutes> = bounds.into_iter().collect();

    bounds.windows(2).map(|pair| Slot { start: pair[0], end: pair[1] }).collect()
}

fn lessons(timetable: &WeekTimeTable) -> impl Iterator<Item = &LessonBlock> {
    timetable.days.iter().flat_map(|day| &day.lessons)
}

/// Whether a slot gets a row: some lesson has to take place in it, and in a short one that
/// lesson has to start or end there rather than merely run through
fn is_shown(slot: Slot, timetable: &WeekTimeTable) -> bool {
    match slot.is_short() {
        true => lessons(timetable).any(|lesson| slot.intersects(lesson) && !slot.is_spanned_by(lesson)),
        false => lessons(timetable).any(|lesson| slot.intersects(lesson)),
    }
}

fn cell(slot: Slot, line: Line, day_lessons: &[LessonBlock]) -> String {
    let text = day_lessons
        .iter()
        .filter(|lesson| slot.intersects(lesson))
        .map(|lesson| line.text(lesson))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");

    match text.chars().count() > DAY_COLUMN_WIDTH {
        true => text.chars().take(DAY_COLUMN_WIDTH - 1).collect::<String>() + "…",
        false => text,
    }
}

pub fn render(timetable: &WeekTimeTable) -> String {
    let slots = slots(timetable);
    if lessons(timetable).next().is_none() {
        return "No lessons scheduled.".to_string();
    }

    let total_width = TIME_COLUMN_WIDTH + COLUMN_FRAME_WIDTH + timetable.days.len() * (DAY_COLUMN_WIDTH + COLUMN_FRAME_WIDTH);
    let separator = format!("{:-<total_width$}\n", "");
    let mut output = String::new();

    let _ = write!(output, "| {:<TIME_COLUMN_WIDTH$} ", "Time");
    for day in &timetable.days {
        let _ = write!(output, "| {:^DAY_COLUMN_WIDTH$} ", day.date.format("%a").to_string());
    }
    output.push_str("|\n");
    output.push_str(&separator);

    for slot in slots.into_iter().filter(|slot| is_shown(*slot, timetable)) {
        for (index, line) in LINES.into_iter().enumerate() {
            // the slot's start and end are written next to its first two lines
            let time = match index {
                0 => format_minutes(slot.start),
                1 => format_minutes(slot.end),
                _ => String::new(),
            };

            let _ = write!(output, "| {time:<TIME_COLUMN_WIDTH$} ");
            for day in &timetable.days {
                let _ = write!(output, "| {:^DAY_COLUMN_WIDTH$} ", cell(slot, line, &day.lessons));
            }
            output.push_str("|\n");
        }
        output.push_str(&separator);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::timetable::{ChangeStatus, DayTimeTable, LessonStatus, Room, Subject, Teacher, TimeRange, Tracked};
    use chrono::NaiveDate;

    fn lesson(day: u32, start: (u32, u32), end: (u32, u32), subject: &str) -> LessonBlock {
        let date = NaiveDate::from_ymd_opt(2026, 9, day).unwrap();
        let tracked = |inner| Tracked { inner, status: ChangeStatus::Regular };
        LessonBlock {
            time_range: TimeRange {
                start: date.and_hms_opt(start.0, start.1, 0).unwrap(),
                end: date.and_hms_opt(end.0, end.1, 0).unwrap(),
            },
            entities: vec![
                tracked(Entity::Subject(Subject { short_name: subject.to_string(), ..Default::default() })),
                tracked(Entity::Teacher(Teacher { short_name: "SMI".to_string(), ..Default::default() })),
                tracked(Entity::Room(Room { name: "R1".to_string() })),
            ],
            status: LessonStatus::from("REGULAR".to_string()),
            ..Default::default()
        }
    }

    fn week(lessons: Vec<LessonBlock>) -> WeekTimeTable {
        let days = [14, 15]
            .into_iter()
            .map(|day| {
                let date = NaiveDate::from_ymd_opt(2026, 9, day).unwrap();
                DayTimeTable { date, lessons: lessons.iter().filter(|l| l.time_range.start.date() == date).cloned().collect() }
            })
            .collect();
        WeekTimeTable { days }
    }

    #[test]
    fn an_empty_week_says_so() {
        assert_eq!(render(&week(vec![])), "No lessons scheduled.");
    }

    #[test]
    fn writes_a_row_of_five_lines_per_period() {
        let table = render(&week(vec![lesson(14, (8, 0), (8, 50), "M"), lesson(15, (8, 0), (8, 50), "D")]));
        let lines: Vec<&str> = table.lines().collect();

        // the header and its rule, then the five lines of the one period and theirs
        assert_eq!(lines.len(), 8);
        assert!(lines[0].starts_with("| Time  |") && lines[0].contains("Mon") && lines[0].contains("Tue"));
        assert!(lines[2].starts_with("| 08:00 |") && lines[2].contains(" M ") && lines[2].contains(" D "));
        assert!(lines[3].starts_with("| 08:50 |") && lines[3].contains("SMI"));
        assert!(lines[5].contains("R1"));
        assert!(lines[6].contains("REGULAR"));
    }

    /// the five minutes between two lessons are no period of their own
    #[test]
    fn a_break_gets_no_row() {
        let table = render(&week(vec![lesson(14, (8, 0), (8, 50), "M"), lesson(14, (8, 55), (9, 45), "D")]));

        // the header and its rule, and two periods of five lines and a rule each
        assert_eq!(table.lines().count(), 14);
        assert!(table.contains("| 08:00 |") && table.contains("| 08:55 |"));
    }
}
