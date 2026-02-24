use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, PartialEq, Debug, Hash, Eq, Serialize, Deserialize)]
pub struct Week {
    pub start: String,
    pub end: String,
}

impl Week {
    pub fn current() -> Self {
        Self::current_plus(0)
    }

    pub fn current_plus(offset: i32) -> Self {
        let today = chrono::Local::now().date_naive();
        let days_from_monday = today.weekday().num_days_from_monday();
        let current_monday = today - Duration::days(days_from_monday as i64);

        let target_monday = current_monday + Duration::weeks(offset as i64);
        let target_friday = target_monday + Duration::days(4);

        Week {
            start: target_monday.format("%Y-%m-%d").to_string(),
            end: target_friday.format("%Y-%m-%d").to_string(),
        }
    }

    pub fn from_date(date: NaiveDate) -> Self {
        let days_from_monday = date.weekday().num_days_from_monday();
        let monday = date - Duration::days(days_from_monday as i64);
        let friday = monday + Duration::days(4);

        Week {
            start: monday.format("%Y-%m-%d").to_string(),
            end: friday.format("%Y-%m-%d").to_string(),
        }
    }

    pub fn previous(&self) -> Self {
        self.shift_weeks(-1)
    }

    pub fn next(&self) -> Self {
        self.shift_weeks(1)
    }

    fn shift_weeks(&self, weeks: i64) -> Self {
        let current_start = NaiveDate::parse_from_str(&self.start, "%Y-%m-%d")
            .unwrap_or_else(|_| chrono::Local::now().date_naive());

        let new_monday = current_start + Duration::weeks(weeks);
        let new_friday = new_monday + Duration::days(4);

        Week {
            start: new_monday.format("%Y-%m-%d").to_string(),
            end: new_friday.format("%Y-%m-%d").to_string(),
        }
    }

    pub fn to_string(&self) -> String {
        let s = NaiveDate::parse_from_str(&self.start, "%Y-%m-%d").unwrap();
        let e = NaiveDate::parse_from_str(&self.end, "%Y-%m-%d").unwrap();
        format!("{}.{} - {}.{}", s.day(), s.month(), e.day(), e.month())
    }
}