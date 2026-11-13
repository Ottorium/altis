use super::card::{autosaved_badge, divider, SettingsCard};
use super::lesson_sections::{LessonDisplaySection, LessonTextSection};
use super::subject_colors::SubjectColors;
use crate::components::forms::{dimmed_unless, input_value, update, Checkbox};
use altis_core::settings::{LessonDisplaySettings, LessonTextSettings, TimePadding, VisualSettings, ALL_WEEKDAYS};
use chrono::{NaiveTime, Weekday};
use web_sys::HtmlInputElement;
use yew::prelude::*;

const TIME_INPUT_FORMAT: &str = "%H:%M";

/// The `HH:MM` an `<input type="time">` takes
fn time_input_value(time: NaiveTime) -> String {
    time.format(TIME_INPUT_FORMAT).to_string()
}

#[derive(Properties, PartialEq)]
pub struct VisualCardProps {
    pub initial: VisualSettings,
    pub on_save: Callback<VisualSettings>,
}

/// How the timetable looks. Stored as it is changed.
#[function_component(VisualSettingsCard)]
pub fn visual_settings_card(props: &VisualCardProps) -> Html {
    let settings = use_state(|| props.initial.clone());

    {
        let on_save = props.on_save.clone();
        use_effect_with((*settings).clone(), move |settings| {
            on_save.emit(settings.clone());
            || ()
        });
    }

    // `onchange`, not `oninput`: a time input reports a half-typed time as empty, and re-rendering
    // the field with that empty value wipes what is being typed before it can be finished
    let on_pad_time_change = |current: NaiveTime, set_time: fn(&mut TimePadding, NaiveTime)| {
        let settings = settings.clone();
        Callback::from(move |e: Event| {
            // an unparseable value means the field was cleared, which leaves the time as it was
            let Ok(time) = NaiveTime::parse_from_str(&input_value(&e), TIME_INPUT_FORMAT) else {
                // puts the kept time back in the field the user just emptied
                return e.target_unchecked_into::<HtmlInputElement>().set_value(&time_input_value(current));
            };
            let mut updated = (*settings).clone();
            set_time(&mut updated.time_padding, time);
            settings.set(updated);
        })
    };

    let on_toggle_weekday = update(&settings, |settings: &mut VisualSettings, day: Weekday| settings.weekday_override.toggle_weekday(day));
    let set_only_lessons = |only_lessons: bool| update(&settings, move |settings: &mut VisualSettings, _: MouseEvent| settings.weekday_override.only_lessons = only_lessons);

    let (time_padding, weekdays) = (&settings.time_padding, &settings.weekday_override);

    html! {
        <SettingsCard title="Visual Settings">
            <form onsubmit={Callback::from(|e: SubmitEvent| e.prevent_default())}>
                <Checkbox id="asciiCheck" label="Force ASCII Timetable" class="mb-3 form-check"
                          checked={settings.force_ascii_timetable}
                          on_toggle={update(&settings, |settings: &mut VisualSettings, _| settings.force_ascii_timetable = !settings.force_ascii_timetable)} />
                { divider() }
                <div class="mb-3">
                    <Checkbox id="padEnabled" label="Always Shown Time Range" class="form-check mb-1" label_class="form-check-label fw-bold small text-light"
                              checked={time_padding.enabled}
                              on_toggle={update(&settings, |settings: &mut VisualSettings, _| settings.time_padding.enabled = !settings.time_padding.enabled)} />
                    <div class="form-text text-secondary small mb-2">
                        {"Draws the timetable over at least this range, even where no lessons are. Lessons outside it are still shown."}
                    </div>
                    <div class="d-flex flex-wrap align-items-center gap-2 ps-4" style={dimmed_unless(time_padding.enabled)}>
                        <label class="small text-secondary" for="padFrom">{"From"}</label>
                        <input type="time" class="form-control form-control-sm bg-dark text-white border-secondary w-auto"
                               id="padFrom" value={time_input_value(time_padding.from)}
                               onchange={on_pad_time_change(time_padding.from, |padding, time| padding.from = time)} />
                        <label class="small text-secondary" for="padUntil">{"until"}</label>
                        <input type="time" class="form-control form-control-sm bg-dark text-white border-secondary w-auto"
                               id="padUntil" value={time_input_value(time_padding.until)}
                               onchange={on_pad_time_change(time_padding.until, |padding, time| padding.until = time)} />
                    </div>
                </div>
                { divider() }
                <div class="mb-3">
                    <label class="form-label fw-bold small text-light mb-2">{"Weekday Override"}</label>
                    <div class="form-check mb-2">
                        <input class="form-check-input" type="radio" name="weekdayOverrideMode" id="modeAlwaysShow" checked={!weekdays.only_lessons} onclick={set_only_lessons(false)} />
                        <label class="form-check-label small text-secondary" for="modeAlwaysShow" style="cursor: pointer;">{"Always show selected weekdays (even if empty)"}</label>
                    </div>
                    <div class="ps-4 mb-3" style={dimmed_unless(!weekdays.only_lessons)}>
                        <div class="btn-group w-100 mb-2" role="group" aria-label="Weekday selector">
                            { for ALL_WEEKDAYS.into_iter().map(|day| {
                                let button_class = if weekdays.is_always_shown(day) { "btn-primary" } else { "btn-outline-secondary" };
                                html! {
                                    <button type="button" class={classes!("btn", "btn-sm", button_class)} onclick={on_toggle_weekday.reform(move |_| day)} disabled={weekdays.only_lessons}>
                                        { day.to_string() }
                                    </button>
                                }
                            })}
                        </div>
                    </div>
                    <div class="form-check mb-2">
                        <input class="form-check-input" type="radio" name="weekdayOverrideMode" id="modeOnlyLessons" checked={weekdays.only_lessons} onclick={set_only_lessons(true)} />
                        <label class="form-check-label small text-secondary" for="modeOnlyLessons" style="cursor: pointer;">{"Only show days with scheduled lessons"}</label>
                    </div>
                </div>
                { divider() }
                <LessonTextSection
                    settings={settings.lesson_text.clone()}
                    on_change={update(&settings, |settings: &mut VisualSettings, text: LessonTextSettings| settings.lesson_text = text)}
                />
                { divider() }
                <LessonDisplaySection
                    settings={settings.lesson_display.clone()}
                    on_change={update(&settings, |settings: &mut VisualSettings, display: LessonDisplaySettings| settings.lesson_display = display)}
                />
                { divider() }
                <SubjectColors
                    colors={settings.subject_color_overrides.clone()}
                    on_set={update(&settings, |settings: &mut VisualSettings, (subject, color): (String, String)| settings.set_subject_color(&subject, color))}
                    on_remove={update(&settings, |settings: &mut VisualSettings, subject: String| { settings.subject_color_overrides.remove(&subject); })}
                />
                <div class="text-end">
                    { autosaved_badge() }
                </div>
            </form>
        </SettingsCard>
    }
}
