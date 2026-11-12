use crate::components::forms::{input_value, on_enter, on_text_input};
use crate::storage;
use altis_core::settings::is_dark_color;
use std::collections::BTreeMap;
use yew::prelude::*;

/// The colour the picker starts on for a new subject
const DEFAULT_COLOR: &str = "#4a90e2";

#[derive(Properties, PartialEq)]
pub struct SubjectColorsProps {
    /// the colour set for each subject, by the subject's name
    pub colors: BTreeMap<String, String>,
    /// a subject was given a colour, as (subject, colour)
    pub on_set: Callback<(String, String)>,
    pub on_remove: Callback<String>,
}

/// The colours subjects are drawn in instead of the ones Untis gives them
#[function_component(SubjectColors)]
pub fn subject_colors(props: &SubjectColorsProps) -> Html {
    let new_subject = use_state(String::new);
    let new_color = use_state(|| DEFAULT_COLOR.to_string());
    let known_subjects = use_state(storage::known_subjects);

    let on_add = {
        let (new_subject, new_color, on_set) = (new_subject.clone(), new_color.clone(), props.on_set.clone());
        Callback::from(move |_: ()| {
            let name = new_subject.trim().to_string();
            if !name.is_empty() {
                on_set.emit((name, (*new_color).clone()));
                new_subject.set(String::new());
            }
        })
    };
    let on_add_click = {
        let on_add = on_add.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            on_add.emit(());
        })
    };

    // the subjects of the cached timetables that have no colour of their own yet
    let suggestions = known_subjects.iter().filter(|subject| !props.colors.keys().any(|name| name.eq_ignore_ascii_case(subject)));

    html! {
        <div class="mb-3">
            <label class="form-label fw-bold small text-light mb-1">{"Subject Colour Overrides"}</label>
            <div class="form-text text-secondary small mb-3">
                {"Set custom background colours for specific subjects in the timetable."}
            </div>

            if props.colors.is_empty() {
                <div class="text-secondary small fst-italic mb-3">
                    {"No subject colour overrides configured."}
                </div>
            } else {
                <div class="d-flex flex-column gap-2 mb-3">
                    { for props.colors.iter().map(|(subject, color)| render_override(subject, color, props)) }
                </div>
            }

            <div class="card bg-black bg-opacity-25 border-secondary border-opacity-25 p-2">
                <div class="d-flex gap-2 align-items-center">
                    <input
                        type="text"
                        class="form-control form-control-sm bg-dark text-light border-secondary"
                        placeholder="Subject (e.g. M, D, ENG)"
                        value={(*new_subject).clone()}
                        oninput={on_text_input(&new_subject)}
                        onkeydown={on_enter(on_add)}
                        list="known-subjects-list"
                    />
                    <datalist id="known-subjects-list">
                        { for suggestions.map(|subject| html! { <option value={subject.clone()} /> }) }
                    </datalist>
                    <input
                        type="color"
                        class="form-control form-control-color form-control-sm border-0 p-0"
                        value={(*new_color).clone()}
                        oninput={on_text_input(&new_color)}
                        title="Choose colour"
                        style="cursor: pointer; width: 38px; height: 32px; background: transparent; flex-shrink: 0;"
                    />
                    <button
                        type="button"
                        class="btn btn-sm btn-primary flex-shrink-0"
                        onclick={on_add_click}
                        disabled={new_subject.trim().is_empty()}
                    >
                        <i class="bi bi-plus-lg me-1"></i>{"Add"}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// A subject with its colour, which can be changed or taken away again
fn render_override(subject: &str, color: &str, props: &SubjectColorsProps) -> Html {
    let on_remove = {
        let subject = subject.to_string();
        props.on_remove.reform(move |_: MouseEvent| subject.clone())
    };
    let on_color_input = {
        let subject = subject.to_string();
        props.on_set.reform(move |e: InputEvent| (subject.clone(), input_value(&e)))
    };
    let text_class = if is_dark_color(color) { "text-white" } else { "text-black" };

    html! {
        <div class="d-flex align-items-center justify-content-between p-2 rounded bg-dark border border-secondary border-opacity-25" key={subject.to_string()}>
            <div class="d-flex align-items-center gap-2">
                <span
                    class={classes!("badge", "fs-6", "fw-bold", text_class)}
                    style={format!("background-color: {color}; min-width: 50px; text-align: center; border: 1px solid rgba(255,255,255,0.15);")}
                >
                    { subject }
                </span>
                <span class="text-secondary small font-monospace">{ color }</span>
            </div>
            <div class="d-flex align-items-center gap-2">
                <input
                    type="color"
                    class="form-control form-control-color form-control-sm border-0 p-0"
                    value={color.to_string()}
                    oninput={on_color_input}
                    title={format!("Change colour for {subject}")}
                    style="cursor: pointer; width: 36px; height: 32px; background: transparent;"
                />
                <button
                    type="button"
                    class="btn btn-outline-danger btn-sm px-2 py-1"
                    onclick={on_remove}
                    title={format!("Remove override for {subject}")}
                >
                    <i class="bi bi-trash"></i>
                </button>
            </div>
        </div>
    }
}
