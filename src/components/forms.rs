//! The bits every form in the app is made of.

use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

/// What was typed into the `<input>` an event came from
pub fn input_value(event: &impl TargetCast) -> String {
    event.target_unchecked_into::<HtmlInputElement>().value()
}

/// The option picked in the `<select>` an event came from
pub fn select_value(event: &impl TargetCast) -> String {
    event.target_unchecked_into::<HtmlSelectElement>().value()
}

/// A callback that keeps a text field's state in step with what is typed into it
pub fn on_text_input(state: &UseStateHandle<String>) -> Callback<InputEvent> {
    let state = state.clone();
    Callback::from(move |event: InputEvent| state.set(input_value(&event)))
}

/// A callback that changes the state's value in place, given what the callback was called with
pub fn update<T: Clone + 'static, A: 'static>(state: &UseStateHandle<T>, change: impl Fn(&mut T, A) + 'static) -> Callback<A> {
    let state = state.clone();
    Callback::from(move |argument| {
        let mut value = (*state).clone();
        change(&mut value, argument);
        state.set(value);
    })
}

/// Runs the action when Enter is pressed in a text field, rather than submitting its form
pub fn on_enter(action: Callback<()>) -> Callback<KeyboardEvent> {
    Callback::from(move |event: KeyboardEvent| {
        if event.key() == "Enter" {
            event.prevent_default();
            action.emit(());
        }
    })
}

/// Greys a section out and takes it out of reach while what it belongs to is switched off
pub fn dimmed_unless(enabled: bool) -> &'static str {
    match enabled {
        true => "",
        false => "opacity: 0.4; pointer-events: none;",
    }
}

#[derive(Properties, PartialEq)]
pub struct CheckboxProps {
    pub id: AttrValue,
    pub label: AttrValue,
    pub checked: bool,
    pub on_toggle: Callback<MouseEvent>,
    /// the classes of the wrapper, which decide the spacing around the checkbox
    #[prop_or(AttrValue::Static("form-check"))]
    pub class: AttrValue,
    #[prop_or(AttrValue::Static("form-check-label small text-secondary"))]
    pub label_class: AttrValue,
    #[prop_or_default]
    pub style: Option<AttrValue>,
    /// what follows the label: a hint, an info icon
    #[prop_or_default]
    pub children: Children,
}

/// A labelled checkbox
#[function_component(Checkbox)]
pub fn checkbox(props: &CheckboxProps) -> Html {
    html! {
        <div class={&props.class} style={props.style.clone()}>
            <input type="checkbox" class="form-check-input" id={&props.id} checked={props.checked} onclick={&props.on_toggle} />
            <label class={&props.label_class} for={&props.id} style="cursor: pointer;">{ &props.label }</label>
            { props.children.clone() }
        </div>
    }
}
