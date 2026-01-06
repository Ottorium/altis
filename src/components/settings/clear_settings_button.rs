use crate::persistence_manager::PersistenceManager;
use yew::{function_component, html, use_state, Callback, Html};

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
            let _ = PersistenceManager::clear_storage();
            show_modal.set(false);
            let _ = web_sys::window().unwrap().location().reload();
        })
    };

    html! {
        <>
            <button class="btn btn-outline-danger" onclick={toggle_modal.clone()}>
                {"Clear Settings"}
            </button>

            if *show_modal {
                <div class="modal d-block" tabindex="-1" style="background: rgba(0,0,0,0.5)">
                    <div class="modal-dialog modal-dialog-centered">
                        <div class="modal-content bg-dark text-white border-primary">
                            <div class="modal-header">
                                <h5 class="modal-title">{"Reset Settings?"}</h5>
                                <button type="button" class="btn-close btn-close-white" onclick={toggle_modal.clone()}></button>
                            </div>
                            <div class="modal-body">
                                <p>{"This will delete all settings, including saved credentials and preferences. This action cannot be undone."}</p>
                            </div>
                            <div class="modal-footer">
                                <button type="button" class="btn btn-success" onclick={toggle_modal}>{"Cancel"}</button>
                                <button type="button" class="btn btn-danger" onclick={confirm_clear}>{"Yes, Clear Everything"}</button>
                            </div>
                        </div>
                    </div>
                </div>
            }
        </>
    }
}
