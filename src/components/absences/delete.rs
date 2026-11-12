use crate::components::modal::Modal;
use crate::untis::UntisClient;
use altis_core::models::absences::Absence;
use wasm_bindgen_futures::spawn_local;
use web_sys::MouseEvent;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct DeleteAbsenceModalProps {
    pub absence: Absence,
    pub on_close: Callback<()>,
    pub on_deleted: Callback<()>,
    pub on_error: Callback<String>,
}

/// Asks before an absence is withdrawn, and withdraws it
#[function_component(DeleteAbsenceModal)]
pub fn delete_absence_modal(props: &DeleteAbsenceModalProps) -> Html {
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
        <Modal title="Withdraw absence" on_close={props.on_close.clone()} body_class="p-4">
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
        </Modal>
    }
}

async fn delete_absence(id: i32) -> Result<(), String> {
    let client = UntisClient::new().map_err(|e| e.to_string())?;
    client.delete_absences(&[id]).await.map_err(|e| e.to_string())
}
