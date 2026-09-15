use crate::data_models::clean_models::untis::*;
use crate::data_models::response_models::untis_messages::*;
use crate::data_models::response_models::untis_response_models::*;
use crate::errors::ApiError;
use crate::env::Env;
use crate::store::Store;
use crate::untis::auth::AuthHelper;
use crate::untis::untis_week::Week;
use chrono::{Duration, NaiveDate};
use futures::future::join_all;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::marker::PhantomData;

pub struct UntisClient<E: Env> {
    school_name: String,
    _env: PhantomData<E>,
}

impl<E: Env> UntisClient<E> {
    pub fn new() -> Result<Self, ApiError> {
        let school_name = Store::<E>::get_settings()?
            .ok_or(ApiError::Authentication("Settings are empty".to_string()))?
            .untis_auth
            .school_identifier;

        Ok(Self { school_name, _env: PhantomData })
    }

    pub fn is_authenticated() -> bool {
        AuthHelper::<E>::is_authenticated()
    }

    pub async fn authenticate(school_name: String, username: String, secret: String) -> Result<(), ApiError> {
        AuthHelper::<E>::authenticate(school_name, username, secret).await
    }

    async fn get_classes(&self, week: Week) -> Result<(Vec<Class>, Option<i32>), ApiError> {
        let url = format!(
            "https://{}.webuntis.com/WebUntis/api/rest/view/v1/timetable/filter?resourceType=CLASS&timetableType=STANDARD&start={}&end={}",
            self.school_name,
            week.start,
            week.end,
        );

        let untis_data: UntisResponse = Self::fetch(&url).await?;

        let classes: Vec<Class> = untis_data
            .classes
            .unwrap_or_default()
            .into_iter()
            .map(Class::from)
            .collect();

        Ok((classes, untis_data.pre_selected.map(|x| x.id)))
    }

    fn check_untis_error(untis_error: &UntisError) -> Result<(), ApiError> {
        if let Some(msg) = &untis_error.error_message && !msg.is_empty() {
            if let Some(code) = &untis_error.error_code && !code.is_empty() {
                return Err(ApiError::Miscellaneous(format!("Error in response from Untis: {} ({})", msg, code)));
            }
            return Err(ApiError::Miscellaneous(format!("Error in response from Untis: {}", msg)));
        }
        if let Some(code) = &untis_error.error_code && !code.is_empty() {
            return Err(ApiError::Miscellaneous(format!("Error in response from Untis: {}", code)));
        }
        Ok(())
    }

    /// Performs an authorized GET request and parses the response, including Untis error checking
    async fn fetch<T: DeserializeOwned>(url: &str) -> Result<T, ApiError> {
        let response = AuthHelper::<E>::authorized_request("GET", url, HashMap::new(), "".to_string())
            .await?;

        // checked first, an error response doesn't parse as the expected type
        if let Ok(untis_error) = serde_json::from_str::<UntisError>(&response.body) {
            Self::check_untis_error(&untis_error)?;
        }

        serde_json::from_str(&response.body).map_err(|e| {
            let line = e.line();
            let col = e.column();
            let line_content = response.body.lines().nth(line.saturating_sub(1)).unwrap_or("");

            let start = col.saturating_sub(20);

            let snippet = if !line_content.is_empty() && col <= line_content.len() {
                let before = &line_content[start..col.saturating_sub(1)];
                let char = &line_content[col.saturating_sub(1)..col];
                let after = &line_content[col..];
                format!("{}[-->]{}[<--] {}", before, char, after)
            } else {
                line_content.to_string()
            };

            ApiError::Parsing(format!(
                "JSON Error: {} at line {} col {}.\nContext: {}",
                e, line, col, snippet
            ))
        })
    }

    /// Fetches timetable entries and fills in the days of the week that have no lessons
    async fn fetch_week(url: &str, week: &Week) -> Result<WeekTimeTable, ApiError> {
        let mut day_tables: Vec<DayTimeTable> = Self::fetch::<UntisResponse>(url)
            .await?
            .days
            .unwrap_or_default()
            .into_iter()
            .map(DayTimeTable::from)
            .collect();

        let start = NaiveDate::parse_from_str(&week.start, "%Y-%m-%d")
            .map_err(|error| ApiError::Miscellaneous(format!("Invalid week start date: {error}")))?;
        let end = NaiveDate::parse_from_str(&week.end, "%Y-%m-%d")
            .map_err(|error| ApiError::Miscellaneous(format!("Invalid week end date: {error}")))?;
        let mut date = start;
        while date <= end {
            if !day_tables.iter().any(|day| day.date == date) {
                day_tables.push(DayTimeTable { date, lessons: Vec::new() });
            }
            date += Duration::days(1);
        }
        day_tables.sort_by_key(|day| day.date);

        Ok(WeekTimeTable { days: day_tables })
    }

    pub async fn get_timetable(&self, week: Week, class: Class) -> Result<WeekTimeTable, ApiError> {
        let url = format!(
            "https://{}.webuntis.com/WebUntis/api/rest/view/v1/timetable/entries?start={}&end={}&format=1&resourceType=CLASS&resources={}&periodTypes=&timetableType=STANDARD&",
            self.school_name,
            week.start,
            week.end,
            class.id,
        );

        let mut timetable = Self::fetch_week(&url, &week).await?;
        for lesson in timetable.days.iter_mut().flat_map(|day| &mut day.lessons) {
            lesson.entities.push(Tracked {
                inner: Entity::Class(class.clone()),
                status: ChangeStatus::Regular,
            });
        }

        Ok(timetable)
    }

    async fn get_me(&self, week: &Week) -> Result<Option<UntisPreSelected>, ApiError> {
        let url = format!(
            "https://{}.webuntis.com/WebUntis/api/rest/view/v1/timetable/filter?resourceType=STUDENT&timetableType=MY_TIMETABLE&start={}&end={}",
            self.school_name,
            week.start,
            week.end,
        );

        Ok(Self::fetch::<UntisResponse>(&url).await?.pre_selected)
    }

    /// Gets the personal timetable of the logged in student, which only contains their own lessons
    pub async fn get_my_timetable(&self, week: Week) -> Result<MyTimeTable, ApiError> {
        let me = match self.get_me(&week).await {
            Ok(Some(me)) => me,
            _ => self.get_me(&Week::current()).await?
                .ok_or(ApiError::Miscellaneous("Untis did not return a personal timetable for this account".to_string()))?,
        };

        let url = format!(
            "https://{}.webuntis.com/WebUntis/api/rest/view/v1/timetable/entries?start={}&end={}&format=3&resourceType=STUDENT&resources={}&periodTypes=&timetableType=MY_TIMETABLE&layout=START_TIME",
            self.school_name,
            week.start,
            week.end,
            me.id,
        );

        Ok(MyTimeTable {
            name: if me.display_name.is_empty() { me.short_name } else { me.display_name },
            timetable: Self::fetch_week(&url, &week).await?,
        })
    }

    async fn get_multiple_timetables(&self, week: Week, classes: &[Class]) -> Result<HashMap<Class, WeekTimeTable>, ApiError> {
        let tasks = classes.iter().map(|class| {
            let week_clone = week.clone();
            let class_clone = class.clone();
            async move {
                let result = self.get_timetable(week_clone, class_clone.clone()).await;
                (class_clone, result)
            }
        });

        let results = join_all(tasks).await;

        let mut map = HashMap::new();
        for (class, result) in results {
            match result {
                Ok(timetable) => { map.insert(class, timetable); }
                Err(e) => return Err(ApiError::Miscellaneous(format!("Could not get timetable for class {}: {}", class.id, e))),
            }
        }

        Ok(map)
    }

    pub async fn get_all_class_timetables(
        &self,
        week: Week,
    ) -> Result<(HashMap<Class, WeekTimeTable>, Option<i32>), ApiError> {
        let (classes, pre_selected) = match self.get_classes(week.clone()).await {
            Ok((classes, pre_selected)) if !classes.is_empty() => (classes, pre_selected),
            _ => self.get_classes(Week::current()).await?,
        };
        let class_results = self.get_multiple_timetables(week.clone(), &classes).await?;
        Ok((class_results, pre_selected))
    }

    /// The messages in the inbox, newest first
    pub async fn get_messages(&self) -> Result<Vec<MessagePreview>, ApiError> {
        let url = format!("https://{}.webuntis.com/WebUntis/api/rest/view/v1/messages", self.school_name);
        Ok(Self::fetch::<MessageList>(&url).await?.incoming_messages)
    }

    /// WebUntis doesn't send a separate request to mark a message as read, loading it does that
    pub async fn get_message(&self, id: i32) -> Result<Message, ApiError> {
        let url = format!("https://{}.webuntis.com/WebUntis/api/rest/view/v1/messages/{}", self.school_name, id);
        Self::fetch(&url).await
    }

    pub async fn get_attachment_download(&self, attachment_id: &str) -> Result<AttachmentDownload, ApiError> {
        let url = format!(
            "https://{}.webuntis.com/WebUntis/api/rest/view/v1/messages/{}/attachmentstorageurl",
            self.school_name,
            attachment_id,
        );
        Self::fetch(&url).await
    }
}
