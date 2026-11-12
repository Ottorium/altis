//! The sections of the visual settings that are about the lessons themselves.

use crate::components::forms::{select_value, Checkbox};
use altis_core::settings::{LessonDisplaySettings, LessonFont, LessonLine, LessonTextSettings};
use yew::prelude::*;

/// The text sizes on offer, in percent of the usual one
const FONT_SCALES: [u32; 6] = [75, 90, 100, 110, 125, 150];

const SELECT_CLASS: &str = "form-select form-select-sm bg-dark text-light border-secondary w-auto";
const HEADING_CLASS: &str = "form-label fw-bold small text-light mb-1";

/// A callback that changes a copy of the value by what it is called with and hands the copy on
fn change<T: Clone + 'static, A: 'static>(value: &T, on_change: &Callback<T>, apply: impl Fn(&mut T, A) + 'static) -> Callback<A> {
    let (value, on_change) = (value.clone(), on_change.clone());
    Callback::from(move |argument| {
        let mut changed = value.clone();
        apply(&mut changed, argument);
        on_change.emit(changed);
    })
}

#[derive(Properties, PartialEq)]
pub struct LessonTextProps {
    pub settings: LessonTextSettings,
    pub on_change: Callback<LessonTextSettings>,
}

/// What is written on a lesson, and in which typeface
#[function_component(LessonTextSection)]
pub fn lesson_text_section(props: &LessonTextProps) -> Html {
    let settings = &props.settings;
    let on_change = &props.on_change;

    let on_toggle_line = change(settings, on_change, |settings: &mut LessonTextSettings, line: LessonLine| settings.toggle_line(line));
    let on_font_change = change(settings, on_change, |settings: &mut LessonTextSettings, e: Event| {
        let picked = select_value(&e);
        if let Some(font) = LessonFont::ALL.into_iter().find(|font| font.label() == picked) {
            settings.font = font;
        }
    });
    let on_scale_change = change(settings, on_change, |settings: &mut LessonTextSettings, e: Event| {
        if let Ok(percent) = select_value(&e).parse() {
            settings.font_scale_percent = percent;
        }
    });

    // a size set by hand, in an import for instance, is offered along with the usual ones
    let mut scales = FONT_SCALES.to_vec();
    if !scales.contains(&settings.font_scale_percent) {
        scales.push(settings.font_scale_percent);
        scales.sort();
    }

    html! {
        <div class="mb-3">
            <label class={HEADING_CLASS}>{"Lesson Text"}</label>
            <div class="form-text text-secondary small mb-2">
                {"What is written on a lesson in the timetable, from top to bottom."}
            </div>
            <div class="btn-group w-100 mb-2" role="group" aria-label="Lines written on a lesson">
                { for LessonLine::ALL.into_iter().map(|line| {
                    let button_class = if settings.has_line(line) { "btn-primary" } else { "btn-outline-secondary" };
                    html! {
                        <button type="button" class={classes!("btn", "btn-sm", button_class)} onclick={on_toggle_line.reform(move |_| line)}>
                            { line.label() }
                        </button>
                    }
                })}
            </div>
            <Checkbox id="lessonLongNames" label="Write subjects and teachers out in full" class="form-check mb-2"
                      checked={settings.long_names}
                      on_toggle={change(settings, on_change, |settings: &mut LessonTextSettings, _| settings.long_names = !settings.long_names)} />
            <div class="d-flex flex-wrap align-items-center gap-2">
                <label class="small text-secondary" for="lessonFont">{"Font"}</label>
                <select id="lessonFont" class={SELECT_CLASS} onchange={on_font_change}>
                    { for LessonFont::ALL.into_iter().map(|font| html! {
                        <option value={font.label()} selected={settings.font == font}>{ font.label() }</option>
                    })}
                </select>
                <label class="small text-secondary" for="lessonFontScale">{"Size"}</label>
                <select id="lessonFontScale" class={SELECT_CLASS} onchange={on_scale_change}>
                    { for scales.into_iter().map(|percent| html! {
                        <option value={percent.to_string()} selected={settings.font_scale_percent == percent}>{ format!("{percent}%") }</option>
                    })}
                </select>
                <Checkbox id="lessonBold" label="Bold" class="form-check ms-2"
                          checked={settings.bold}
                          on_toggle={change(settings, on_change, |settings: &mut LessonTextSettings, _| settings.bold = !settings.bold)} />
            </div>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct LessonDisplayProps {
    pub settings: LessonDisplaySettings,
    pub on_change: Callback<LessonDisplaySettings>,
}

/// How the lessons of a timetable are drawn
#[function_component(LessonDisplaySection)]
pub fn lesson_display_section(props: &LessonDisplayProps) -> Html {
    let settings = &props.settings;
    let toggle = |flip: fn(&mut LessonDisplaySettings) -> &mut bool| {
        change(settings, &props.on_change, move |settings: &mut LessonDisplaySettings, _: MouseEvent| {
            let flag = flip(settings);
            *flag = !*flag;
        })
    };

    html! {
        <div class="mb-3">
            <label class={HEADING_CLASS}>{"Lesson Display"}</label>
            <div class="d-flex flex-column gap-1 mt-1">
                <Checkbox id="dimPastLessons" label="Grey out lessons that are over"
                          checked={settings.dim_past_lessons} on_toggle={toggle(|settings| &mut settings.dim_past_lessons)} />
                <Checkbox id="showNowLine" label="Mark the current time with a line"
                          checked={settings.show_now_line} on_toggle={toggle(|settings| &mut settings.show_now_line)} />
                <Checkbox id="hideCancelled" label="Hide cancelled lessons"
                          checked={settings.hide_cancelled} on_toggle={toggle(|settings| &mut settings.hide_cancelled)} />
            </div>
        </div>
    }
}
