use crate::data_models::response_models::untis_response_models::deserialize_null_as_default;
use chrono::{NaiveDate, NaiveTime};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;

/// The classreg API writes dates as `20260915`
fn deserialize_untis_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let value = i32::deserialize(deserializer)?;
    untis_date(value).ok_or_else(|| serde::de::Error::custom(format!("Invalid date: {value}")))
}

/// ... and times as `1435`, or `750` for 07:50
fn deserialize_untis_time<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
where
    D: Deserializer<'de>,
{
    let value = i32::deserialize(deserializer)?;
    untis_time(value).ok_or_else(|| serde::de::Error::custom(format!("Invalid time: {value}")))
}

/// The meta endpoint leaves its defaults out or sends `0` when the school didn't set any
fn deserialize_optional_untis_date<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<i32>::deserialize(deserializer)?.and_then(untis_date))
}

fn deserialize_optional_untis_time<'de, D>(deserializer: D) -> Result<Option<NaiveTime>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<i32>::deserialize(deserializer)?.and_then(untis_time))
}

fn untis_date(value: i32) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(value / 10_000, (value / 100 % 100) as u32, (value % 100) as u32)
}

fn untis_time(value: i32) -> Option<NaiveTime> {
    NaiveTime::from_hms_opt((value / 100) as u32, (value % 100) as u32, 0)
}

/// `20260915`, the form the classreg API takes dates in
pub fn to_untis_date(date: NaiveDate) -> i32 {
    date.format("%Y%m%d").to_string().parse().unwrap_or(0)
}

/// `1435`, the form the classreg API takes times in
pub fn to_untis_time(time: NaiveTime) -> i32 {
    time.format("%H%M").to_string().parse().unwrap_or(0)
}

#[derive(Clone, Debug, Deserialize)]
pub struct AbsencesResponse {
    pub data: AbsencesData,
}

#[derive(Default, Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbsencesData {
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub absences: Vec<Absence>,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub absence_reasons: Vec<AbsenceReason>,
    /// whether the school lets students report an absence themselves
    #[serde(default)]
    pub show_create_absence: bool,
}

#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Absence {
    pub id: i32,
    #[serde(deserialize_with = "deserialize_untis_date")]
    pub start_date: NaiveDate,
    #[serde(deserialize_with = "deserialize_untis_date")]
    pub end_date: NaiveDate,
    #[serde(deserialize_with = "deserialize_untis_time")]
    pub start_time: NaiveTime,
    #[serde(deserialize_with = "deserialize_untis_time")]
    pub end_time: NaiveTime,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub reason: String,
    /// 0 when none of the school's reasons was picked
    #[serde(default)]
    pub reason_id: i32,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub text: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub created_user: String,
    /// only absences the student entered themselves, and only until a teacher touched them
    #[serde(default)]
    pub can_edit: bool,
    /// the school's own wording, empty as long as nobody decided yet
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub excuse_status: String,
    #[serde(default)]
    pub is_excused: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize)]
pub struct AbsenceReason {
    pub id: i32,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AbsenceMetaResponse {
    pub data: AbsenceMeta,
}

/// What the school allows, and the values its web client pre-fills the report form with
#[derive(Default, Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbsenceMeta {
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub absence_reasons: Vec<AbsenceReason>,
    #[serde(default)]
    pub can_report_absence: bool,
    #[serde(default)]
    pub can_delete: bool,
    pub default_absence_reason: Option<i32>,
    #[serde(default, deserialize_with = "deserialize_optional_untis_date")]
    pub default_date: Option<NaiveDate>,
    #[serde(default, deserialize_with = "deserialize_optional_untis_time")]
    pub default_start_time: Option<NaiveTime>,
    #[serde(default, deserialize_with = "deserialize_optional_untis_time")]
    pub default_end_time: Option<NaiveTime>,
    /// which of the school's excuse groups a student's excuse note is printed for
    pub excuse_group_for_students: Option<i32>,
}

/// A report Untis rendered and now keeps around for a moment under `report_params`
#[derive(Clone, Debug, Deserialize)]
pub struct ReportResponse {
    pub data: Report,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    #[serde(default)]
    pub error: bool,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub report_name: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub format: String,
    /// the query string the rendered file is fetched with
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub report_params: String,
}

/// Everything needed to fetch a rendered report, for a caller that downloads it itself
#[derive(Clone, PartialEq, Debug)]
pub struct ReportDownload {
    pub url: String,
    pub file_name: String,
    pub headers: HashMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateAbsenceResponse {
    pub data: CreateAbsenceData,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CreateAbsenceData {
    pub result: Absence,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SuccessResponse {
    pub data: Success,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Success {
    #[serde(default)]
    pub success: bool,
}

/// How the classreg API reports a refusal: it answers `{"errors": [...]}` with the reason in
/// `title`, rather than the `errorMessage` the timetable API uses
#[derive(Clone, Debug, Deserialize)]
pub struct ErrorResponse {
    pub errors: Vec<ErrorEntry>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ErrorEntry {
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub title: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub details: String,
}

impl ErrorResponse {
    pub fn message(&self) -> String {
        self.errors
            .iter()
            .map(|error| match (error.title.as_str(), error.details.as_str()) {
                (title, "") => title.to_string(),
                ("", details) => details.to_string(),
                (title, details) => format!("{title}: {details}"),
            })
            .filter(|message| !message.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// An absence a student reports for themselves
#[derive(Clone, PartialEq, Debug)]
pub struct NewAbsence {
    pub start_date: NaiveDate,
    pub start_time: NaiveTime,
    pub end_date: NaiveDate,
    pub end_time: NaiveTime,
    /// none of the school's reasons picked
    pub reason_id: Option<i32>,
    pub text: String,
}
