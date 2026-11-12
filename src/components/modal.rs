use web_sys::MouseEvent;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ModalProps {
    pub title: AttrValue,
    pub on_close: Callback<()>,
    /// a wider dialog, for content that is more than a short form
    #[prop_or_default]
    pub large: bool,
    /// keeps the dialog within the screen and scrolls its body instead of the page
    #[prop_or_default]
    pub scrollable: bool,
    /// extra classes of the body, its padding for instance
    #[prop_or_default]
    pub body_class: Classes,
    /// limits the body's height and scrolls what doesn't fit
    #[prop_or_default]
    pub limit_body_height: bool,
    #[prop_or_default]
    pub footer: Option<Html>,
    #[prop_or_default]
    pub children: Children,
}

/// A dialog over the page. Clicking beside it closes it, as does the cross in its corner.
#[function_component(Modal)]
pub fn modal(props: &ModalProps) -> Html {
    let close = props.on_close.reform(|_: MouseEvent| ());
    let dialog_class = classes!(
        "modal-dialog",
        props.large.then_some("modal-lg"),
        "modal-dialog-centered",
        props.scrollable.then_some("modal-dialog-scrollable"),
    );
    let body_style = match props.limit_body_height {
        true => "max-height: 80vh; overflow-y: auto; background-color: #1a1d20;",
        false => "background-color: #1a1d20;",
    };

    html! {
        <div class="modal d-block" style="background: rgba(0,0,0,0.85); z-index: 1050;" onclick={close.clone()}>
            <div class={dialog_class} onclick={|e: MouseEvent| e.stop_propagation()}>
                <div class="modal-content border-primary shadow-lg bg-dark text-light">
                    <div class="modal-header border-primary bg-black text-white">
                        <h5 class="modal-title fw-bold">{ &props.title }</h5>
                        <button type="button" class="btn-close btn-close-white" onclick={close}></button>
                    </div>
                    <div class={classes!("modal-body", props.body_class.clone())} style={body_style}>
                        { props.children.clone() }
                    </div>
                    if let Some(footer) = &props.footer {
                        <div class="modal-footer border-primary bg-black">{ footer.clone() }</div>
                    }
                </div>
            </div>
        </div>
    }
}
