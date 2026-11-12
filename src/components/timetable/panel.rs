//! What one week's panel of the timetable screen shows.

use super::compare::{CompareView, Compared, Comparison};
use super::free_rooms::{FreeRooms, FreeRoomsView};
use super::week_grid::WeekGrid;
use super::weeks::LoadedWeeks;
use crate::components::loading::Loading;
use altis_core::models::timetable::{Entity, EntityKind, WeekTimeTable};
use altis_core::settings::{Favorites, TimetableView, VisualSettings};
use altis_core::untis::week::Week;
use std::collections::HashMap;
use yew::prelude::*;

pub enum Panel {
    Loading,
    Error(String),
    Table(WeekTimeTable),
    /// the rooms no lesson takes up, for the whole week at once
    FreeRooms(FreeRooms),
    /// several timetables side by side
    Compare(Comparison),
    /// fewer than two timetables are picked for comparing
    CompareHint,
    NoSelection,
}

/// What the screen shows for a week: the panel, and for a view of one entity the entities there
/// are to pick from and the one that is picked
pub struct Resolved {
    pub names: Vec<String>,
    pub active_name: Option<String>,
    pub panel: Panel,
}

impl Resolved {
    fn panel_only(panel: Panel) -> Self {
        Self { names: vec![], active_name: None, panel }
    }
}

/// What decides the content of a week's panel, apart from the week itself
pub struct Selection<'a> {
    pub view: TimetableView,
    /// the class, teacher or room picked by hand, `None` to go with the default
    pub selected_name: &'a Option<String>,
    pub favorites: &'a Favorites,
    pub compared: &'a [Compared],
    pub visual: &'a VisualSettings,
}

/// Puts the compared timetables of a week side by side, once every one of them is loaded
fn compare_panel(loaded: &LoadedWeeks, week: &Week, selection: &Selection) -> Panel {
    let compared = selection.compared;
    if compared.len() < 2 {
        return Panel::CompareHint;
    }

    let mine = match compared.contains(&Compared::Me).then(|| loaded.mine(week)) {
        Some(None) => return Panel::Loading,
        Some(Some(Err(err))) => return Panel::Error(err.to_string()),
        Some(Some(Ok(mine))) => Some(mine),
        None => None,
    };
    let all = match compared.iter().any(|item| *item != Compared::Me).then(|| loaded.all(week)) {
        Some(None) => return Panel::Loading,
        Some(Some(Err(err))) => return Panel::Error(err.to_string()),
        Some(Some(Ok((timetables, _)))) => Some(timetables),
        None => None,
    };

    let mut tables = vec![];
    let mut missing = vec![];
    for item in compared {
        let table = match item {
            Compared::Me => mine.map(|mine| &mine.timetable),
            Compared::Entity { kind, name } => all.and_then(|timetables| {
                timetables.iter().find(|(entity, _)| entity.kind() == Some(*kind) && entity.name() == *name).map(|(_, table)| table)
            }),
        };
        match table {
            Some(table) => tables.push((item.label(), table)),
            None => missing.push(item.label()),
        }
    }
    Panel::Compare(Comparison::of(&tables, missing, selection.visual))
}

/// The timetable of one class, teacher or room, and the others of its kind to pick from
fn entity_panel(timetables: &HashMap<Entity, WeekTimeTable>, own_class_id: Option<i32>, kind: EntityKind, selection: &Selection) -> Resolved {
    let mut of_kind: Vec<(&Entity, &WeekTimeTable)> = timetables.iter().filter(|(entity, _)| entity.kind() == Some(kind)).collect();
    of_kind.sort_by_key(|(entity, _)| entity.name());

    // what is shown unless something was picked by hand: the student's own class, or the first favourite
    let own_class = (kind == EntityKind::Class)
        .then(|| timetables.keys().find(|entity| matches!(entity, Entity::Class(class) if Some(class.id) == own_class_id)))
        .flatten()
        .map(Entity::name);
    let first_favorite = of_kind.iter().map(|(entity, _)| entity.name()).find(|name| selection.favorites.contains(kind, name));
    let wanted_name = selection.selected_name.clone().or(own_class).or(first_favorite);

    let active = of_kind.iter().find(|(entity, _)| Some(entity.name()) == wanted_name).or(of_kind.first());

    Resolved {
        names: of_kind.iter().map(|(entity, _)| entity.name()).collect(),
        active_name: active.map(|(entity, _)| entity.name()),
        panel: active.map_or(Panel::NoSelection, |(_, table)| Panel::Table((*table).clone())),
    }
}

/// Picks what to show for a week
pub fn resolve(loaded: &LoadedWeeks, week: &Week, selection: &Selection) -> Resolved {
    match selection.view {
        TimetableView::Compare => return Resolved::panel_only(compare_panel(loaded, week, selection)),
        TimetableView::Me => {
            return match loaded.mine(week) {
                None => Resolved::panel_only(Panel::Loading),
                Some(Err(err)) => Resolved::panel_only(Panel::Error(err.to_string())),
                Some(Ok(mine)) => Resolved {
                    names: vec![mine.name.clone()],
                    active_name: Some(mine.name.clone()),
                    panel: Panel::Table(mine.timetable.clone()),
                },
            };
        }
        _ => {}
    }

    let (timetables, own_class_id) = match loaded.all(week) {
        None => return Resolved::panel_only(Panel::Loading),
        Some(Err(err)) => return Resolved::panel_only(Panel::Error(err.to_string())),
        Some(Ok(all)) => all,
    };

    match selection.view.entity_kind() {
        Some(kind) => entity_panel(timetables, *own_class_id, kind, selection),
        // the free rooms are the week as a whole, there is nothing to pick an entity from
        None => Resolved::panel_only(Panel::FreeRooms(FreeRooms::of(timetables, &selection.visual.weekday_override))),
    }
}

/// A week's panel on the track, `left` being where on it: the week on screen, or one beside it
pub fn render_panel(week: &Week, left: &str, panel: Panel, visual: &VisualSettings, on_entity_select: Callback<Entity>) -> Html {
    html! {
        <div key={week.start.to_string()} class="d-flex flex-column position-absolute top-0 h-100 w-100" style={format!("left: {left}; overflow-y: auto;")}>
            { match panel {
                Panel::Loading => html! { <Loading /> },
                Panel::Error(err) => html! { <div class="alert alert-danger m-3">{ err }</div> },
                Panel::Table(timetable) => html! { <WeekGrid {timetable} visual={visual.clone()} {on_entity_select} /> },
                Panel::FreeRooms(free_rooms) => html! { <FreeRoomsView {free_rooms} show_now_line={visual.lesson_display.show_now_line} {on_entity_select} /> },
                Panel::Compare(comparison) => html! { <CompareView {comparison} visual={visual.clone()} /> },
                Panel::CompareHint => html! {
                    <p class="text-secondary p-3">{ "Pick two or more timetables to compare." }</p>
                },
                Panel::NoSelection => html! { <p class="text-light"> {"No selection made"} </p> },
            }}
        </div>
    }
}
