use super::card::{autosaved_badge, divider, SettingsCard};
use crate::components::forms::{select_value, update, Checkbox};
use altis_core::settings::{describe_minutes, GeneralSettings, Tab, TimetableView};
use yew::prelude::*;

/// How long a loaded timetable can be kept, in minutes: from five minutes to a day
const CACHE_DURATIONS: [u32; 7] = [5, 15, 30, 60, 180, 720, 1440];

const SELECT_CLASS: &str = "form-select form-select-sm bg-dark text-light border-secondary";

#[derive(Properties, PartialEq)]
pub struct GeneralCardProps {
    pub initial: GeneralSettings,
    pub on_save: Callback<GeneralSettings>,
}

/// Where the app starts, what its navigation offers and how long it keeps what it loaded.
/// Stored as it is changed.
#[function_component(GeneralSettingsCard)]
pub fn general_settings_card(props: &GeneralCardProps) -> Html {
    let settings = use_state(|| props.initial.clone());

    {
        let on_save = props.on_save.clone();
        use_effect_with((*settings).clone(), move |settings| {
            on_save.emit(settings.clone());
            || ()
        });
    }

    let on_start_tab_change = update(&settings, |settings: &mut GeneralSettings, e: Event| {
        let picked = select_value(&e);
        if let Some(tab) = Tab::ALL.into_iter().find(|tab| tab.label() == picked) {
            settings.start_tab = tab;
        }
    });
    let on_start_view_change = update(&settings, |settings: &mut GeneralSettings, e: Event| {
        if let Some(view) = TimetableView::from_key(&select_value(&e)) {
            settings.start_view = view;
        }
    });
    let on_toggle_tab = update(&settings, |settings: &mut GeneralSettings, tab: Tab| settings.toggle_tab(tab));
    let on_cache_change = update(&settings, |settings: &mut GeneralSettings, e: Event| {
        if let Ok(minutes) = select_value(&e).parse() {
            settings.timetable_cache_minutes = minutes;
        }
    });

    // a duration set by hand, in an import for instance, is offered along with the usual ones
    let mut cache_durations = CACHE_DURATIONS.to_vec();
    if !cache_durations.contains(&settings.timetable_cache_minutes) {
        cache_durations.push(settings.timetable_cache_minutes);
        cache_durations.sort();
    }

    html! {
        <SettingsCard title="General">
            <form onsubmit={Callback::from(|e: SubmitEvent| e.prevent_default())}>
                <div class="mb-3">
                    <label class="form-label small text-secondary mb-1" for="startTab">{"Open the app on"}</label>
                    <select id="startTab" class={SELECT_CLASS} style="width: auto;" onchange={on_start_tab_change}>
                        { for settings.shown_tabs().into_iter().map(|tab| html! {
                            <option value={tab.label()} selected={settings.initial_tab() == tab}>{ tab.label() }</option>
                        })}
                    </select>
                </div>
                <div class="mb-3">
                    <label class="form-label small text-secondary mb-1" for="startView">{"Open the timetable on"}</label>
                    <select id="startView" class={SELECT_CLASS} style="width: auto;" onchange={on_start_view_change}>
                        { for TimetableView::ALL.into_iter().map(|view| html! {
                            <option value={view.key()} selected={settings.start_view == view}>{ view.label() }</option>
                        })}
                    </select>
                </div>

                { divider() }

                <div class="mb-3">
                    <label class="form-label fw-bold small text-light mb-1">{"Tabs"}</label>
                    <div class="form-text text-secondary small mb-2">
                        {"The tabs the navigation offers. The settings always stay."}
                    </div>
                    <div class="btn-group w-100" role="group" aria-label="Tabs">
                        { for Tab::ALL.into_iter().filter(|tab| tab.can_be_hidden()).map(|tab| {
                            let button_class = if settings.is_tab_shown(tab) { "btn-primary" } else { "btn-outline-secondary" };
                            html! {
                                <button type="button" class={classes!("btn", "btn-sm", button_class)} onclick={on_toggle_tab.reform(move |_| tab)}>
                                    { tab.label() }
                                </button>
                            }
                        })}
                    </div>
                </div>
                <Checkbox id="weekSwipe" label="Swipe the timetable to change the week" class="mb-3 form-check"
                          checked={settings.week_swipe}
                          on_toggle={update(&settings, |settings: &mut GeneralSettings, _| settings.week_swipe = !settings.week_swipe)} />

                { divider() }

                <div class="mb-3">
                    <label class="form-label small text-secondary mb-1" for="cacheDuration">{"Keep loaded timetables for"}</label>
                    <select id="cacheDuration" class={SELECT_CLASS} style="width: auto;" onchange={on_cache_change}>
                        { for cache_durations.into_iter().map(|minutes| html! {
                            <option value={minutes.to_string()} selected={settings.timetable_cache_minutes == minutes}>
                                { describe_minutes(i64::from(minutes)) }
                            </option>
                        })}
                    </select>
                    <div class="form-text text-secondary small">
                        {"After that Untis is asked again. Reload always asks right away."}
                    </div>
                </div>
                <Checkbox id="checkForUpdates" label="Check for new versions of Altis" class="mb-3 form-check"
                          checked={settings.check_for_updates}
                          on_toggle={update(&settings, |settings: &mut GeneralSettings, _| settings.check_for_updates = !settings.check_for_updates)}>
                    <div class="form-text small text-secondary">{"Asks GitHub once a day and says so in a banner when there is one."}</div>
                </Checkbox>

                <div class="text-end">
                    { autosaved_badge() }
                </div>
            </form>
        </SettingsCard>
    }
}
