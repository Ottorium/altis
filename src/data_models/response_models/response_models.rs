use serde::Deserialize;
use crate::data_models::response_models::timetable_stuff::UntisDayEntry;

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisResponse {
    pub format: Option<i32>,
    pub days: Option<Vec<UntisDayEntry>>,
    pub resource_type: Option<String>,
    pub pre_selected: Option<UntisPreSelected>,
    pub buildings: Option<Vec<serde_json::Value>>,
    pub departments: Option<Vec<UntisDepartment>>,
    pub room_groups: Option<Vec<serde_json::Value>>,
    pub resource_types: Option<Vec<serde_json::Value>>,
    pub assignment_groups: Option<Vec<serde_json::Value>>,
    pub classes: Option<Vec<UntisClassEntry>>,
    pub resources: Option<Vec<serde_json::Value>>,
    pub rooms: Option<Vec<serde_json::Value>>,
    pub subjects: Option<Vec<serde_json::Value>>,
    pub students: Option<Vec<serde_json::Value>>,
    pub teachers: Option<Vec<serde_json::Value>>,
    pub errors: Option<Vec<serde_json::Value>>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisPreSelected {
    pub id: i32,
    pub short_name: String,
    pub long_name: String,
    pub display_name: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisClassEntry {
    #[serde(rename = "class")]
    pub class_info: UntisClassInfo,
    pub class_teacher1: Option<UntisTeacher>,
    pub class_teacher2: Option<UntisTeacher>,
    pub department: UntisDepartment,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisClassInfo {
    pub id: i32,
    pub short_name: String,
    pub long_name: String,
    pub display_name: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisTeacher {
    pub id: i32,
    pub short_name: String,
    pub long_name: String,
    pub display_name: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisDepartment {
    pub id: i32,
    pub short_name: String,
    pub long_name: String,
    pub display_name: String,
}
