//! What sits in the bar above the list of absences.

use crate::native;
use crate::untis::UntisClient;
use altis_core::models::absences::AbsencesData;
use altis_core::untis::week::school_year;
use chrono::{Datelike, NaiveDate};
use wasm_bindgen_futures::spawn_local;
use web_sys::MouseEvent;
use yew::prelude::*;

/// How many absences there are, and how many of them still need an excuse
pub fn render_summary(data: Option<&AbsencesData>) -> Html {
    let Some(data) = data else { return html! {} };
    let open = data.absences.iter().filter(|absence| !absence.is_excused).count();

    html! {
        <>
            <span class="badge rounded-pill bg-secondary text-dark">{ data.absences.len() }</span>
            if open > 0 {
                <span class="badge rounded-pill bg-warning text-dark" title="Not excused">
                    { format!("{open} open") }
                </span>
            }
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct SchoolYearPickerProps {
    /// how many years back from the running one the shown school year is
    pub offset: i32,
    pub on_change: Callback<i32>,
}

/// Steps through the school years, back from the running one
#[function_component(SchoolYearPicker)]
pub fn school_year_picker(props: &SchoolYearPickerProps) -> Html {
    let (start, end) = school_year(props.offset);
    let label = format!("{}/{}", start.year(), end.format("%y"));

    let step = |by: i32| {
        let offset = props.offset + by;
        props.on_change.reform(move |_: MouseEvent| offset)
    };

    html! {
        <div class="btn-group btn-group-sm" role="group">
            <button class="btn btn-outline-secondary" title="Previous school year" onclick={step(1)}>
                <i class="bi bi-chevron-left"></i>
            </button>
            <span class="btn btn-outline-secondary disabled text-white">{ label }</span>
            <button class="btn btn-outline-secondary" title="Next school year"
                    disabled={props.offset <= 0} onclick={step(-1)}>
                <i class="bi bi-chevron-right"></i>
            </button>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct ExcuseNoteButtonProps {
    pub year_offset: i32,
    pub excuse_group: Option<i32>,
    pub on_error: Callback<String>,
}

/// Downloads the excuse note of the shown school year, the PDF the school wants signed
#[function_component(ExcuseNoteButton)]
pub fn excuse_note_button(props: &ExcuseNoteButtonProps) -> Html {
    let downloading = use_state(|| false);

    let onclick = {
        let (year_offset, excuse_group) = (props.year_offset, props.excuse_group);
        let (downloading, on_error) = (downloading.clone(), props.on_error.clone());
        Callback::from(move |_: MouseEvent| {
            downloading.set(true);
            let (downloading, on_error) = (downloading.clone(), on_error.clone());
            spawn_local(async move {
                let (start, end) = school_year(year_offset);
                if let Err(message) = download_excuse_note(start, end, excuse_group).await {
                    on_error.emit(message);
                }
                downloading.set(false);
            });
        })
    };

    html! {
        <button class="btn btn-outline-primary btn-sm" title="Excuse note" disabled={*downloading} {onclick}>
            if *downloading {
                <span class="spinner-border spinner-border-sm" role="status"></span>
            } else {
                <i class="bi bi-file-earmark-pdf"></i>
            }
        </button>
    }
}

async fn download_excuse_note(start: NaiveDate, end: NaiveDate, excuse_group: Option<i32>) -> Result<(), String> {
    let client = UntisClient::new().map_err(|e| e.to_string())?;
    let report = client.get_excuse_report(start, end, excuse_group).await.map_err(|e| e.to_string())?;
    native::download_file(&report.url, &report.headers, &report.file_name).await.map(|_| ())
}
