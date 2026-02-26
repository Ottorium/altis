use crate::data_models::clean_models::untis::*;
use crate::data_models::response_models::untis_response_models::*;
use crate::errors::ApiError;
use crate::persistence_manager::PersistenceManager;
use crate::untis::auth::AuthHelper;
use crate::untis::untis_week::Week;
use futures::future::join_all;
use std::collections::HashMap;

pub struct UntisClient {
    school_name: String,
}

impl UntisClient {
    pub fn new() -> Result<Self, ApiError> {
        let school_name = PersistenceManager::get_settings()?
            .ok_or(ApiError::Authentication("Settings are empty".to_string()))?
            .untis_auth
            .school_identifier;

        Ok(Self { school_name })
    }

    pub fn is_authenticated() -> bool {
        AuthHelper::is_authenticated()
    }

    pub async fn authenticate(school_name: String, username: String, secret: String) -> Result<(), ApiError> {
        AuthHelper::authenticate(school_name, username, secret).await
    }

    async fn get_classes(&self, week: Week) -> Result<(Vec<Class>, Option<i32>), ApiError> {
        let url = format!(
            "https://{}.webuntis.com/WebUntis/api/rest/view/v1/timetable/filter?resourceType=CLASS&timetableType=STANDARD&start={}&end={}",
            self.school_name,
            week.start,
            week.end,
        );

        let response = AuthHelper::authorized_request("GET", url.as_str(), HashMap::new(), "".to_string()).await?;
        let untis_data: UntisResponse =
            serde_json::from_str(&response.body).map_err(|e| ApiError::Parsing(format!("Serialization error: {}", e)))?;

        let classes: Vec<Class> = untis_data
            .classes
            .unwrap_or_default()
            .into_iter()
            .map(Class::from)
            .collect();

        Ok((classes, untis_data.pre_selected.map(|x| x.id)))
    }

    pub async fn get_timetable(&self, week: Week, class: Class) -> Result<WeekTimeTable, ApiError> {
        let url = format!(
            "https://{}.webuntis.com/WebUntis/api/rest/view/v1/timetable/entries?start={}&end={}&format=1&resourceType=CLASS&resources={}&periodTypes=&timetableType=STANDARD&",
            self.school_name,
            week.start,
            week.end,
            class.id,
        );

        let response = AuthHelper::authorized_request("GET", url.as_str(), HashMap::new(), "".to_string())
            .await?;

        let untis_data: UntisResponse = serde_json::from_str(&response.body).map_err(|e| {
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
        })?;

        if let Some(error) = untis_data.error_message && !error.is_empty() {
            return Err(ApiError::Miscellaneous(format!("Error in response from Untis: {:#?}", error)));
        }

        let day_tables = untis_data
            .days
            .unwrap_or_default()
            .into_iter()
            .map(|day| {
                let mut day_table = DayTimeTable::from(day);
                for lesson in &mut day_table.lessons {
                    lesson.entities.push(Tracked {
                        inner: Entity::Class(class.clone()),
                        status: ChangeStatus::Regular,
                    });
                }
                day_table
            })
            .collect();

        Ok(WeekTimeTable { days: day_tables })
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
        let (classes, pre_selected) = self.get_classes(Week::current()).await?;
        let class_results = self.get_multiple_timetables(week.clone(), &classes).await?;
        Ok((class_results, pre_selected))
    } 
}
