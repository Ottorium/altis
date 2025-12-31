use crate::authorization_untis_client::authorized_request;
use crate::data_models::clean_models::clean_models::*;
use crate::data_models::response_models::response_models::*;
use crate::persistence_manager::PersistenceManager;
use crate::untis_week::Week;
use std::collections::HashMap;

pub fn school_name() -> Result<String, String> {
    Ok(PersistenceManager::get_settings()?.school_name)
}

pub async fn get_classes(week: Week) -> Result<Vec<Class>, String> {
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
        .unwrap()
        .into_iter()
        .map(|entry| Class {
            id: entry.class_info.id,
            name: entry.class_info.short_name,
            class_teacher: {
                if entry.class_teacher1.is_some() {
                    Some(Teacher {
                        id: entry.class_teacher1.clone().unwrap().id,
                        short_name: entry.class_teacher1.clone().unwrap().short_name,
                        long_name: entry.class_teacher1.clone().unwrap().long_name,
                        display_name: entry.class_teacher1.clone().unwrap().long_name,
                    })
                } else {
                    None
                }
            },
            department: Department {
                id: entry.department.clone().id,
                short_name: entry.department.clone().short_name,
                long_name: entry.department.clone().long_name,
                display_name: entry.department.clone().display_name,
            },
        })
        .collect();

    Ok(classes)
}
