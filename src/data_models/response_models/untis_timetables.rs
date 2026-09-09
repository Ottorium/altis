use crate::data_models::response_models::untis_response_models::{deserialize_null_as_default, UntisClassInfo};
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisDayEntry {
    pub date: String,
    pub resource_type: String,
    pub resource: UntisClassInfo,
    pub status: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub day_entries: Vec<serde_json::Value>,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub grid_entries: Vec<UntisGridEntry>,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub back_entries: Vec<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisGridEntry {
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub ids: Vec<i64>,
    pub duration: UntisDuration,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub r#type: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub status: String,
    pub status_detail: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub layout_start_position: i32,
    #[serde(default)]
    pub layout_width: i32,
    #[serde(default)]
    pub layout_group: i32,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub color: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub notes_all: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub icons: Vec<String>,
    pub position1: Option<Vec<UntisPosition>>,
    pub position2: Option<Vec<UntisPosition>>,
    pub position3: Option<Vec<UntisPosition>>,
    pub position4: Option<Vec<UntisPosition>>,
    pub position5: Option<Vec<UntisPosition>>,
    pub position6: Option<Vec<UntisPosition>>,
    pub position7: Option<Vec<UntisPosition>>,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub lesson_text: String,
    pub lesson_info: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub substitution_text: String,
    pub user_name: Option<String>,
    #[serde(default)]
    pub moved: serde_json::Value,
    #[serde(default)]
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
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub r#type: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub status: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub short_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub long_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub display_name: String,
    pub display_name_label: Option<String>,
    pub text: Option<String>,
}
