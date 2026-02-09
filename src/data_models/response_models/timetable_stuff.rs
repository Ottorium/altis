use crate::data_models::response_models::response_models::UntisClassInfo;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisDayEntry {
    pub date: String,
    pub resource_type: String,
    pub resource: UntisClassInfo,
    pub status: String,
    pub day_entries: Vec<serde_json::Value>,
    pub grid_entries: Vec<UntisGridEntry>,
    pub back_entries: Vec<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisGridEntry {
    pub ids: Vec<i64>,
    pub duration: UntisDuration,
    pub r#type: String,
    pub status: String,
    pub status_detail: Option<String>,
    pub name: Option<String>,
    pub layout_start_position: i32,
    pub layout_width: i32,
    pub layout_group: i32,
    pub color: String,
    pub notes_all: String,
    pub icons: Vec<String>,
    pub position1: Option<Vec<UntisPosition>>,
    pub position2: Option<Vec<UntisPosition>>,
    pub position3: Option<Vec<UntisPosition>>,
    pub position4: Option<Vec<UntisPosition>>,
    pub position5: Option<Vec<UntisPosition>>,
    pub position6: Option<Vec<UntisPosition>>,
    pub position7: Option<Vec<UntisPosition>>,
    pub lesson_text: String,
    pub lesson_info: Option<String>,
    pub substitution_text: String,
    pub user_name: Option<String>,
    pub moved: serde_json::Value,
    pub duration_total: serde_json::Value,
    pub link: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisDuration {
    pub start: String,
    pub end: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisPosition {
    pub current: Option<UntisResource>,
    pub removed: Option<UntisResource>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisResource {
    pub r#type: String,
    pub status: String,
    pub short_name: String,
    pub long_name: String,
    pub display_name: String,
    pub display_name_label: Option<String>,
    pub text: Option<String>,
}
