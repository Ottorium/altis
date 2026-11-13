//! Several timetables side by side: a column per day, split into a lane per timetable.

mod picker;
mod view;

pub use picker::ComparePicker;
pub use view::CompareView;

use altis_core::models::timetable::{EntityKind, LessonBlock, WeekTimeTable};
use altis_core::settings::VisualSettings;
use chrono::{Datelike, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};

/// What the personal timetable is stored as among the compared ones, next to the entity kinds
const ME: &str = "Me";

/// A timetable picked for comparing
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(try_from = "StoredCompared", into = "StoredCompared")]
pub enum Compared {
    /// the student's own
    Me,
    /// a class's, teacher's or room's, by the name the timetable picker shows it under
    Entity { kind: EntityKind, name: String },
}

impl Compared {
    pub fn label(&self) -> String {
        match self {
            Compared::Me => ME.to_string(),
            Compared::Entity { name, .. } => name.clone(),
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Compared::Me => "bi-person",
            Compared::Entity { kind: EntityKind::Class, .. } => "bi-people",
            Compared::Entity { kind: EntityKind::Teacher, .. } => "bi-person-badge",
            Compared::Entity { kind: EntityKind::Room, .. } => "bi-geo-alt",
        }
    }
}

/// How a compared timetable is written to localStorage: the category of the timetable picker it
/// was picked from, and its name there (none for "Me")
#[derive(Serialize, Deserialize)]
struct StoredCompared {
    category: String,
    name: String,
}

impl From<Compared> for StoredCompared {
    fn from(compared: Compared) -> Self {
        match compared {
            Compared::Me => Self { category: ME.to_string(), name: String::new() },
            Compared::Entity { kind, name } => Self { category: kind.as_str().to_string(), name },
        }
    }
}

impl TryFrom<StoredCompared> for Compared {
    type Error = String;

    fn try_from(stored: StoredCompared) -> Result<Self, Self::Error> {
        if stored.category == ME {
            return Ok(Compared::Me);
        }
        match EntityKind::from_name(&stored.category) {
            Some(kind) => Ok(Compared::Entity { kind, name: stored.name }),
            None => Err(format!("{} has no timetable to compare", stored.category)),
        }
    }
}

/// One day of the comparison: every compared timetable's lessons
#[derive(Clone, PartialEq, Debug)]
struct Day {
    date: NaiveDate,
    /// the lessons per compared timetable, in the order of the comparison
    lanes: Vec<Vec<LessonBlock>>,
}

impl Day {
    fn has_lessons(&self) -> bool {
        self.lanes.iter().any(|lessons| !lessons.is_empty())
    }
}

/// A week of several timetables side by side, ready to render
#[derive(Clone, PartialEq, Debug)]
pub struct Comparison {
    /// the compared timetables, in the order of the lanes
    names: Vec<String>,
    /// compared timetables that aren't in this week
    missing: Vec<String>,
    days: Vec<Day>,
    start: NaiveTime,
    end: NaiveTime,
}

impl Comparison {
    /// Lines the timetables up by day, each given by the name it is listed under
    pub fn of(tables: &[(String, &WeekTimeTable)], missing: Vec<String>, visual: &VisualSettings) -> Self {
        let mut dates: Vec<NaiveDate> = tables.iter().flat_map(|(_, table)| table.days.iter().map(|day| day.date)).collect();
        dates.sort();
        dates.dedup();

        let lane = |table: &WeekTimeTable, date: NaiveDate| -> Vec<LessonBlock> {
            let mut lessons: Vec<LessonBlock> = table
                .days
                .iter()
                .filter(|day| day.date == date)
                .flat_map(|day| day.lessons.iter().filter(|lesson| visual.lesson_display.shows(lesson)).cloned())
                .collect();
            lessons.iter_mut().for_each(|lesson| visual.apply_color_override(lesson));
            lessons
        };

        let days: Vec<Day> = dates
            .into_iter()
            .map(|date| Day { date, lanes: tables.iter().map(|(_, table)| lane(table, date)).collect() })
            .filter(|day| visual.weekday_override.should_show(day.date.weekday(), day.has_lessons()))
            .collect();

        let lessons = || days.iter().flat_map(|day| day.lanes.iter().flatten());
        let first_start = lessons().map(|lesson| lesson.time_range.start.time()).min();
        let last_end = lessons().map(|lesson| lesson.time_range.end.time()).max();
        let (start, end) = match (first_start, last_end) {
            (Some(start), Some(end)) => visual.padded_range(start, end),
            _ => (NaiveTime::MIN, NaiveTime::MIN),
        };

        Self { names: tables.iter().map(|(name, _)| name.clone()).collect(), missing, days, start, end }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use altis_core::models::timetable::{DayTimeTable, TimeRange};

    #[test]
    fn is_stored_the_way_it_always_was() {
        let compared = vec![Compared::Me, Compared::Entity { kind: EntityKind::Teacher, name: "SMI".to_string() }];
        let stored = r#"[{"category":"Me","name":""},{"category":"Teacher","name":"SMI"}]"#;

        assert_eq!(serde_json::to_string(&compared).unwrap(), stored);
        assert_eq!(serde_json::from_str::<Vec<Compared>>(stored).unwrap(), compared);
    }

    #[test]
    fn only_what_has_a_timetable_can_be_compared() {
        assert!(serde_json::from_str::<Compared>(r#"{"category":"AvailableRooms","name":""}"#).is_err());
    }

    fn table(lessons: &[(u32, u32, u32)]) -> WeekTimeTable {
        let date = |day| NaiveDate::from_ymd_opt(2026, 9, day).unwrap();
        let days = [14, 15, 19]
            .into_iter()
            .map(|day| DayTimeTable {
                date: date(day),
                lessons: lessons
                    .iter()
                    .filter(|(lesson_day, _, _)| *lesson_day == day)
                    .map(|&(_, start, end)| LessonBlock {
                        time_range: TimeRange { start: date(day).and_hms_opt(start, 0, 0).unwrap(), end: date(day).and_hms_opt(end, 0, 0).unwrap() },
                        ..Default::default()
                    })
                    .collect(),
            })
            .collect();
        WeekTimeTable { days }
    }

    #[test]
    fn cancelled_lessons_can_be_left_out() {
        let mut cancelled = table(&[(14, 8, 9), (15, 9, 10)]);
        cancelled.days[0].lessons[0].status = altis_core::models::timetable::LessonStatus::Cancelled;
        let mut visual = VisualSettings::default();
        visual.lesson_display.hide_cancelled = true;

        let comparison = Comparison::of(&[("A".to_string(), &cancelled)], vec![], &visual);

        // the Monday is still one of the days that are always shown, it just has nothing on it
        assert_eq!(comparison.days.iter().map(Day::has_lessons).collect::<Vec<_>>(), vec![false, true]);
    }

    #[test]
    fn every_timetable_gets_a_lane_in_every_shown_day() {
        let (first, second) = (table(&[(14, 8, 9)]), table(&[(14, 10, 12), (15, 9, 10)]));
        let comparison = Comparison::of(&[("A".to_string(), &first), ("B".to_string(), &second)], vec![], &VisualSettings::default());

        // the Saturday has no lessons and isn't one of the days that are always shown
        assert_eq!(comparison.days.len(), 2);
        assert_eq!(comparison.days[0].lanes.iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1]);
        assert_eq!(comparison.days[1].lanes.iter().map(Vec::len).collect::<Vec<_>>(), vec![0, 1]);
        // drawn from the first lesson's start to the last one's end
        assert_eq!((comparison.start, comparison.end), (NaiveTime::from_hms_opt(8, 0, 0).unwrap(), NaiveTime::from_hms_opt(12, 0, 0).unwrap()));
    }
}
