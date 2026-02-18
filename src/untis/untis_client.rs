use crate::untis::authorization_untis_client::authorized_request;
use crate::data_models::clean_models::untis::*;
use crate::data_models::response_models::untis_response_models::*;
use crate::errors::ApiError;
use crate::persistence_manager::PersistenceManager;
use crate::untis::untis_week::Week;
use futures::future::join_all;
use std::collections::HashMap;

pub fn school_name() -> Result<String, ApiError> {
    Ok(PersistenceManager::get_settings()?.ok_or(ApiError::Authentication("Settings are empty".to_string()))?.untis_auth.school_identifier)
}

pub async fn get_classes(week: Week) -> Result<(Vec<Class>, Option<i32>), ApiError> {
    let url = format!(
        "https://{}.webuntis.com/WebUntis/api/rest/view/v1/timetable/filter?resourceType=CLASS&timetableType=STANDARD&start={}&end={}",
        school_name()?,
        week.start,
        week.end,
    );

    let response = authorized_request("GET", url.as_str(), HashMap::new(), "".to_string()).await?;
    let untis_data: UntisResponse =
        serde_json::from_str(&response.body).map_err(|e| ApiError::Parsing(format!("Serialization error: {}", e)))?;

    let classes: Vec<Class> = untis_data
        .classes
        .unwrap_or_default()
        .into_iter()
        .map(Class::from)
        .collect();

    Ok((classes, untis_data.pre_selected.map(|x| x.id)))
    //Ok((vec![Class {id: 2419, name: "4BHITS".to_string(), class_teacher: None,department: Department { id: 45, short_name: "IT".to_string(), long_name: "INFORMATIONSTECHNOLOGIE".to_string(), display_name: "INFORMATIONSTECHNOLOGIE".to_string()}}], None))

}

pub async fn get_timetable(week: Week, class: Class) -> Result<WeekTimeTable, ApiError> {
    let url = format!(
        "https://{}.webuntis.com/WebUntis/api/rest/view/v1/timetable/entries?start={}&end={}&format=1&resourceType=CLASS&resources={}&periodTypes=&timetableType=STANDARD&",
        school_name()?,
        week.start,
        week.end,
        class.id,
    );

    let response = authorized_request("GET", url.as_str(), HashMap::new(), "".to_string()).await?;

    let untis_data: UntisResponse = serde_json::from_str(&response.body).map_err(|e| {
        let line = e.line();
        let col = e.column();
        let line_content = response.body.lines().nth(line.saturating_sub(1)).unwrap_or("");

        let start = col.saturating_sub(20);
        let end = (col + 20).min(line_content.len());

        let snippet = if !line_content.is_empty() && col <= line_content.len() {
            let before = &line_content[start..col.saturating_sub(1)];
            let char = &line_content[col.saturating_sub(1)..col];
            let after = &line_content[col..end];
            format!("{}[-->]{}[<--] {}", before, char, after)
        } else {
            line_content.to_string()
        };

        ApiError::Parsing(format!(
            "JSON Error: {} at line {} col {}.\nContext: {}",
            e, line, col, snippet
        ))
    })?;

    if let Some(errors) = untis_data.errors && !errors.is_empty() {
        return Err(ApiError::Miscellaneous(format!("Error in response from Untis: {:#?}", errors)));
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

pub async fn get_multiple_timetables(
    week: Week,
    classes: &[Class],
) -> Result<HashMap<Class, WeekTimeTable>, ApiError> {
    let tasks = classes.iter().map(|class| {
        let week_clone = week.clone();
        let class_clone = class.clone();
        async move {
            let result = get_timetable(week_clone, class_clone.clone()).await;
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