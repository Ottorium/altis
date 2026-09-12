use crate::data_models::response_models::untis_timetables::UntisDayEntry;
use serde::{Deserialize, Deserializer};

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        Some(serde_json::Value::String(s)) => Ok(Some(s)),
        Some(serde_json::Value::Number(n)) => Ok(Some(n.to_string())),
        Some(serde_json::Value::Null) | None => Ok(None),
        Some(other) => Ok(Some(other.to_string())),
    }
}

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
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub request_id: Option<String>,
}

/// The error fields of a response, independent of what the endpoint returns otherwise
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisError {
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub error_code: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub error_message: Option<String>,
}

pub fn deserialize_null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    let opt = Option::<T>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisPreSelected {
    pub id: i32,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub short_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub long_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
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
    #[serde(default)]
    pub department: Option<UntisDepartment>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisClassInfo {
    pub id: i32,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub short_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub long_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub display_name: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisTeacher {
    pub id: i32,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub short_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub long_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub display_name: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisDepartment {
    pub id: i32,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub short_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub long_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub display_name: String,
}
