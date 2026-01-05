use crate::authorization_untis_client::authorized_request;
use crate::data_models::clean_models::clean_models::*;
use crate::data_models::response_models::response_models::*;
use crate::persistence_manager::PersistenceManager;
use crate::untis_week::Week;
use futures::future::join_all;
use std::collections::HashMap;

pub fn school_name() -> Result<String, String> {
    Ok(PersistenceManager::get_settings()?.school_name)
}

pub async fn get_classes(week: Week) -> Result<(Vec<Class>, Option<i32>), String> {
    let url = format!(
        "https://{}.webuntis.com/WebUntis/api/rest/view/v1/timetable/filter?resourceType=CLASS&timetableType=STANDARD&start={}&end={}",
        school_name()?,
        week.start,
        week.end,
    );

    let response = authorized_request("GET", url.as_str(), HashMap::new(), "".to_string()).await?;
    let untis_data: UntisResponse =
        serde_json::from_str(&response.body).map_err(|e| format!("Serialization error: {}", e))?;

    let classes = untis_data
        .classes
        .unwrap_or_default()
        .into_iter()
        .map(Class::from)
        .collect();

    Ok((classes, untis_data.pre_selected.map(|x| x.id)))
}

pub async fn get_timetable(week: Week, class: Class) -> Result<WeekTimeTable, String> {
    let url = format!(
        "https://{}.webuntis.com/WebUntis/api/rest/view/v1/timetable/entries?start={}&end={}&format=1&resourceType=CLASS&resources={}&periodTypes=&timetableType=STANDARD&",
        school_name()?,
        week.start,
        week.end,
        class.id,
    );

    let response = authorized_request("GET", url.as_str(), HashMap::new(), "".to_string()).await?;

    let untis_data: UntisResponse =
        serde_json::from_str(&response.body).map_err(|e| {
            format!("Serialization error: {}", e)
        })?;

    if let Some(errors) = untis_data.errors && errors.len() > 0 {
        return Err(format!("Error in response from Untis: {:#?}", errors).into());
    }

    let day_tables = untis_data
        .days
        .unwrap_or_default()
        .into_iter()
        .map(|day| {
            let mut day_table = DayTimeTable::from(day);
            for lesson in &mut day_table.lessons {
                lesson.entities.push(Tracked {
                    data: Entity::Class(class.clone()),
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
    classes: &Vec<Class>,
) -> Result<HashMap<Class, WeekTimeTable>, String> {
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
            Err(e) => return Err(format!("Could not get timetable for class {}: {}", class.id, e)),
        }
    }

    Ok(map)
}