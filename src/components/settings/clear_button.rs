use super::card::SettingsModal;
use crate::storage;
use yew::prelude::*;

/// Resets the app to how it was installed, after asking
#[function_component(ClearSettingsButton)]
pub fn clear_settings_button() -> Html {
    let show_modal = use_state(|| false);

    let toggle_modal = {
        let show_modal = show_modal.clone();
        Callback::from(move |_| show_modal.set(!*show_modal))
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
            <button class="btn btn-outline-danger" onclick={toggle_modal.clone()}>
                {"Clear Settings"}
            </button>

            if *show_modal {
                <SettingsModal title="Reset Settings?" on_close={toggle_modal.clone()}>
                    <div class="modal-body">
                        <p>{"This will delete all settings, including saved credentials and preferences. This action cannot be undone."}</p>
                    </div>
                    <div class="modal-footer">
                        <button type="button" class="btn btn-success" onclick={toggle_modal}>{"Cancel"}</button>
                        <button type="button" class="btn btn-danger" onclick={confirm_clear}>{"Yes, Clear Everything"}</button>
                    </div>
                </SettingsModal>
            }
        </>
    }
}
