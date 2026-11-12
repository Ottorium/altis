use crate::models::deserialize_null_as_default;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisDayEntry {
    pub date: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub grid_entries: Vec<UntisGridEntry>,
}

/// One block in the timetable grid: a lesson, an exam, an event
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisGridEntry {
    pub duration: UntisDuration,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub r#type: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub status: String,
    pub status_detail: Option<String>,
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub color: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub notes_all: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub icons: Vec<String>,
    // which position holds the teachers, the subject or the rooms depends on the timetable's
    // format, so each resource says what it is itself
    pub position1: Option<Vec<UntisPosition>>,
    pub position2: Option<Vec<UntisPosition>>,
    pub position3: Option<Vec<UntisPosition>>,
    pub position4: Option<Vec<UntisPosition>>,
    pub position5: Option<Vec<UntisPosition>>,
    pub position6: Option<Vec<UntisPosition>>,
    pub position7: Option<Vec<UntisPosition>>,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub lesson_text: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub substitution_text: String,
    pub user_name: Option<String>,
    pub link: Option<String>,
}

/// When a grid entry takes place, as `2026-09-14T07:50`
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisDuration {
    pub start: String,
    pub end: String,
}

/// A teacher, subject, room, ... of a grid entry, and the one it replaced if it was changed
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisPosition {
    pub current: Option<UntisResource>,
    pub removed: Option<UntisResource>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UntisResource {
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub r#type: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub short_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub long_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub display_name: String,
    pub text: Option<String>,
}
