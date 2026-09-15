use crate::components::settings::settings_card::SettingsCard;
use crate::persistence_manager::PersistenceManager;
use altis_core::settings::{ALL_WEEKDAYS, VisualSettings, is_dark_color};
use chrono::Weekday;
use web_sys::HtmlInputElement;
use yew::{
    Callback, Html, InputEvent, KeyboardEvent, MouseEvent, Properties, TargetCast, classes,
    function_component, html, use_effect_with, use_state,
};

#[derive(Properties, PartialEq)]
pub struct VisualCardProps {
    pub initial: VisualSettings,
    pub on_save: Callback<VisualSettings>,
}

#[function_component(VisualSettingsCard)]
pub fn visual_settings_card(props: &VisualCardProps) -> Html {
    let force_ascii_timetable = use_state(|| props.initial.force_ascii_timetable);
    let weekday_override = use_state(|| props.initial.weekday_override.clone());
    let subject_color_overrides = use_state(|| props.initial.subject_color_overrides.clone());

    let new_subject_name = use_state(String::new);
    let new_subject_color = use_state(|| "#4a90e2".to_string());
    let known_subjects = use_state(PersistenceManager::get_known_subjects);

    let on_save = props.on_save.clone();
    let settings = VisualSettings {
        force_ascii_timetable: *force_ascii_timetable,
        weekday_override: (*weekday_override).clone(),
        subject_color_overrides: (*subject_color_overrides).clone(),
    };
    use_effect_with(settings, move |settings| {
        on_save.emit(settings.clone());
        || ()
    });

    let on_toggle_ascii = {
        let force_ascii_timetable = force_ascii_timetable.clone();
        Callback::from(move |_| {
            force_ascii_timetable.set(!*force_ascii_timetable);
        })
    };

    let on_select_always_show = {
        let weekday_override = weekday_override.clone();
        Callback::from(move |_| {
            let mut updated = (*weekday_override).clone();
            updated.only_lessons = false;
            weekday_override.set(updated);
        })
    };
    let on_select_only_lessons = {
        let weekday_override = weekday_override.clone();
        Callback::from(move |_| {
            let mut updated = (*weekday_override).clone();
            updated.only_lessons = true;
            weekday_override.set(updated);
        })
    };
    let on_toggle_weekday = {
        let weekday_override = weekday_override.clone();
        Callback::from(move |day: Weekday| {
            let mut updated = (*weekday_override).clone();
            updated.toggle_weekday(day);
            weekday_override.set(updated);
        })
    };

    let on_new_subject_input = {
        let new_subject_name = new_subject_name.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            new_subject_name.set(input.value());
        })
    };

    let on_new_color_input = {
        let new_subject_color = new_subject_color.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            new_subject_color.set(input.value());
        })
    };

    let on_add_override = {
        let subject_color_overrides = subject_color_overrides.clone();
        let new_subject_name = new_subject_name.clone();
        let new_subject_color = new_subject_color.clone();
        Callback::from(move |_: ()| {
            let name = new_subject_name.trim().to_string();
            if !name.is_empty() {
                let mut updated = (*subject_color_overrides).clone();
                if let Some(existing) = updated.keys().find(|k| k.eq_ignore_ascii_case(&name)).cloned() {
                    updated.remove(&existing);
                }
                updated.insert(name, (*new_subject_color).clone());
                subject_color_overrides.set(updated);
                new_subject_name.set(String::new());
            }
        })
    };

    let on_add_click = {
        let on_add_override = on_add_override.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            on_add_override.emit(());
        })
    };

    let on_new_subject_keydown = {
        let on_add_override = on_add_override.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                e.prevent_default();
                on_add_override.emit(());
            }
        })
    };

    let on_remove_override = {
        let subject_color_overrides = subject_color_overrides.clone();
        Callback::from(move |subj: String| {
            let mut updated = (*subject_color_overrides).clone();
            updated.remove(&subj);
            subject_color_overrides.set(updated);
        })
    };

    let on_change_override_color = {
        let subject_color_overrides = subject_color_overrides.clone();
        Callback::from(move |(subj, new_color): (String, String)| {
            let mut updated = (*subject_color_overrides).clone();
            updated.insert(subj, new_color);
            subject_color_overrides.set(updated);
        })
    };

    let available_suggestions: Vec<_> = known_subjects
        .iter()
        .filter(|s| !subject_color_overrides.iter().any(|(k, _)| k.eq_ignore_ascii_case(s)))
        .cloned()
        .collect();

    html! {
        <SettingsCard title="Visual Settings">
            <form onsubmit={Callback::from(|e: yew::prelude::SubmitEvent| e.prevent_default())}>
                <div class="mb-3 form-check">
                    <input
                        type="checkbox"
                        class="form-check-input"
                        id="asciiCheck"
                        checked={*force_ascii_timetable}
                        onclick={on_toggle_ascii}
                    />
                    <label class="form-check-label small text-secondary" for="asciiCheck" style="cursor: pointer;">
                        {"Force ASCII Timetable"}
                    </label>
                </div>
                <hr class="border-secondary opacity-25 my-3" />
                <div class="mb-3">
                    <label class="form-label fw-bold small text-light mb-2">{"Weekday Override"}</label>
                    <div class="form-check mb-2">
                        <input class="form-check-input" type="radio" name="weekdayOverrideMode" id="modeAlwaysShow" checked={!weekday_override.only_lessons} onclick={on_select_always_show} />
                        <label class="form-check-label small text-secondary" for="modeAlwaysShow" style="cursor: pointer;">{"Always show selected weekdays (even if empty)"}</label>
                    </div>
                    <div class="ps-4 mb-3" style={if weekday_override.only_lessons { "opacity: 0.4; pointer-events: none;" } else { "" }}>
                        <div class="btn-group w-100 mb-2" role="group" aria-label="Weekday selector">
                            { for ALL_WEEKDAYS.iter().map(|&day| {
                                let label = match day { Weekday::Mon => "Mon", Weekday::Tue => "Tue", Weekday::Wed => "Wed", Weekday::Thu => "Thu", Weekday::Fri => "Fri", Weekday::Sat => "Sat", Weekday::Sun => "Sun" };
                                let is_selected = weekday_override.is_weekday_always_shown(day);
                                let onclick = { let on_toggle_weekday = on_toggle_weekday.clone(); Callback::from(move |_| on_toggle_weekday.emit(day)) };
                                html! { <button type="button" class={classes!("btn", "btn-sm", if is_selected { "btn-primary" } else { "btn-outline-secondary" })} {onclick} disabled={weekday_override.only_lessons}>{label}</button> }
                            })}
                        </div>
                    </div>
                    <div class="form-check mb-2">
                        <input class="form-check-input" type="radio" name="weekdayOverrideMode" id="modeOnlyLessons" checked={weekday_override.only_lessons} onclick={on_select_only_lessons} />
                        <label class="form-check-label small text-secondary" for="modeOnlyLessons" style="cursor: pointer;">{"Only show days with scheduled lessons"}</label>
                    </div>
                </div>
                <hr class="border-secondary opacity-25 my-3" />
                <div class="mb-3">
                    <label class="form-label fw-bold small text-light mb-1">{"Subject Colour Overrides"}</label>
                    <div class="form-text text-secondary small mb-3">
                        {"Set custom background colours for specific subjects in the timetable."}
                    </div>

                    if subject_color_overrides.is_empty() {
                        <div class="text-secondary small fst-italic mb-3">
                            {"No subject colour overrides configured."}
                        </div>
                    } else {
                        <div class="d-flex flex-column gap-2 mb-3">
                            { for subject_color_overrides.iter().map(|(subj, color)| {
                                let s_remove = subj.clone();
                                let on_remove = {
                                    let on_remove_override = on_remove_override.clone();
                                    Callback::from(move |_| on_remove_override.emit(s_remove.clone()))
                                };
                                let s_change = subj.clone();
                                let on_color_input = {
                                    let on_change_override_color = on_change_override_color.clone();
                                    Callback::from(move |e: InputEvent| {
                                        let input: HtmlInputElement = e.target_unchecked_into();
                                        on_change_override_color.emit((s_change.clone(), input.value()));
                                    })
                                };
                                let badge_text_cls = if is_dark_color(color) { "text-white" } else { "text-black" };

                                html! {
                                    <div class="d-flex align-items-center justify-content-between p-2 rounded bg-dark border border-secondary border-opacity-25" key={subj.clone()}>
                                        <div class="d-flex align-items-center gap-2">
                                            <span
                                                class={classes!("badge", "fs-6", "fw-bold", badge_text_cls)}
                                                style={format!("background-color: {color}; min-width: 50px; text-align: center; border: 1px solid rgba(255,255,255,0.15);")}
                                            >
                                                { subj }
                                            </span>
                                            <span class="text-secondary small font-monospace">{ color }</span>
                                        </div>
                                        <div class="d-flex align-items-center gap-2">
                                            <input
                                                type="color"
                                                class="form-control form-control-color form-control-sm border-0 p-0"
                                                value={color.clone()}
                                                oninput={on_color_input}
                                                title={format!("Change colour for {subj}")}
                                                style="cursor: pointer; width: 36px; height: 32px; background: transparent;"
                                            />
                                            <button
                                                type="button"
                                                class="btn btn-outline-danger btn-sm px-2 py-1"
                                                onclick={on_remove}
                                                title={format!("Remove override for {subj}")}
                                            >
                                                <i class="bi bi-trash"></i>
                                            </button>
                                        </div>
                                    </div>
                                }
                            })}
                        </div>
                    }

                    <div class="card bg-black bg-opacity-25 border-secondary border-opacity-25 p-2">
                        <div class="d-flex gap-2 align-items-center">
                            <input
                                type="text"
                                class="form-control form-control-sm bg-dark text-light border-secondary"
                                placeholder="Subject (e.g. M, D, ENG)"
                                value={(*new_subject_name).clone()}
                                oninput={on_new_subject_input}
                                onkeydown={on_new_subject_keydown}
                                list="known-subjects-list"
                            />
                            <datalist id="known-subjects-list">
                                { for available_suggestions.iter().map(|subj| {
                                    html! { <option value={subj.clone()} /> }
                                })}
                            </datalist>
                            <input
                                type="color"
                                class="form-control form-control-color form-control-sm border-0 p-0"
                                value={(*new_subject_color).clone()}
                                oninput={on_new_color_input}
                                title="Choose colour"
                                style="cursor: pointer; width: 38px; height: 32px; background: transparent; flex-shrink: 0;"
                            />
                            <button
                                type="button"
                                class="btn btn-sm btn-primary flex-shrink-0"
                                onclick={on_add_click}
                                disabled={new_subject_name.trim().is_empty()}
                            >
                                <i class="bi bi-plus-lg me-1"></i>{"Add"}
                            </button>
                        </div>
                    </div>
                </div>
                <div class="text-end">
                    <span class="badge rounded-pill bg-success opacity-75" style="font-size: 0.7rem;">
                        {"Settings autosaved"}
                    </span>
                </div>
            </form>
        </SettingsCard>
    }
}
