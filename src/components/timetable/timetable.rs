use crate::components::loading::LoadingComponent;
use crate::components::timetable::timetable_controls::TimetableControls;
use crate::components::timetable::timetable_render::TimeTableRender;
use crate::untis::cached_untis_client::{AllTimeTables, CachedUntisClient};
use altis_core::data_models::clean_models::untis::{Entity, MyTimeTable, WeekTimeTable};
use altis_core::errors::ApiError;
use altis_core::untis::untis_week::Week;
use chrono::{Local, NaiveDateTime};
use gloo_timers::callback::Timeout;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::{Closure, JsValue};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Element, HtmlElement};
use yew::prelude::*;

const SWIPE_MS: u32 = 250;

/// Resolves once the browser has rendered the next frame
async fn next_paint() {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let Some(window) = web_sys::window() else {
            let _ = resolve.call0(&JsValue::NULL);
            return;
        };
        // animation frame callbacks run right before the paint, so a timeout scheduled from one runs right after it
        let after_frame = Closure::once_into_js(move || {
            if let Some(window) = web_sys::window() {
                let _ = window.set_timeout_with_callback(&resolve);
            }
        });
        let _ = window.request_animation_frame(after_frame.unchecked_ref());
    });
    let _ = JsFuture::from(promise).await;
}

/// A loaded timetable along with the time it expires
type Entry<T> = (NaiveDateTime, Rc<Result<T, ApiError>>);

fn entry<T>(res: Result<(NaiveDateTime, T), ApiError>) -> Entry<T> {
    match res {
        Ok((expiry, value)) => (expiry, Rc::new(Ok(value))),
        // expire errors right away, so they are retried the next time the week is shown
        Err(e) => (Local::now().naive_local(), Rc::new(Err(e))),
    }
}

/// loaded successfully and not expired yet
fn is_fresh<T>(entry: Option<&Entry<T>>) -> bool {
    entry.is_some_and(|(expiry, res)| res.is_ok() && *expiry > Local::now().naive_local())
}

/// Timetables of the displayed week and its neighbours, so swiping can show them right away
#[derive(Default, Clone)]
struct LoadedWeeks {
    mine: HashMap<Week, Entry<MyTimeTable>>,
    all: HashMap<Week, Entry<AllTimeTables>>,
}

impl LoadedWeeks {
    fn get_mine(&self, week: &Week) -> Option<&Result<MyTimeTable, ApiError>> {
        self.mine.get(week).map(|(_, r)| &**r)
    }

    fn get_all(&self, week: &Week) -> Option<&Result<AllTimeTables, ApiError>> {
        self.all.get(week).map(|(_, r)| &**r)
    }
}

enum LoadedAction {
    Mine(Week, Result<(NaiveDateTime, MyTimeTable), ApiError>),
    All(Week, Result<(NaiveDateTime, AllTimeTables), ApiError>),
    /// drops all other weeks, so only the visible ones are kept in memory
    Retain(Vec<Week>),
    Clear,
}

impl Reducible for LoadedWeeks {
    type Action = LoadedAction;

    fn reduce(self: Rc<Self>, action: LoadedAction) -> Rc<Self> {
        let mut next = (*self).clone();
        match action {
            LoadedAction::Mine(week, res) => { next.mine.insert(week, entry(res)); }
            LoadedAction::All(week, res) => { next.all.insert(week, entry(res)); }
            LoadedAction::Retain(weeks) => {
                next.mine.retain(|w, _| weeks.contains(w));
                next.all.retain(|w, _| weeks.contains(w));
            }
            LoadedAction::Clear => next = Self::default(),
        }
        Rc::new(next)
    }
}

enum Panel {
    Loading,
    Error(String),
    Table(WeekTimeTable),
    NoSelection,
}

struct Resolved {
    names: Vec<String>,
    active_name: Option<String>,
    panel: Panel,
}

/// Picks what to show for a week, given the selected category and entity
fn resolve(loaded: &LoadedWeeks, week: &Week, category: &str, selected_name: &Option<String>) -> Resolved {
    let without_names = |panel| Resolved { names: vec![], active_name: None, panel };

    if category == "Me" {
        return match loaded.get_mine(week) {
            None => without_names(Panel::Loading),
            Some(Err(err)) => without_names(Panel::Error(err.to_string())),
            Some(Ok(mine)) => Resolved {
                names: vec![mine.name.clone()],
                active_name: Some(mine.name.clone()),
                panel: Panel::Table(mine.timetable.clone()),
            },
        };
    }

    match loaded.get_all(week) {
        None => without_names(Panel::Loading),
        Some(Err(err)) => without_names(Panel::Error(err.to_string())),
        Some(Ok((map, initial_id))) => {
            let mut filtered_data: Vec<(&Entity, &WeekTimeTable)> = map.iter()
                .filter(|(entity, _)| match (category, entity) {
                    ("Class", Entity::Class(_)) => true,
                    ("Teacher", Entity::Teacher(_)) => true,
                    ("Room", Entity::Room(_)) => true,
                    _ => false,
                })
                .collect();
            filtered_data.sort_by_key(|(e, _)| e.name());

            let initial_class = (category == "Class")
                .then(|| map.keys().find(|e| matches!(e, Entity::Class(c) if Some(c.id) == *initial_id)))
                .flatten()
                .map(|e| e.name());
            let wanted_name = selected_name.clone().or(initial_class);

            let active = filtered_data.iter()
                .find(|(e, _)| Some(e.name()) == wanted_name)
                .or(filtered_data.first());

            Resolved {
                names: filtered_data.iter().map(|(e, _)| e.name()).collect(),
                active_name: active.map(|(e, _)| e.name()),
                panel: active.map_or(Panel::NoSelection, |(_, t)| Panel::Table((*t).clone())),
            }
        }
    }
}

fn render_panel(week: &Week, left: &str, panel: Panel, on_entity_select: Callback<Entity>) -> Html {
    html! {
        <div key={week.start.clone()} class="d-flex flex-column position-absolute top-0 h-100 w-100" style={format!("left: {left}; overflow-y: auto;")}>
            { match panel {
                Panel::Loading => html! { <LoadingComponent /> },
                Panel::Error(err) => html! { <div class="alert alert-danger m-3">{ err }</div> },
                Panel::Table(tt) => html! { <TimeTableRender timetable={tt} {on_entity_select} /> },
                Panel::NoSelection => html! { <p class="text-light"> {"No selection made"} </p> },
            }}
        </div>
    }
}

struct Drag {
    pointer_id: i32,
    start_x: f64,
    offset: f64,
    last_x: f64,
    last_time: f64,
    /// px/ms of the latest movement
    velocity: f64,
    captured: bool,
}

#[derive(Default)]
struct Swipe {
    drag: Option<Drag>,
    /// the track is sliding into place, input is ignored until it's done
    animating: bool,
}

/// Moves the track holding the week panels, `None` puts it back to rest
fn move_track(track: &NodeRef, offset: Option<f64>, animate: bool) {
    let Some(el) = track.cast::<HtmlElement>() else { return };
    let style = el.style();
    let transition = if animate { format!("transform {SWIPE_MS}ms ease-out") } else { "none".to_string() };
    let _ = style.set_property("transition", &transition);
    match offset {
        Some(x) => { let _ = style.set_property("transform", &format!("translateX({x}px)")); }
        // any transform makes the track the containing block of the fixed lesson modal, so it's removed at rest
        None => { let _ = style.remove_property("transform"); }
    }
}

fn snap_back(track: &NodeRef, swipe: &Rc<RefCell<Swipe>>) {
    swipe.borrow_mut().animating = true;
    move_track(track, Some(0.0), true);
    let (track, swipe) = (track.clone(), swipe.clone());
    Timeout::new(SWIPE_MS, move || {
        move_track(&track, None, false);
        swipe.borrow_mut().animating = false;
    }).forget();
}

#[function_component(TimetableComponent)]
pub fn timetable() -> Html {
    let reload_trigger = use_state(|| 0);
    let category = use_state(|| "Me".to_string());
    let selected_name = use_state(|| None::<String>);
    let selected_week = use_state(Week::current);
    let loaded = use_reducer(LoadedWeeks::default);
    let track_ref = use_node_ref();
    let swipe = use_mut_ref(Swipe::default);

    {
        let dispatch = loaded.dispatcher();
        let loaded = loaded.clone();
        use_effect_with(((*selected_week).clone(), (*category).clone(), *reload_trigger), move |(week, category, _)| {
            let neighbours = [week.previous(), week.next()];
            dispatch.dispatch(LoadedAction::Retain(vec![neighbours[0].clone(), week.clone(), neighbours[1].clone()]));

            // the personal timetable is its own request; the class, teacher and room tables cost
            // one request per class and a full regeneration on top, so they are only loaded once
            // something actually shows them. Loading them for "Me" is what froze a swipe to a
            // week that wasn't cached yet.
            let wants_all = category.as_str() != "Me";

            let needs_mine = !is_fresh(loaded.mine.get(week));
            let needs_all = wants_all && !is_fresh(loaded.all.get(week));
            let mine_missing: Vec<Week> = neighbours.iter().filter(|w| !is_fresh(loaded.mine.get(w))).cloned().collect();
            let all_missing: Vec<Week> = if wants_all {
                neighbours.iter().filter(|w| !is_fresh(loaded.all.get(w))).cloned().collect()
            } else {
                Vec::new()
            };

            // set when the week changes or the component unmounts, so no further loading is started
            let cancelled = Rc::new(Cell::new(false));
            let is_cancelled = cancelled.clone();
            let week = week.clone();
            spawn_local(async move {
                let client = match CachedUntisClient::new() {
                    Ok(client) => client,
                    Err(e) => {
                        dispatch.dispatch(LoadedAction::Mine(week.clone(), Err(e.clone())));
                        dispatch.dispatch(LoadedAction::All(week, Err(e)));
                        return;
                    }
                };

                if needs_mine {
                    let mine = client.get_my_timetable(week.clone()).await;
                    dispatch.dispatch(LoadedAction::Mine(week.clone(), mine));
                }
                // neighbouring weeks are only taken from the cache, loading them happens once they're swiped to
                for (w, expiry, mine) in CachedUntisClient::cached_my_timetables(&mine_missing) {
                    dispatch.dispatch(LoadedAction::Mine(w, Ok((expiry, mine))));
                }

                // the rest blocks the main thread for a while (reading the cache, generating the teacher/room
                // tables) so it may only start once the personal timetable is actually on screen
                next_paint().await;
                if is_cancelled.get() { return; }
                if needs_all {
                    let all = client.get_all_timetables(week.clone()).await;
                    dispatch.dispatch(LoadedAction::All(week, all));
                    next_paint().await;
                    if is_cancelled.get() { return; }
                }
                for (w, all) in CachedUntisClient::cached_all_timetables(&all_missing) {
                    dispatch.dispatch(LoadedAction::All(w, all));
                }
            });

            move || cancelled.set(true)
        });
    }

    {
        let track_ref = track_ref.clone();
        let swipe = swipe.clone();
        use_effect_with((*selected_week).clone(), move |_| {
            // the new week is now the middle panel, put the track back before the browser paints
            move_track(&track_ref, None, false);
            swipe.borrow_mut().animating = false;
        });
    }

    let (prev_week, next_week) = (selected_week.previous(), selected_week.next());
    let current = resolve(&loaded, &selected_week, &category, &selected_name);
    // "Me" never loads the class/teacher/room tables, so its spinner follows the personal
    // timetable instead - otherwise the controls would sit on "Loading..." forever
    let loading = if *category == "Me" {
        loaded.get_mine(&selected_week).is_none()
    } else {
        loaded.get_all(&selected_week).is_none()
    };
    let prev_panel = resolve(&loaded, &prev_week, &category, &selected_name).panel;
    let next_panel = resolve(&loaded, &next_week, &category, &selected_name).panel;

    let on_category_change = {
        let category = category.clone();
        let selected_name = selected_name.clone();
        Callback::from(move |cat| {
            category.set(cat);
            selected_name.set(None);
        })
    };

    let on_reload = {
        let trigger = reload_trigger.clone();
        let selected_name = selected_name.clone();
        let dispatch = loaded.dispatcher();
        Callback::from(move |_| {
            let _ = CachedUntisClient::clear_cache();
            dispatch.dispatch(LoadedAction::Clear);
            selected_name.set(None);
            trigger.set(*trigger + 1);
        })
    };

    let on_entity_change = {
        let selected_name = selected_name.clone();
        Callback::from(move |name| selected_name.set(Some(name)))
    };

    let on_entity_select = {
        let category = category.clone();
        let selected_name = selected_name.clone();
        Callback::from(move |entity: Entity| {
            let cat = match entity {
                Entity::Class(_) => "Class",
                Entity::Teacher(_) => "Teacher",
                Entity::Room(_) => "Room",
                _ => return,
            };
            category.set(cat.to_string());
            selected_name.set(Some(entity.name()));
        })
    };

    let on_week_change = {
        let selected_week = selected_week.clone();
        Callback::from(move |week| selected_week.set(week))
    };

    let on_pointer_down = {
        let swipe = swipe.clone();
        Callback::from(move |e: PointerEvent| {
            let mut swipe = swipe.borrow_mut();
            if swipe.animating || !e.is_primary() || (e.pointer_type() == "mouse" && e.button() != 0) {
                return;
            }
            // the lesson modal lives inside the track, it shouldn't swipe
            if e.target_dyn_into::<Element>().and_then(|t| t.closest(".modal").ok().flatten()).is_some() {
                return;
            }
            let x = e.client_x() as f64;
            swipe.drag = Some(Drag {
                pointer_id: e.pointer_id(),
                start_x: x,
                offset: 0.0,
                last_x: x,
                last_time: e.time_stamp(),
                velocity: 0.0,
                captured: false,
            });
        })
    };

    let on_pointer_move = {
        let swipe = swipe.clone();
        let track_ref = track_ref.clone();
        Callback::from(move |e: PointerEvent| {
            let mut swipe = swipe.borrow_mut();
            let Some(drag) = swipe.drag.as_mut().filter(|d| d.pointer_id == e.pointer_id()) else { return };
            if e.buttons() == 0 {
                // released somewhere we didn't get told about
                swipe.drag = None;
                return;
            }

            let (x, time) = (e.client_x() as f64, e.time_stamp());
            if time > drag.last_time {
                drag.velocity = (x - drag.last_x) / (time - drag.last_time);
            }
            drag.last_x = x;
            drag.last_time = time;
            drag.offset = x - drag.start_x;

            // only take over the pointer once it's clearly a swipe, so taps still open the lessons
            if !drag.captured && drag.offset.abs() > 10.0 {
                drag.captured = true;
                if let Some(el) = track_ref.cast::<Element>() {
                    let _ = el.set_pointer_capture(e.pointer_id());
                }
            }
            if drag.captured {
                move_track(&track_ref, Some(drag.offset), false);
            }
        })
    };

    let on_pointer_up = {
        let swipe = swipe.clone();
        let track_ref = track_ref.clone();
        let selected_week = selected_week.clone();
        Callback::from(move |e: PointerEvent| {
            let Some(drag) = swipe.borrow_mut().drag.take() else { return };
            if !drag.captured {
                return;
            }

            let width = track_ref.cast::<Element>().map_or(0.0, |el| el.client_width() as f64);
            // a pause before letting go isn't a flick
            let velocity = if e.time_stamp() - drag.last_time > 100.0 { 0.0 } else { drag.velocity };
            let direction = drag.offset.signum();
            let commit = velocity * direction > -0.3 && (drag.offset.abs() > width / 4.0 || velocity * direction > 0.3);

            if !commit {
                snap_back(&track_ref, &swipe);
                return;
            }

            swipe.borrow_mut().animating = true;
            move_track(&track_ref, Some(direction * width), true);
            let target = if direction < 0.0 { selected_week.next() } else { selected_week.previous() };
            let selected_week = selected_week.clone();
            Timeout::new(SWIPE_MS, move || selected_week.set(target)).forget();
        })
    };

    let on_pointer_cancel = {
        let swipe = swipe.clone();
        let track_ref = track_ref.clone();
        Callback::from(move |_: PointerEvent| {
            let drag = swipe.borrow_mut().drag.take();
            if drag.is_some_and(|d| d.captured) {
                snap_back(&track_ref, &swipe);
            }
        })
    };

    html! {
        <div class="d-flex flex-column flex-grow-1 h-100">
            <TimetableControls
                category={(*category).clone()}
                selected_name={current.active_name}
                selected_week={(*selected_week).clone()}
                filtered_names={current.names}
                loading={loading}
                on_category_change={on_category_change}
                on_entity_change={on_entity_change}
                on_week_change={on_week_change}
                on_reload={on_reload}
            />
            <div class="flex-grow-1 w-100 position-relative overflow-hidden">
                <div
                    ref={track_ref}
                    class="week-track position-absolute top-0 start-0 h-100 w-100"
                    style="user-select: none;"
                    onpointerdown={on_pointer_down}
                    onpointermove={on_pointer_move}
                    onpointerup={on_pointer_up}
                    onpointercancel={on_pointer_cancel}
                >
                    { render_panel(&prev_week, "-100%", prev_panel, on_entity_select.clone()) }
                    { render_panel(&selected_week, "0", current.panel, on_entity_select.clone()) }
                    { render_panel(&next_week, "100%", next_panel, on_entity_select) }
                </div>
            </div>
        </div>
    }
}
