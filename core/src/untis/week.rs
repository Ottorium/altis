use chrono::{Datelike, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A week of the timetable, Monday to Sunday
#[derive(Clone, PartialEq, Debug, Hash, Eq, Serialize, Deserialize)]
pub struct Week {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

/// The school year's first and last day. Untis starts one in September and groups the things it
/// keeps per year, absences among them, by it. `offset` counts years back from the running one.
pub fn school_year(offset: i32) -> (NaiveDate, NaiveDate) {
    let today = Local::now().date_naive();
    let first_year = if today.month() >= 9 { today.year() } else { today.year() - 1 } - offset;

    (
        NaiveDate::from_ymd_opt(first_year, 9, 1).unwrap_or(today),
        NaiveDate::from_ymd_opt(first_year + 1, 8, 31).unwrap_or(today),
    )
}

impl Week {
    pub fn current() -> Self {
        Self::from_date(Local::now().date_naive())
    }

    /// The week the date is in
    pub fn from_date(date: NaiveDate) -> Self {
        let monday = date - Duration::days(i64::from(date.weekday().num_days_from_monday()));
        Self { start: monday, end: monday + Duration::days(6) }
    }

    pub fn previous(&self) -> Self {
        Self::from_date(self.start - Duration::weeks(1))
    }

    pub fn next(&self) -> Self {
        Self::from_date(self.start + Duration::weeks(1))
    }

    /// The seven days of the week, in order
    pub fn days(&self) -> impl Iterator<Item = NaiveDate> + use<> {
        self.start.iter_days().take(7)
    }
}

/// The week as the timetable's date picker shows it, `14.9 - 20.9`
impl fmt::Display for Week {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{} - {}.{}", self.start.day(), self.start.month(), self.end.day(), self.end.month())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn a_week_runs_from_monday_to_sunday() {
        // a Wednesday, a Monday and a Sunday of the same week
        for day in [16, 14, 20] {
            let week = Week::from_date(date(2026, 9, day));
            assert_eq!((week.start, week.end), (date(2026, 9, 14), date(2026, 9, 20)));
        }
    }

    #[test]
    fn steps_a_week_at_a_time_across_months() {
        let week = Week::from_date(date(2026, 9, 30));

        assert_eq!(week.next().start, date(2026, 10, 5));
        assert_eq!(week.previous().start, date(2026, 9, 21));
        assert_eq!(week.next().previous(), week);
    }

    #[test]
    fn is_written_as_its_first_and_last_day() {
        assert_eq!(Week::from_date(date(2026, 9, 30)).to_string(), "28.9 - 4.10");
    }

    #[test]
    fn has_seven_days() {
        let days: Vec<NaiveDate> = Week::from_date(date(2026, 9, 14)).days().collect();
        assert_eq!(days.len(), 7);
        assert_eq!((days[0], days[6]), (date(2026, 9, 14), date(2026, 9, 20)));
    }

    /// The timetable caches are keyed by week and were written when a week held its dates as
    /// `2026-09-14` strings. Dates are stored as exactly that, so those caches still read.
    #[test]
    fn is_stored_as_two_dates_in_text() {
        let week = Week::from_date(date(2026, 9, 14));
        assert_eq!(serde_json::to_string(&week).unwrap(), r#"{"start":"2026-09-14","end":"2026-09-20"}"#);
    }
}
