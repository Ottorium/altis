use crate::components::settings::settings_card::SettingsCard;
use crate::persistence_manager::{ALL_WEEKDAYS, VisualSettings};
use chrono::Weekday;
use yew::{Callback, Html, Properties, classes, function_component, html, use_effect_with, use_state};

#[derive(Properties, PartialEq)]
pub struct VisualCardProps {
    pub initial: VisualSettings,
    pub on_save: Callback<VisualSettings>,
}

#[function_component(VisualSettingsCard)]
pub fn visual_settings_card(props: &VisualCardProps) -> Html {
    let force_ascii_timetable = use_state(|| props.initial.force_ascii_timetable);
    let weekday_override = use_state(|| props.initial.weekday_override.clone());

    let on_save = props.on_save.clone();
    let settings = VisualSettings { force_ascii_timetable: *force_ascii_timetable, weekday_override: (*weekday_override).clone() };
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
                <div class="text-end">
                    <span class="badge rounded-pill bg-success opacity-75" style="font-size: 0.7rem;">
                        {"Settings autosaved"}
                    </span>
                </div>
            </form>
        </SettingsCard>
    }
}
