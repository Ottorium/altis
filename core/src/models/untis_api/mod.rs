//! The timetable API's responses as Untis sends them, reduced to the fields the app reads.
//! `conversions` turns them into the app's own [`crate::models::timetable`] types.

mod conversions;
pub mod timetable;

use crate::errors::ApiError;
use crate::models::deserialize_null_as_default;
use serde::{Deserialize, Deserializer};
use timetable::UntisDayEntry;

/// Untis writes its error codes as a number in some responses and as text in others
fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match Option::<serde_json::Value>::deserialize(deserializer)? {
        Some(serde_json::Value::String(text)) => Some(text),
        Some(serde_json::Value::Null) | None => None,
        Some(other) => Some(other.to_string()),
    })
}

/// What both the `filter` and the `entries` endpoint answer with, each filling in its own part
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisResponse {
    pub days: Option<Vec<UntisDayEntry>>,
    pub pre_selected: Option<UntisPreSelected>,
    pub classes: Option<Vec<UntisClassEntry>>,
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

impl UntisError {
    /// An error if the response carries a message or a code, which a successful one doesn't
    pub fn into_result(self) -> Result<(), ApiError> {
        let message = self.error_message.filter(|message| !message.is_empty());
        let code = self.error_code.filter(|code| !code.is_empty());

        let details = match (message, code) {
            (Some(message), Some(code)) => format!("{message} ({code})"),
            (Some(message), None) => message,
            (None, Some(code)) => code,
            (None, None) => return Ok(()),
        };
        Err(ApiError::Miscellaneous(format!("Error in response from Untis: {details}")))
    }
}

/// The entity a `filter` request is answered for: the student's own class, or the student
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisPreSelected {
    pub id: i32,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub short_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub display_name: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisClassEntry {
    #[serde(rename = "class")]
    pub class_info: UntisClassInfo,
    pub class_teacher1: Option<UntisTeacher>,
    #[serde(default)]
    pub department: Option<UntisDepartment>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisClassInfo {
    pub id: i32,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub short_name: String,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn error(code: Option<&str>, message: Option<&str>) -> Result<(), String> {
        UntisError { error_code: code.map(String::from), error_message: message.map(String::from) }
            .into_result()
            .map_err(|error| error.to_string())
    }

    #[test]
    fn a_response_without_error_fields_is_no_error() {
        assert_eq!(error(None, None), Ok(()));
        assert_eq!(error(Some(""), Some("")), Ok(()));
    }

    #[test]
    fn reports_the_message_and_the_code_it_has() {
        assert_eq!(error(Some("42"), Some("Nope")), Err("Error in response from Untis: Nope (42)".to_string()));
        assert_eq!(error(None, Some("Nope")), Err("Error in response from Untis: Nope".to_string()));
        assert_eq!(error(Some("42"), None), Err("Error in response from Untis: 42".to_string()));
    }

    #[test]
    fn reads_a_numeric_error_code() {
        let error: UntisError = serde_json::from_str(r#"{"errorCode": 403, "errorMessage": null}"#).unwrap();
        assert_eq!(error.error_code.as_deref(), Some("403"));
    }
}
