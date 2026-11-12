use super::compare::{ComparePicker, Compared};
use super::controls::TimetableControls;
use super::panel::{render_panel, resolve, Selection};
use super::swipe::use_week_swipe;
use super::weeks::{use_loaded_weeks, LoadedAction, LoadedWeeks};
use crate::storage;
use crate::untis::cached_client::CachedUntisClient;
use altis_core::models::timetable::{Entity, EntityKind};
use altis_core::settings::TimetableView;
use altis_core::untis::week::Week;
use yew::prelude::*;

/// The names there are to compare per kind of entity, empty while the week is still loading
fn compare_options(loaded: &LoadedWeeks, week: &Week) -> Vec<(EntityKind, Vec<String>)> {
    let Some(Ok((timetables, _))) = loaded.all(week) else { return vec![] };

    EntityKind::ALL
        .into_iter()
        .map(|kind| {
            let mut names: Vec<String> =
                timetables.keys().filter(|entity| entity.kind() == Some(kind)).map(Entity::name).filter(|name| !name.is_empty()).collect();
            names.sort();
            names.dedup();
            (kind, names)
        })
        .collect()
}

/// The timetable screen: a week of the student's own lessons, of a class, a teacher or a room,
/// the free rooms, or several timetables compared - with the neighbouring weeks a swipe away
#[function_component(TimetablePage)]
pub fn timetable_page() -> Html {
    let settings = storage::settings_or_default();
    let reload_trigger = use_state(|| 0u32);
    let view = use_state(|| settings.general.start_view);
    let selected_name = use_state(|| None::<String>);
    let selected_week = use_state(Week::current);
    let favorites = use_state(|| settings.favorites.clone());
    let compared = use_state(storage::compared);
    let picker_open = use_state(|| false);

    let loaded = use_loaded_weeks(&selected_week, *view, *reload_trigger);
    let swipe = use_week_swipe(&selected_week, settings.general.week_swipe);

    let visual = settings.visual_settings;
    let selection = Selection { view: *view, selected_name: &selected_name, favorites: &favorites, compared: &compared, visual: &visual };
    let (previous_week, next_week) = (selected_week.previous(), selected_week.next());
    let current = resolve(&loaded, &selected_week, &selection);
    let previous_panel = resolve(&loaded, &previous_week, &selection).panel;
    let next_panel = resolve(&loaded, &next_week, &selection).panel;

    // "Me" never loads the class/teacher/room tables, so its spinner follows the personal
    // timetable instead - otherwise the controls would sit on "Loading..." forever
    let loading = match *view {
        TimetableView::Me => loaded.mine(&selected_week).is_none(),
        _ => loaded.all(&selected_week).is_none(),
    };

    let on_view_change = {
        let view = view.clone();
        let selected_name = selected_name.clone();
        Callback::from(move |new_view| {
            view.set(new_view);
            selected_name.set(None);
        })
    };

    let on_reload = {
        let reload_trigger = reload_trigger.clone();
        let selected_name = selected_name.clone();
        let dispatch = loaded.dispatcher();
        Callback::from(move |_| {
            let _ = CachedUntisClient::clear_cache();
            dispatch.dispatch(LoadedAction::Clear);
            selected_name.set(None);
            reload_trigger.set(*reload_trigger + 1);
        })
    };

    let on_entity_change = {
        let selected_name = selected_name.clone();
        Callback::from(move |name| selected_name.set(Some(name)))
    };

    // a teacher, class or room picked in a lesson's details or among the free rooms
    let on_entity_select = {
        let view = view.clone();
        let selected_name = selected_name.clone();
        Callback::from(move |entity: Entity| {
            let Some(kind) = entity.kind() else { return };
            view.set(TimetableView::from(kind));
            selected_name.set(Some(entity.name()));
        })
    };

    let on_toggle_favorite = {
        let favorites = favorites.clone();
        let kind = view.entity_kind();
        Callback::from(move |name: String| {
            let Some(kind) = kind else { return };
            let mut settings = storage::settings_or_default();
            settings.favorites.toggle(kind, &name);
            if storage::save_settings(&settings).is_ok() {
                favorites.set(settings.favorites);
            }
        })
    };

    let on_compared_change = {
        let compared = compared.clone();
        Callback::from(move |next: Vec<Compared>| {
            let _ = storage::save_compared(&next);
            compared.set(next);
        })
    };

    let set_picker_open = |open: bool| {
        let picker_open = picker_open.clone();
        Callback::from(move |_| picker_open.set(open))
    };

    let on_week_change = {
        let selected_week = selected_week.clone();
        Callback::from(move |week| selected_week.set(week))
    };

    html! {
        <div class="d-flex flex-column flex-grow-1 h-100">
            <TimetableControls
                view={*view}
                selected_name={current.active_name}
                selected_week={(*selected_week).clone()}
                names={current.names}
                favorites={(*favorites).clone()}
                compared={(*compared).clone()}
                {loading}
                {on_view_change}
                {on_entity_change}
                {on_week_change}
                {on_reload}
                {on_toggle_favorite}
                on_open_picker={set_picker_open(true)}
            />
            if *picker_open {
                <ComparePicker
                    options={compare_options(&loaded, &selected_week)}
                    favorites={(*favorites).clone()}
                    compared={(*compared).clone()}
                    on_change={on_compared_change}
                    on_close={set_picker_open(false)}
                />
            }
            <div class="flex-grow-1 w-100 position-relative overflow-hidden">
                <div
                    ref={swipe.track}
                    class="week-track position-absolute top-0 start-0 h-100 w-100"
                    style="user-select: none;"
                    onpointerdown={swipe.onpointerdown}
                    onpointermove={swipe.onpointermove}
                    onpointerup={swipe.onpointerup}
                    onpointercancel={swipe.onpointercancel}
                >
                    { render_panel(&previous_week, "-100%", previous_panel, &visual, on_entity_select.clone()) }
                    { render_panel(&selected_week, "0", current.panel, &visual, on_entity_select.clone()) }
                    { render_panel(&next_week, "100%", next_panel, &visual, on_entity_select) }
                </div>
            </div>
        </div>
    }
}
