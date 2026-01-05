use chrono::{Datelike, Duration};

#[derive(Clone, Debug)]
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
}