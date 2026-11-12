use web_sys::MouseEvent;
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
        <span class="badge rounded-pill bg-success opacity-75" style="font-size: 0.7rem;">
            {"Settings autosaved"}
        </span>
    }
}

#[derive(Properties, PartialEq)]
pub struct SettingsModalProps {
    pub title: AttrValue,
    pub on_close: Callback<MouseEvent>,
    /// keeps the dialog within the screen and scrolls its body instead of the page
    #[prop_or_default]
    pub scrollable: bool,
    /// the dialog's body and footer
    pub children: Children,
}

/// A dialog of the settings screen
#[function_component(SettingsModal)]
pub fn settings_modal(props: &SettingsModalProps) -> Html {
    html! {
        <div class="modal d-block" tabindex="-1" style="background: rgba(0,0,0,0.5)">
            <div class={classes!("modal-dialog", "modal-dialog-centered", props.scrollable.then_some("modal-dialog-scrollable"))}>
                <div class="modal-content bg-dark text-white border-primary">
                    <div class="modal-header">
                        <h5 class="modal-title">{ &props.title }</h5>
                        <button type="button" class="btn-close btn-close-white" onclick={&props.on_close}></button>
                    </div>
                    { props.children.clone() }
                </div>
            </div>
        </div>
    }
}
