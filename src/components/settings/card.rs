use crate::components::modal::use_modal_dialog;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SettingsCardProps {
    pub title: AttrValue,
    pub children: Children,
}

/// A group of settings under a heading
#[function_component(SettingsCard)]
pub fn settings_card(props: &SettingsCardProps) -> Html {
    html! {
        <div class="card border-primary mb-4 shadow-sm">
            <div class="card-header">
                <h5 class="mb-0">{&props.title}</h5>
            </div>
            <div class="card-body">
                {props.children.clone()}
            </div>
        </div>
    }
}

/// The line between two groups of settings in a card
pub fn divider() -> Html {
    html! { <hr class="border-secondary opacity-25 my-3" /> }
}

/// Tells that a card's settings are stored as they are changed, without a button to press
pub fn autosaved_badge() -> Html {
    html! {
        <span class="badge rounded-pill bg-success opacity-75 autosave-badge">
            {"Settings autosaved"}
        </span>
    }
}

#[derive(Properties, PartialEq)]
pub struct SettingsModalProps {
    pub title: AttrValue,
    /// the dialog was closed: by its cross, Esc or the back gesture
    pub on_close: Callback<()>,
    /// keeps the dialog within the screen and scrolls its body instead of the whole dialog
    #[prop_or_default]
    pub scrollable: bool,
    /// the dialog's body and footer
    pub children: Children,
}

/// A dialog of the settings screen. Unlike the other dialogs a click beside it doesn't close it:
/// what is in it is about to change or wipe the settings.
#[function_component(SettingsModal)]
pub fn settings_modal(props: &SettingsModalProps) -> Html {
    let dialog = use_modal_dialog(props.on_close.clone(), false);

    html! {
        <dialog ref={dialog} aria-labelledby="settings-dialog-title"
                class={classes!("app-dialog", props.scrollable.then_some("app-dialog-scrollable"))}>
            <div class="modal-content bg-dark text-white border-primary">
                <div class="modal-header">
                    <h5 class="modal-title" id="settings-dialog-title">{ &props.title }</h5>
                    <button type="button" class="btn-close btn-close-white" aria-label="Close" onclick={props.on_close.reform(|_| ())}></button>
                </div>
                { props.children.clone() }
            </div>
        </dialog>
    }
}
