use crate::components::forms::{input_value, select_value};
use crate::components::modal::Modal;
use crate::untis::UntisClient;
use altis_core::models::absences::{Absence, AbsenceMeta, AbsenceReason, NewAbsence};
use chrono::{Local, NaiveDate, NaiveTime};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlTextAreaElement, MouseEvent};
use yew::prelude::*;

const DATE_INPUT_FORMAT: &str = "%Y-%m-%d";
const TIME_INPUT_FORMAT: &str = "%H:%M";

/// When an absence starts and ends where neither the absence nor the school says: the school day
const FALLBACK_START: (u32, u32) = (8, 0);
const FALLBACK_END: (u32, u32) = (17, 0);

#[derive(Properties, PartialEq)]
pub struct AbsenceFormModalProps {
    pub reasons: Vec<AbsenceReason>,
    pub meta: AbsenceMeta,
    /// the absence being changed, none to report a new one
    #[prop_or_default]
    pub editing: Option<Absence>,
    pub on_close: Callback<()>,
    pub on_saved: Callback<()>,
    pub on_error: Callback<String>,
}

/// The form an absence is reported or changed in
#[function_component(AbsenceFormModal)]
pub fn absence_form_modal(props: &AbsenceFormModalProps) -> Html {
    let today = Local::now().date_naive();
    let (meta, editing) = (&props.meta, props.editing.as_ref());

    // an absence being changed fills the form, a new one starts from the school's defaults
    let date = use_state(|| date_value(editing.map(|absence| absence.start_date).or(meta.default_date), today));
    let end_date = use_state(|| date_value(editing.map(|absence| absence.end_date).or(meta.default_date), today));
    let start_time = use_state(|| time_value(editing.map(|absence| absence.start_time).or(meta.default_start_time), FALLBACK_START));
    let end_time = use_state(|| time_value(editing.map(|absence| absence.end_time).or(meta.default_end_time), FALLBACK_END));
    let reason_id = use_state(|| match editing {
        Some(absence) => Some(absence.reason_id).filter(|id| *id > 0),
        None => meta.default_absence_reason,
    });
    let text = use_state(|| editing.map(|absence| absence.text.clone()).unwrap_or_default());
    let saving = use_state(|| false);
    let error = use_state(|| None::<String>);

    let on_submit = {
        let (date, end_date, start_time, end_time) = (date.clone(), end_date.clone(), start_time.clone(), end_time.clone());
        let (reason_id, text, saving, error) = (reason_id.clone(), text.clone(), saving.clone(), error.clone());
        let (on_saved, on_error) = (props.on_saved.clone(), props.on_error.clone());
        let edited_id = editing.map(|absence| absence.id);
        Callback::from(move |_: MouseEvent| {
            let absence = match build_absence(&date, &end_date, &start_time, &end_time, *reason_id, &text) {
                Ok(absence) => absence,
                Err(message) => return error.set(Some(message)),
            };

            error.set(None);
            saving.set(true);
            let (saving, on_saved, on_error) = (saving.clone(), on_saved.clone(), on_error.clone());
            spawn_local(async move {
                match save_absence(edited_id, &absence).await {
                    Ok(()) => on_saved.emit(()),
                    Err(message) => on_error.emit(message),
                }
                saving.set(false);
            });
        })
    };

    // `onchange`, not `oninput`: a date or time input reports a half-typed value as empty, and
    // re-rendering the field with that empty value wipes what is being typed
    let on_change = |state: &UseStateHandle<String>| {
        let state = state.clone();
        Callback::from(move |e: Event| state.set(input_value(&e)))
    };
    let on_reason_change = {
        let reason_id = reason_id.clone();
        Callback::from(move |e: Event| reason_id.set(select_value(&e).parse().ok()))
    };
    let on_text_input = {
        let text = text.clone();
        Callback::from(move |e: InputEvent| text.set(e.target_unchecked_into::<HtmlTextAreaElement>().value()))
    };

    html! {
        <Modal title={if editing.is_some() { "Edit absence" } else { "Report absence" }} on_close={props.on_close.clone()} body_class="p-4">
            <div class="row g-3">
                <div class="col-12 col-md-6">
                    <label class="form-label small text-secondary">{"From"}</label>
                    <div class="d-flex gap-2">
                        <input type="date" class="form-control bg-dark text-white border-secondary"
                               value={(*date).clone()} onchange={on_change(&date)} />
                        <input type="time" class="form-control bg-dark text-white border-secondary"
                               value={(*start_time).clone()} onchange={on_change(&start_time)} />
                    </div>
                </div>
                <div class="col-12 col-md-6">
                    <label class="form-label small text-secondary">{"To"}</label>
                    <div class="d-flex gap-2">
                        <input type="date" class="form-control bg-dark text-white border-secondary"
                               value={(*end_date).clone()} onchange={on_change(&end_date)} />
                        <input type="time" class="form-control bg-dark text-white border-secondary"
                               value={(*end_time).clone()} onchange={on_change(&end_time)} />
                    </div>
                </div>
                if !props.reasons.is_empty() {
                    <div class="col-12">
                        <label class="form-label small text-secondary">{"Reason"}</label>
                        <select class="form-select bg-dark text-white border-secondary" onchange={on_reason_change}>
                            <option value="" selected={reason_id.is_none()}>{"No reason"}</option>
                            { for props.reasons.iter().map(|reason| html! {
                                <option value={reason.id.to_string()} selected={*reason_id == Some(reason.id)}>
                                    { &reason.name }
                                </option>
                            }) }
                        </select>
                    </div>
                }
                <div class="col-12">
                    <label class="form-label small text-secondary">{"Note"}</label>
                    <textarea class="form-control bg-dark text-white border-secondary" rows="2"
                              value={(*text).clone()}
                              oninput={on_text_input} />
                </div>
            </div>

            if let Some(error) = &*error {
                <div class="alert alert-danger mt-3 mb-0 py-2">{ error }</div>
            }

            <div class="d-flex justify-content-end gap-2 mt-4">
                <button class="btn btn-outline-secondary" onclick={props.on_close.reform(|_| ())}>{"Cancel"}</button>
                <button class="btn btn-primary text-dark fw-semibold" disabled={*saving} onclick={on_submit}>
                    if *saving {
                        <span class="spinner-border spinner-border-sm me-2" role="status"></span>
                    }
                    { if editing.is_some() { "Save" } else { "Report" } }
                </button>
            </div>
        </Modal>
    }
}

/// The `YYYY-MM-DD` an `<input type="date">` takes, falling back to the given day
fn date_value(date: Option<NaiveDate>, fallback: NaiveDate) -> String {
    date.unwrap_or(fallback).format(DATE_INPUT_FORMAT).to_string()
}

/// The `HH:MM` an `<input type="time">` takes, falling back to the given hour and minute
fn time_value(time: Option<NaiveTime>, (hour, minute): (u32, u32)) -> String {
    time.or_else(|| NaiveTime::from_hms_opt(hour, minute, 0)).map(|time| time.format(TIME_INPUT_FORMAT).to_string()).unwrap_or_default()
}

/// Turns what the form's fields hold into an absence, or says what is wrong with it
fn build_absence(start_date: &str, end_date: &str, start_time: &str, end_time: &str, reason_id: Option<i32>, text: &str) -> Result<NewAbsence, String> {
    let parse_date = |value: &str| NaiveDate::parse_from_str(value, DATE_INPUT_FORMAT).map_err(|_| "Pick a start and an end date".to_string());
    let parse_time = |value: &str| NaiveTime::parse_from_str(value, TIME_INPUT_FORMAT).map_err(|_| "Pick a start and an end time".to_string());

    let absence = NewAbsence {
        start_date: parse_date(start_date)?,
        start_time: parse_time(start_time)?,
        end_date: parse_date(end_date)?,
        end_time: parse_time(end_time)?,
        reason_id,
        text: text.to_string(),
    };

    if (absence.end_date, absence.end_time) <= (absence.start_date, absence.start_time) {
        return Err("The absence has to end after it starts".to_string());
    }

    Ok(absence)
}

/// Saves the form, as a change to `edited_id` or as a new absence
async fn save_absence(edited_id: Option<i32>, absence: &NewAbsence) -> Result<(), String> {
    let client = UntisClient::new().map_err(|e| e.to_string())?;
    match edited_id {
        Some(id) => client.edit_absence(id, absence).await,
        None => client.create_absence(absence).await,
    }
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_absence_the_fields_describe() {
        let absence = build_absence("2026-09-15", "2026-09-16", "07:50", "13:35", Some(3), "ill").unwrap();

        assert_eq!(absence.start_date, NaiveDate::from_ymd_opt(2026, 9, 15).unwrap());
        assert_eq!(absence.end_time, NaiveTime::from_hms_opt(13, 35, 0).unwrap());
        assert_eq!((absence.reason_id, absence.text.as_str()), (Some(3), "ill"));
    }

    #[test]
    fn says_what_is_missing_or_wrong() {
        assert_eq!(build_absence("", "2026-09-16", "07:50", "13:35", None, "").unwrap_err(), "Pick a start and an end date");
        assert_eq!(build_absence("2026-09-15", "2026-09-15", "07:50", "", None, "").unwrap_err(), "Pick a start and an end time");
        assert_eq!(build_absence("2026-09-15", "2026-09-15", "07:50", "07:50", None, "").unwrap_err(), "The absence has to end after it starts");
        // ending on a later day at an earlier time is still after the start
        assert!(build_absence("2026-09-15", "2026-09-16", "07:50", "07:00", None, "").is_ok());
    }

    #[test]
    fn the_form_starts_from_what_it_is_given_or_the_fallback() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 15).unwrap();
        assert_eq!(date_value(None, today), "2026-09-15");
        assert_eq!(date_value(NaiveDate::from_ymd_opt(2026, 1, 2), today), "2026-01-02");
        assert_eq!(time_value(None, FALLBACK_START), "08:00");
        assert_eq!(time_value(NaiveTime::from_hms_opt(7, 50, 0), FALLBACK_END), "07:50");
    }
}
