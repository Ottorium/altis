//! The timetables of the week on screen and its neighbours, and the loading of them.

use crate::untis::cached_client::{AllTimeTables, CachedUntisClient, Expiring};
use altis_core::errors::ApiError;
use altis_core::models::timetable::MyTimeTable;
use altis_core::settings::TimetableView;
use altis_core::untis::week::Week;
use chrono::{Local, NaiveDateTime};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::{Closure, JsValue};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use yew::prelude::*;

/// A loaded timetable along with the time it expires
type Entry<T> = (NaiveDateTime, Rc<Result<T, ApiError>>);

fn entry<T>(loaded: Result<Expiring<T>, ApiError>) -> Entry<T> {
    match loaded {
        Ok((expiry, value)) => (expiry, Rc::new(Ok(value))),
        // expire errors right away, so they are retried the next time the week is shown
        Err(e) => (Local::now().naive_local(), Rc::new(Err(e))),
    }
}

/// loaded successfully and not expired yet
fn is_fresh<T>(entry: Option<&Entry<T>>) -> bool {
    entry.is_some_and(|(expiry, loaded)| loaded.is_ok() && *expiry > Local::now().naive_local())
}

/// Timetables of the displayed week and its neighbours, so swiping can show them right away
#[derive(Default, Clone)]
pub struct LoadedWeeks {
    mine: HashMap<Week, Entry<MyTimeTable>>,
    all: HashMap<Week, Entry<AllTimeTables>>,
}

impl LoadedWeeks {
    /// The personal timetable of a week, `None` while it is still loading
    pub fn mine(&self, week: &Week) -> Option<&Result<MyTimeTable, ApiError>> {
        self.mine.get(week).map(|(_, loaded)| &**loaded)
    }

    /// The timetables of every class, teacher and room of a week, `None` while they are still loading
    pub fn all(&self, week: &Week) -> Option<&Result<AllTimeTables, ApiError>> {
        self.all.get(week).map(|(_, loaded)| &**loaded)
    }
}

pub enum LoadedAction {
    Mine(Week, Result<Expiring<MyTimeTable>, ApiError>),
    All(Week, Result<Expiring<AllTimeTables>, ApiError>),
    /// drops all other weeks, so only the visible ones are kept in memory
    Retain(Vec<Week>),
    Clear,
}

impl Reducible for LoadedWeeks {
    type Action = LoadedAction;

    fn reduce(self: Rc<Self>, action: LoadedAction) -> Rc<Self> {
        let mut next = (*self).clone();
        match action {
            LoadedAction::Mine(week, loaded) => {
                next.mine.insert(week, entry(loaded));
            }
            LoadedAction::All(week, loaded) => {
                next.all.insert(week, entry(loaded));
            }
            LoadedAction::Retain(weeks) => {
                next.mine.retain(|week, _| weeks.contains(week));
                next.all.retain(|week, _| weeks.contains(week));
            }
            LoadedAction::Clear => next = Self::default(),
        }
        Rc::new(next)
    }
}

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

/// The timetables of the week and its neighbours, loading what the view needs of them whenever
/// the week or the view changes, or `reload` does
#[hook]
pub fn use_loaded_weeks(week: &Week, view: TimetableView, reload: u32) -> UseReducerHandle<LoadedWeeks> {
    let loaded = use_reducer(LoadedWeeks::default);

    {
        let dispatch = loaded.dispatcher();
        let loaded = loaded.clone();
        use_effect_with((week.clone(), view, reload), move |(week, view, _)| {
            let neighbours = [week.previous(), week.next()];
            dispatch.dispatch(LoadedAction::Retain(vec![neighbours[0].clone(), week.clone(), neighbours[1].clone()]));

            // the personal timetable is its own request; the class, teacher and room tables cost
            // one request per class and a full regeneration on top, so they are only loaded once
            // something actually shows them. Loading them for "Me" is what froze a swipe to a
            // week that wasn't cached yet.
            let wants_all = *view != TimetableView::Me;

            let needs_mine = !is_fresh(loaded.mine.get(week));
            let needs_all = wants_all && !is_fresh(loaded.all.get(week));
            let mine_missing: Vec<Week> = neighbours.iter().filter(|week| !is_fresh(loaded.mine.get(week))).cloned().collect();
            let all_missing: Vec<Week> = match wants_all {
                true => neighbours.iter().filter(|week| !is_fresh(loaded.all.get(week))).cloned().collect(),
                false => Vec::new(),
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
                for (week, mine) in CachedUntisClient::cached_my_timetables(&mine_missing) {
                    dispatch.dispatch(LoadedAction::Mine(week, Ok(mine)));
                }

                // the rest blocks the main thread for a while (reading the cache, generating the teacher/room
                // tables) so it may only start once the personal timetable is actually on screen
                next_paint().await;
                if is_cancelled.get() {
                    return;
                }
                if needs_all {
                    let all = client.get_all_timetables(week.clone()).await;
                    dispatch.dispatch(LoadedAction::All(week, all));
                    next_paint().await;
                    if is_cancelled.get() {
                        return;
                    }
                }
                for (week, all) in CachedUntisClient::cached_all_timetables(&all_missing) {
                    dispatch.dispatch(LoadedAction::All(week, Ok(all)));
                }
            });

            move || cancelled.set(true)
        });
    }

    loaded
}
