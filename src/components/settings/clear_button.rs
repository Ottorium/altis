use super::card::SettingsModal;
use crate::storage;
use yew::prelude::*;

/// Resets the app to how it was installed, after asking
#[function_component(ClearSettingsButton)]
pub fn clear_settings_button() -> Html {
    let show_modal = use_state(|| false);

    let set_modal = |open: bool| {
        let show_modal = show_modal.clone();
        Callback::from(move |_: ()| show_modal.set(open))
    };

    let confirm_clear = {
        let show_modal = show_modal.clone();
        Callback::from(move |_| {
            let _ = storage::clear_all();
            show_modal.set(false);
            // every screen holds on to what it loaded, so the app starts over
            if let Some(window) = web_sys::window() {
                let _ = window.location().reload();
            }
        })
    };

    html! {
        <>
            <button class="btn btn-outline-danger" onclick={set_modal(true).reform(|_| ())}>
                {"Clear Settings"}
            </button>

            if *show_modal {
                <SettingsModal title="Reset Settings?" on_close={set_modal(false)}>
                    <div class="modal-body">
                        <p>{"This will delete all settings, including saved credentials and preferences. This action cannot be undone."}</p>
                    </div>
                    <div class="modal-footer">
                        <button type="button" class="btn btn-success" autofocus=true onclick={set_modal(false).reform(|_| ())}>{"Cancel"}</button>
                        <button type="button" class="btn btn-danger" onclick={confirm_clear}>{"Yes, Clear Everything"}</button>
                    </div>
                </SettingsModal>
            }
        </>
    }
}
