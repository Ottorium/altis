//! Dialogs over the page: native `<dialog>`s opened as modals, which the browser keeps on top,
//! keeps the focus in, and closes on Esc or Android's back gesture.

use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Element, Event, EventTarget, HtmlDialogElement, MouseEvent};
use yew::prelude::*;

/// An event listener that is taken off its target again when it is dropped
struct Listener {
    target: EventTarget,
    kind: &'static str,
    callback: Closure<dyn FnMut(Event)>,
}

impl Listener {
    fn new(target: &EventTarget, kind: &'static str, handler: impl FnMut(Event) + 'static) -> Self {
        let callback = Closure::<dyn FnMut(Event)>::new(handler);
        let _ = target.add_event_listener_with_callback(kind, callback.as_ref().unchecked_ref());
        Self { target: target.clone(), kind, callback }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self.target.remove_event_listener_with_callback(self.kind, self.callback.as_ref().unchecked_ref());
    }
}

/// Whether the browser closes a dialog on a click beside it by itself, given `closedby="any"`.
/// WebKit doesn't yet.
fn closes_on_click_beside() -> bool {
    let prototype = web_sys::window()
        .and_then(|window| js_sys::Reflect::get(&window, &JsValue::from_str("HTMLDialogElement")).ok())
        .and_then(|class| js_sys::Reflect::get(&class, &JsValue::from_str("prototype")).ok());
    prototype.is_some_and(|prototype| js_sys::Reflect::has(&prototype, &JsValue::from_str("closedBy")).unwrap_or(false))
}

/// Whether a pointer event happened beside the dialog, on its backdrop. The position decides, not
/// the target: the dialog is also the target of a click on its own padding.
fn is_beside(event: &Event, dialog: &Element) -> bool {
    let Some(event) = event.dyn_ref::<MouseEvent>() else { return false };
    let bounds = dialog.get_bounding_client_rect();
    let (x, y) = (f64::from(event.client_x()), f64::from(event.client_y()));
    x < bounds.left() || x > bounds.right() || y < bounds.top() || y > bounds.bottom()
}

/// Opens the `<dialog>` the returned ref is put on as a modal once it is in the page, and calls
/// `on_close` when the browser closes it: on Esc, the back gesture, or - with `closes_beside` - a
/// click beside it. A dialog that is taken out of the page by its parent doesn't call it.
#[hook]
pub fn use_modal_dialog(on_close: Callback<()>, closes_beside: bool) -> NodeRef {
    let dialog = use_node_ref();
    // the listener outlives the render it was made in, so it reads the callback of the latest one
    let latest_on_close = use_mut_ref(|| on_close.clone());
    *latest_on_close.borrow_mut() = on_close;

    {
        let dialog = dialog.clone();
        use_effect_with((), move |_| {
            let mut listeners = Vec::new();
            if let Some(element) = dialog.cast::<HtmlDialogElement>() {
                let _ = element.show_modal();
                listeners.push(Listener::new(&element, "close", move |_| latest_on_close.borrow().emit(())));

                // where `closedby` isn't known, a press and a release both beside the dialog close
                // it, so a text selection dragged out over the backdrop doesn't
                if closes_beside && !closes_on_click_beside() {
                    let pressed_beside = std::rc::Rc::new(std::cell::Cell::new(false));
                    let (pressed, on_down) = (pressed_beside.clone(), element.clone());
                    listeners.push(Listener::new(&element, "pointerdown", move |event| pressed.set(is_beside(&event, &on_down))));
                    let on_up = element.clone();
                    listeners.push(Listener::new(&element, "pointerup", move |event| {
                        if pressed_beside.get() && is_beside(&event, &on_up) {
                            on_up.close();
                        }
                    }));
                }
            }
            move || drop(listeners)
        });
    }
    dialog
}

#[derive(Properties, PartialEq)]
pub struct ModalProps {
    pub title: AttrValue,
    pub on_close: Callback<()>,
    /// a wider dialog, for content that is more than a short form
    #[prop_or_default]
    pub large: bool,
    /// keeps the dialog within the screen and scrolls its body instead of the whole dialog
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

/// A dialog over the page. Clicking beside it closes it, as do Esc, the back gesture and the
/// cross in its corner.
#[function_component(Modal)]
pub fn modal(props: &ModalProps) -> Html {
    let dialog = use_modal_dialog(props.on_close.clone(), true);
    let title_id = format!("dialog-title-{}", props.title.to_lowercase().replace(' ', "-"));

    html! {
        <dialog ref={dialog} closedby="any" aria-labelledby={title_id.clone()}
                class={classes!("app-dialog", props.large.then_some("app-dialog-large"), props.scrollable.then_some("app-dialog-scrollable"))}>
            <div class="modal-content border-primary shadow-lg bg-dark text-light">
                <div class="modal-header border-primary bg-black text-white">
                    <h5 class="modal-title fw-bold" id={title_id}>{ &props.title }</h5>
                    <button type="button" class="btn-close btn-close-white" aria-label="Close" onclick={props.on_close.reform(|_| ())}></button>
                </div>
                <div class={classes!("modal-body", "modal-body-sunken", props.limit_body_height.then_some("modal-body-limited"), props.body_class.clone())}>
                    { props.children.clone() }
                </div>
                if let Some(footer) = &props.footer {
                    <div class="modal-footer border-primary bg-black">{ footer.clone() }</div>
                }
            </div>
        </dialog>
    }
}
