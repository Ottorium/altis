use crate::native;
use crate::untis::UntisClient;
use altis_core::data_models::response_models::untis_absences::{
    Absence, AbsenceMeta, AbsenceReason, AbsencesData, NewAbsence,
};
use altis_core::untis::untis_week::school_year;
use chrono::{Datelike, Local, NaiveDate, NaiveTime};
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement, MouseEvent};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::suspense::use_future_with;

/// The absence form that is open, if any
#[derive(Clone, PartialEq)]
enum AbsenceForm {
    New,
    Edit(Absence),
}

#[function_component(AbsencesComponent)]
pub fn absences() -> HtmlResult {
    let reload_trigger = use_state(|| 0u32);
    // 0 is the running school year, 1 the one before it
    let year_offset = use_state(|| 0);
    let form = use_state(|| None::<AbsenceForm>);
    let pending_delete = use_state(|| None::<Absence>);
    // of a delete or a report, the list itself reports its own errors
    let action_error = use_state(|| None::<String>);

    let res = use_future_with((*reload_trigger, *year_offset), |deps| {
        let (start, end) = school_year(deps.1);
        async move {
            let client = UntisClient::new()?;
            let absences = client.get_absences(start, end).await?;
            // only decides which buttons are offered, so a school that doesn't answer it still
            // gets a list
            let meta = client.get_absence_meta().await.ok();
            Ok::<_, altis_core::errors::ApiError>((absences, meta))
        }
    })?;

    let reload = {
        let trigger = reload_trigger.clone();
        Callback::from(move |_: ()| trigger.set(*trigger + 1))
    };

    let on_year_change = {
        let year_offset = year_offset.clone();
        Callback::from(move |offset: i32| year_offset.set(offset))
    };

    let on_delete = {
        let pending_delete = pending_delete.clone();
        let action_error = action_error.clone();
        Callback::from(move |absence: Absence| {
            action_error.set(None);
            pending_delete.set(Some(absence));
        })
    };

    let on_edit = {
        let form = form.clone();
        let action_error = action_error.clone();
        Callback::from(move |absence: Absence| {
            action_error.set(None);
            form.set(Some(AbsenceForm::Edit(absence)));
        })
    };

    let (data, meta) = match &*res {
        Ok((data, meta)) => (Some(data), meta.as_ref()),
        Err(_) => (None, None),
    };

    let can_report = data.is_some_and(|d| d.show_create_absence)
        && meta.is_none_or(|m| m.can_report_absence);

    let list = match &*res {
        Err(err) => html! { <div class="alert alert-danger m-3">{ err.to_string() }</div> },
        Ok((data, _)) if data.absences.is_empty() => html! {
            <div class="d-flex flex-column align-items-center justify-content-center text-secondary py-5">
                <i class="bi bi-calendar-check fs-1 mb-2"></i>
                <span>{"No absences this school year"}</span>
            </div>
        },
        Ok((data, meta)) => {
            // newest first, the way Untis' own list shows them
            let mut absences = data.absences.clone();
            absences.sort_by_key(|absence| (absence.start_date, absence.start_time));
            absences.reverse();

            let can_delete = meta.as_ref().is_none_or(|meta| meta.can_delete);
            html! {
                <div class="d-flex flex-column gap-2 p-3">
                    { for absences.into_iter().map(|absence| html! {
                        <AbsenceCard key={absence.id} absence={absence.clone()} deletable={can_delete}
                                     on_edit={on_edit.clone()} on_delete={on_delete.clone()} />
                    }) }
                </div>
            }
        }
    };

    Ok(html! {
        <div class="d-flex flex-column flex-grow-1 overflow-hidden">
            <div class="d-flex flex-wrap align-items-center justify-content-between gap-2 px-3 py-3 border-bottom border-secondary border-opacity-25 sticky-top"
                 style="background-color: #1e1e1e;">
                <div class="d-flex align-items-center gap-2">
                    <h5 class="mb-0 fw-bold">{"Absences"}</h5>
                    { render_summary(data) }
                </div>
                <div class="d-flex align-items-center gap-2">
                    <SchoolYearPicker offset={*year_offset} on_change={on_year_change} />
                    if data.is_some() {
                        <ExcuseNoteButton
                            year_offset={*year_offset}
                            excuse_group={meta.and_then(|meta| meta.excuse_group_for_students)}
                            on_error={
                                let action_error = action_error.clone();
                                Callback::from(move |message| action_error.set(Some(message)))
                            }
                        />
                    }
                    if can_report {
                        <button class="btn btn-primary btn-sm text-dark fw-semibold" onclick={
                            let form = form.clone();
                            let action_error = action_error.clone();
                            move |_| { action_error.set(None); form.set(Some(AbsenceForm::New)); }
                        }>
                            <i class="bi bi-plus-lg me-1"></i>{"Report"}
                        </button>
                    }
                    <button class="btn btn-outline-primary btn-sm" title="Reload" onclick={reload.reform(|_| ())}>
                        <i class="bi bi-arrow-clockwise"></i>
                    </button>
                </div>
            </div>

            if let Some(error) = &*action_error {
                <div class="alert alert-danger m-3 mb-0 py-2">{ error }</div>
            }

            <div class="flex-grow-1 overflow-y-auto">
                { list }
            </div>

            if let Some(open_form) = &*form {
                <AbsenceFormModal
                    reasons={meta.map(|m| m.absence_reasons.clone())
                        .or_else(|| data.map(|d| d.absence_reasons.clone()))
                        .unwrap_or_default()}
                    meta={meta.cloned().unwrap_or_default()}
                    editing={match open_form {
                        AbsenceForm::Edit(absence) => Some(absence.clone()),
                        AbsenceForm::New => None,
                    }}
                    on_close={
                        let form = form.clone();
                        Callback::from(move |_| form.set(None))
                    }
                    on_saved={
                        let (form, reload) = (form.clone(), reload.clone());
                        Callback::from(move |_| { form.set(None); reload.emit(()); })
                    }
                    on_error={
                        let action_error = action_error.clone();
                        Callback::from(move |message| action_error.set(Some(message)))
                    }
                />
            }

            if let Some(absence) = &*pending_delete {
                <DeleteAbsenceModal
                    absence={absence.clone()}
                    on_close={
                        let pending_delete = pending_delete.clone();
                        Callback::from(move |_| pending_delete.set(None))
                    }
                    on_deleted={
                        let (pending_delete, reload) = (pending_delete.clone(), reload.clone());
                        Callback::from(move |_| { pending_delete.set(None); reload.emit(()); })
                    }
                    on_error={
                        let (pending_delete, action_error) = (pending_delete.clone(), action_error.clone());
                        Callback::from(move |message| { pending_delete.set(None); action_error.set(Some(message)); })
                    }
                />
            }
        </div>
    })
}

/// How many absences there are, and how many of them still need an excuse
fn render_summary(data: Option<&AbsencesData>) -> Html {
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
struct SchoolYearPickerProps {
    offset: i32,
    on_change: Callback<i32>,
}

#[function_component(SchoolYearPicker)]
fn school_year_picker(props: &SchoolYearPickerProps) -> Html {
    let (start, end) = school_year(props.offset);
    let label = format!("{}/{}", start.year(), end.format("%y"));

    let step = |by: i32| {
        let (on_change, offset) = (props.on_change.clone(), props.offset);
        Callback::from(move |_: MouseEvent| on_change.emit(offset + by))
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
struct ExcuseNoteButtonProps {
    year_offset: i32,
    excuse_group: Option<i32>,
    on_error: Callback<String>,
}

/// Downloads the excuse note of the shown school year, the PDF the school wants signed
#[function_component(ExcuseNoteButton)]
fn excuse_note_button(props: &ExcuseNoteButtonProps) -> Html {
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

#[derive(Properties, PartialEq)]
struct AbsenceCardProps {
    absence: Absence,
    /// whether the school lets students withdraw absences at all
    deletable: bool,
    on_edit: Callback<Absence>,
    on_delete: Callback<Absence>,
}

#[function_component(AbsenceCard)]
fn absence_card(props: &AbsenceCardProps) -> Html {
    let absence = &props.absence;

    let (badge_class, badge_text) = match (absence.is_excused, absence.excuse_status.as_str()) {
        (true, "") => ("bg-success text-dark", "Excused".to_string()),
        (true, status) => ("bg-success text-dark", status.to_string()),
        (false, "") => ("bg-warning text-dark", "Open".to_string()),
        (false, status) => ("bg-danger", status.to_string()),
    };

    let date = match absence.start_date == absence.end_date {
        true => absence.start_date.format("%a, %d.%m.%Y").to_string(),
        false => format!(
            "{} - {}",
            absence.start_date.format("%a, %d.%m.%Y"),
            absence.end_date.format("%a, %d.%m.%Y"),
        ),
    };

    // the same absences a student may withdraw are the ones they may still change
    let emit_own = |callback: &Callback<Absence>| {
        let (callback, absence) = (callback.clone(), absence.clone());
        Callback::from(move |_: MouseEvent| callback.emit(absence.clone()))
    };

    html! {
        <div class="card border-0 shadow-sm" style="background-color: #1a1a1a;">
            // `.card` resets the text colour to the light theme's, so the card says its own
            <div class="card-body text-light d-flex align-items-start gap-3 py-3">
                <div class="flex-grow-1" style="min-width: 0;">
                    <div class="d-flex flex-wrap align-items-center gap-2 mb-1">
                        <span class="fw-semibold">{ date }</span>
                        <span class={classes!("badge", badge_class)}>{ badge_text }</span>
                    </div>
                    <div class="text-secondary small">
                        <i class="bi bi-clock me-1"></i>
                        { format!("{} - {}", absence.start_time.format("%H:%M"), absence.end_time.format("%H:%M")) }
                        if !absence.reason.is_empty() {
                            <span class="ms-3"><i class="bi bi-tag me-1"></i>{ &absence.reason }</span>
                        }
                        if !absence.created_user.is_empty() {
                            <span class="ms-3"><i class="bi bi-person me-1"></i>{ &absence.created_user }</span>
                        }
                    </div>
                    if !absence.text.is_empty() {
                        <div class="mt-2 small message-content">{ &absence.text }</div>
                    }
                </div>
                if absence.can_edit {
                    <div class="d-flex gap-2 flex-shrink-0">
                        <button class="btn btn-sm btn-outline-primary" title="Edit" onclick={emit_own(&props.on_edit)}>
                            <i class="bi bi-pencil"></i>
                        </button>
                        if props.deletable {
                            <button class="btn btn-sm btn-outline-danger" title="Withdraw" onclick={emit_own(&props.on_delete)}>
                                <i class="bi bi-trash"></i>
                            </button>
                        }
                    </div>
                }
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
struct DeleteAbsenceModalProps {
    absence: Absence,
    on_close: Callback<()>,
    on_deleted: Callback<()>,
    on_error: Callback<String>,
}

#[function_component(DeleteAbsenceModal)]
fn delete_absence_modal(props: &DeleteAbsenceModalProps) -> Html {
    let deleting = use_state(|| false);

    let on_confirm = {
        let (id, deleting) = (props.absence.id, deleting.clone());
        let (on_deleted, on_error) = (props.on_deleted.clone(), props.on_error.clone());
        Callback::from(move |_: MouseEvent| {
            deleting.set(true);
            let (deleting, on_deleted, on_error) = (deleting.clone(), on_deleted.clone(), on_error.clone());
            spawn_local(async move {
                match delete_absence(id).await {
                    Ok(()) => on_deleted.emit(()),
                    Err(error) => on_error.emit(error),
                }
                deleting.set(false);
            });
        })
    };

    html! {
        <ModalFrame title="Withdraw absence" on_close={props.on_close.clone()}>
            <p class="mb-4">
                { format!(
                    "Withdraw the absence on {} from {} to {}?",
                    props.absence.start_date.format("%d.%m.%Y"),
                    props.absence.start_time.format("%H:%M"),
                    props.absence.end_time.format("%H:%M"),
                ) }
            </p>
            <div class="d-flex justify-content-end gap-2">
                <button class="btn btn-outline-secondary" onclick={props.on_close.reform(|_| ())}>{"Cancel"}</button>
                <button class="btn btn-danger" disabled={*deleting} onclick={on_confirm}>
                    if *deleting {
                        <span class="spinner-border spinner-border-sm me-2" role="status"></span>
                    }
                    {"Withdraw"}
                </button>
            </div>
        </ModalFrame>
    }
}

async fn delete_absence(id: i32) -> Result<(), String> {
    let client = UntisClient::new().map_err(|e| e.to_string())?;
    client.delete_absences(&[id]).await.map_err(|e| e.to_string())
}

#[derive(Properties, PartialEq)]
struct AbsenceFormModalProps {
    reasons: Vec<AbsenceReason>,
    meta: AbsenceMeta,
    /// the absence being changed, none to report a new one
    #[prop_or_default]
    editing: Option<Absence>,
    on_close: Callback<()>,
    on_saved: Callback<()>,
    on_error: Callback<String>,
}

#[function_component(AbsenceFormModal)]
fn absence_form_modal(props: &AbsenceFormModalProps) -> Html {
    let today = Local::now().date_naive();
    let (meta, editing) = (&props.meta, &props.editing);

    // an absence being changed fills the form, a new one starts from the school's defaults
    let date = use_state(|| date_value(editing.as_ref().map(|a| a.start_date).or(meta.default_date), today));
    let end_date = use_state(|| date_value(editing.as_ref().map(|a| a.end_date).or(meta.default_date), today));
    let start_time = use_state(|| time_value(editing.as_ref().map(|a| a.start_time).or(meta.default_start_time), 8, 0));
    let end_time = use_state(|| time_value(editing.as_ref().map(|a| a.end_time).or(meta.default_end_time), 17, 0));
    let reason_id = use_state(|| match editing {
        Some(absence) => Some(absence.reason_id).filter(|id| *id > 0),
        None => meta.default_absence_reason,
    });
    let text = use_state(|| editing.as_ref().map(|a| a.text.clone()).unwrap_or_default());
    let saving = use_state(|| false);
    let error = use_state(|| None::<String>);

    let on_submit = {
        let (date, end_date, start_time, end_time) = (date.clone(), end_date.clone(), start_time.clone(), end_time.clone());
        let (reason_id, text, saving, error) = (reason_id.clone(), text.clone(), saving.clone(), error.clone());
        let (on_saved, on_error) = (props.on_saved.clone(), props.on_error.clone());
        let edited_id = props.editing.as_ref().map(|absence| absence.id);
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

    let on_input = |state: UseStateHandle<String>| {
        Callback::from(move |e: InputEvent| {
            state.set(e.target_unchecked_into::<HtmlInputElement>().value());
        })
    };

    html! {
        <ModalFrame title={if editing.is_some() { "Edit absence" } else { "Report absence" }}
                    on_close={props.on_close.clone()}>
            <div class="row g-3">
                <div class="col-12 col-md-6">
                    <label class="form-label small text-secondary">{"From"}</label>
                    <div class="d-flex gap-2">
                        <input type="date" class="form-control bg-dark text-white border-secondary"
                               value={(*date).clone()} oninput={on_input(date.clone())} />
                        <input type="time" class="form-control bg-dark text-white border-secondary"
                               value={(*start_time).clone()} oninput={on_input(start_time.clone())} />
                    </div>
                </div>
                <div class="col-12 col-md-6">
                    <label class="form-label small text-secondary">{"To"}</label>
                    <div class="d-flex gap-2">
                        <input type="date" class="form-control bg-dark text-white border-secondary"
                               value={(*end_date).clone()} oninput={on_input(end_date.clone())} />
                        <input type="time" class="form-control bg-dark text-white border-secondary"
                               value={(*end_time).clone()} oninput={on_input(end_time.clone())} />
                    </div>
                </div>
                if !props.reasons.is_empty() {
                    <div class="col-12">
                        <label class="form-label small text-secondary">{"Reason"}</label>
                        <select class="form-select bg-dark text-white border-secondary" onchange={
                            let reason_id = reason_id.clone();
                            Callback::from(move |e: Event| {
                                let value = e.target_unchecked_into::<HtmlSelectElement>().value();
                                reason_id.set(value.parse().ok());
                            })
                        }>
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
                              oninput={
                                  let text = text.clone();
                                  Callback::from(move |e: InputEvent| {
                                      text.set(e.target_unchecked_into::<HtmlTextAreaElement>().value());
                                  })
                              } />
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
        </ModalFrame>
    }
}

/// The `YYYY-MM-DD` an `<input type="date">` takes, falling back to the given day
fn date_value(date: Option<NaiveDate>, fallback: NaiveDate) -> String {
    date.unwrap_or(fallback).format("%Y-%m-%d").to_string()
}

/// The `HH:MM` an `<input type="time">` takes, falling back to the given hour and minute
fn time_value(time: Option<NaiveTime>, hour: u32, minute: u32) -> String {
    time.or_else(|| NaiveTime::from_hms_opt(hour, minute, 0))
        .map(|time| time.format("%H:%M").to_string())
        .unwrap_or_default()
}

fn build_absence(
    start_date: &str,
    end_date: &str,
    start_time: &str,
    end_time: &str,
    reason_id: Option<i32>,
    text: &str,
) -> Result<NewAbsence, String> {
    let parse_date = |value: &str| NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| "Pick a start and an end date".to_string());
    let parse_time = |value: &str| NaiveTime::parse_from_str(value, "%H:%M")
        .map_err(|_| "Pick a start and an end time".to_string());

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

#[derive(Properties, PartialEq)]
struct ModalFrameProps {
    title: AttrValue,
    on_close: Callback<()>,
    #[prop_or_default]
    children: Children,
}

#[function_component(ModalFrame)]
fn modal_frame(props: &ModalFrameProps) -> Html {
    let on_close = props.on_close.clone();

    html! {
        <div class="modal d-block" style="background: rgba(0,0,0,0.85); z-index: 1050;" onclick={
            let on_close = on_close.clone();
            move |_| on_close.emit(())
        }>
            <div class="modal-dialog modal-dialog-centered" onclick={|e: MouseEvent| e.stop_propagation()}>
                <div class="modal-content border-primary shadow-lg bg-dark text-light">
                    <div class="modal-header border-primary bg-black text-white">
                        <h5 class="modal-title fw-bold">{ &props.title }</h5>
                        <button type="button" class="btn-close btn-close-white" onclick={
                            let on_close = on_close.clone();
                            move |_| on_close.emit(())
                        }></button>
                    </div>
                    <div class="modal-body p-4" style="background-color: #1a1d20;">
                        { props.children.clone() }
                    </div>
                </div>
            </div>
        </div>
    }
}
