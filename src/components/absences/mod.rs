mod card;
mod delete;
mod form;
mod toolbar;

use crate::untis::UntisClient;
use altis_core::errors::ApiError;
use altis_core::models::absences::{Absence, AbsenceMeta, AbsencesData};
use altis_core::untis::week::school_year;
use card::AbsenceCard;
use delete::DeleteAbsenceModal;
use form::AbsenceFormModal;
use toolbar::{render_summary, ExcuseNoteButton, SchoolYearPicker};
use yew::prelude::*;
use yew::suspense::use_future_with;

/// The absence form that is open, if any
#[derive(Clone, PartialEq)]
enum AbsenceForm {
    New,
    Edit(Absence),
}

/// The absences of a school year, and what the school lets the student do about them
async fn load_absences(year_offset: i32) -> Result<(AbsencesData, Option<AbsenceMeta>), ApiError> {
    let (start, end) = school_year(year_offset);
    let client = UntisClient::new()?;
    let absences = client.get_absences(start, end).await?;
    // only decides which buttons are offered, so a school that doesn't answer it still gets a list
    let meta = client.get_absence_meta().await.ok();
    Ok((absences, meta))
}

/// Every absence of a school year with its excuse status, and the means to report, change and
/// withdraw one's own
#[function_component(AbsencesPage)]
pub fn absences_page() -> HtmlResult {
    let reload_trigger = use_state(|| 0u32);
    // 0 is the running school year, 1 the one before it
    let year_offset = use_state(|| 0);
    let form = use_state(|| None::<AbsenceForm>);
    let pending_delete = use_state(|| None::<Absence>);
    // of a delete or a report, the list itself reports its own errors
    let action_error = use_state(|| None::<String>);

    let loaded = use_future_with((*reload_trigger, *year_offset), |deps| load_absences(deps.1))?;

    let reload = {
        let reload_trigger = reload_trigger.clone();
        Callback::from(move |_: ()| reload_trigger.set(*reload_trigger + 1))
    };

    let on_year_change = {
        let year_offset = year_offset.clone();
        Callback::from(move |offset: i32| year_offset.set(offset))
    };

    let show_error = {
        let action_error = action_error.clone();
        Callback::from(move |message: String| action_error.set(Some(message)))
    };

    // opening a dialog puts away the error of the last one
    let open_form = |open: fn(Absence) -> AbsenceForm| {
        let (form, action_error) = (form.clone(), action_error.clone());
        Callback::from(move |absence: Absence| {
            action_error.set(None);
            form.set(Some(open(absence)));
        })
    };
    let on_edit = open_form(AbsenceForm::Edit);
    let on_report = {
        let (form, action_error) = (form.clone(), action_error.clone());
        Callback::from(move |_: MouseEvent| {
            action_error.set(None);
            form.set(Some(AbsenceForm::New));
        })
    };
    let on_delete = {
        let (pending_delete, action_error) = (pending_delete.clone(), action_error.clone());
        Callback::from(move |absence: Absence| {
            action_error.set(None);
            pending_delete.set(Some(absence));
        })
    };

    let (data, meta) = match &*loaded {
        Ok((data, meta)) => (Some(data), meta.as_ref()),
        Err(_) => (None, None),
    };

    let can_report = data.is_some_and(|data| data.show_create_absence) && meta.is_none_or(|meta| meta.can_report_absence);

    let list = match &*loaded {
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

            let deletable = meta.as_ref().is_none_or(|meta| meta.can_delete);
            html! {
                <div class="d-flex flex-column gap-2 p-3">
                    { for absences.into_iter().map(|absence| html! {
                        <AbsenceCard key={absence.id} absence={absence.clone()} {deletable} on_edit={on_edit.clone()} on_delete={on_delete.clone()} />
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
                            on_error={show_error.clone()}
                        />
                    }
                    if can_report {
                        <button class="btn btn-primary btn-sm text-dark fw-semibold" onclick={on_report}>
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
                    reasons={meta.map(|meta| meta.absence_reasons.clone())
                        .or_else(|| data.map(|data| data.absence_reasons.clone()))
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
                    on_error={show_error.clone()}
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
                        let (pending_delete, show_error) = (pending_delete.clone(), show_error.clone());
                        Callback::from(move |message| { pending_delete.set(None); show_error.emit(message); })
                    }
                />
            }
        </div>
    })
}
