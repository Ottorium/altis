//! Swiping from week to week: the three week panels sit side by side on a track that follows the
//! finger and slides the next week in when let go far or fast enough.

use altis_core::untis::week::Week;
use gloo_timers::callback::Timeout;
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::{Element, HtmlElement};
use yew::prelude::*;

/// How long the track takes to slide into place
const SLIDE_MS: u32 = 250;
/// How far the pointer has to move before it is a swipe rather than a tap, in px
const SWIPE_START_PX: f64 = 10.0;
/// A swipe let go at this speed (px/ms) changes the week however short it was
const FLICK_VELOCITY: f64 = 0.3;
/// The part of the track's width a slow swipe has to cover to change the week
const COMMIT_DISTANCE: f64 = 0.25;
/// Letting go this long after the last movement is no flick, in ms
const FLICK_PAUSE_MS: f64 = 100.0;

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
    let Some(element) = track.cast::<HtmlElement>() else { return };
    let style = element.style();
    let transition = if animate { format!("transform {SLIDE_MS}ms ease-out") } else { "none".to_string() };
    let _ = style.set_property("transition", &transition);
    match offset {
        Some(x) => {
            let _ = style.set_property("transform", &format!("translateX({x}px)"));
        }
        // any transform makes the track the containing block of the fixed lesson modal, so it's removed at rest
        None => {
            let _ = style.remove_property("transform");
        }
    }
}

fn snap_back(track: &NodeRef, swipe: &Rc<RefCell<Swipe>>) {
    swipe.borrow_mut().animating = true;
    move_track(track, Some(0.0), true);
    let (track, swipe) = (track.clone(), swipe.clone());
    Timeout::new(SLIDE_MS, move || {
        move_track(&track, None, false);
        swipe.borrow_mut().animating = false;
    })
    .forget();
}

/// Whether a swipe let go changes the week: one that covered enough of the track or was a flick,
/// unless it was flicked back the other way
fn commits(offset: f64, velocity: f64, track_width: f64) -> bool {
    let along = velocity * offset.signum();
    along > -FLICK_VELOCITY && (offset.abs() > track_width * COMMIT_DISTANCE || along > FLICK_VELOCITY)
}

/// What the track element needs to be swiped
pub struct WeekSwipe {
    pub track: NodeRef,
    pub onpointerdown: Callback<PointerEvent>,
    pub onpointermove: Callback<PointerEvent>,
    pub onpointerup: Callback<PointerEvent>,
    pub onpointercancel: Callback<PointerEvent>,
}

/// Lets the week be changed by swiping the track
#[hook]
pub fn use_week_swipe(selected_week: &UseStateHandle<Week>) -> WeekSwipe {
    let track = use_node_ref();
    let swipe = use_mut_ref(Swipe::default);

    {
        let track = track.clone();
        let swipe = swipe.clone();
        use_effect_with((**selected_week).clone(), move |_| {
            // the new week is now the middle panel, put the track back before the browser paints
            move_track(&track, None, false);
            swipe.borrow_mut().animating = false;
        });
    }

    let onpointerdown = {
        let swipe = swipe.clone();
        Callback::from(move |e: PointerEvent| {
            let mut swipe = swipe.borrow_mut();
            if swipe.animating || !e.is_primary() || (e.pointer_type() == "mouse" && e.button() != 0) {
                return;
            }
            // the lesson modal lives inside the track, it shouldn't swipe
            if e.target_dyn_into::<Element>().and_then(|target| target.closest(".modal").ok().flatten()).is_some() {
                return;
            }
            let x = f64::from(e.client_x());
            swipe.drag = Some(Drag { pointer_id: e.pointer_id(), start_x: x, offset: 0.0, last_x: x, last_time: e.time_stamp(), velocity: 0.0, captured: false });
        })
    };

    let onpointermove = {
        let swipe = swipe.clone();
        let track = track.clone();
        Callback::from(move |e: PointerEvent| {
            let mut swipe = swipe.borrow_mut();
            let Some(drag) = swipe.drag.as_mut().filter(|drag| drag.pointer_id == e.pointer_id()) else { return };
            if e.buttons() == 0 {
                // released somewhere we didn't get told about
                swipe.drag = None;
                return;
            }

            let (x, time) = (f64::from(e.client_x()), e.time_stamp());
            if time > drag.last_time {
                drag.velocity = (x - drag.last_x) / (time - drag.last_time);
            }
            drag.last_x = x;
            drag.last_time = time;
            drag.offset = x - drag.start_x;

            // only take over the pointer once it's clearly a swipe, so taps still open the lessons
            if !drag.captured && drag.offset.abs() > SWIPE_START_PX {
                drag.captured = true;
                if let Some(element) = track.cast::<Element>() {
                    let _ = element.set_pointer_capture(e.pointer_id());
                }
            }
            if drag.captured {
                move_track(&track, Some(drag.offset), false);
            }
        })
    };

    let onpointerup = {
        let swipe = swipe.clone();
        let track = track.clone();
        let selected_week = selected_week.clone();
        Callback::from(move |e: PointerEvent| {
            let Some(drag) = swipe.borrow_mut().drag.take() else { return };
            if !drag.captured {
                return;
            }

            let width = track.cast::<Element>().map_or(0.0, |element| f64::from(element.client_width()));
            // a pause before letting go isn't a flick
            let velocity = if e.time_stamp() - drag.last_time > FLICK_PAUSE_MS { 0.0 } else { drag.velocity };
            if !commits(drag.offset, velocity, width) {
                snap_back(&track, &swipe);
                return;
            }

            let direction = drag.offset.signum();
            swipe.borrow_mut().animating = true;
            move_track(&track, Some(direction * width), true);
            let target = if direction < 0.0 { selected_week.next() } else { selected_week.previous() };
            let selected_week = selected_week.clone();
            Timeout::new(SLIDE_MS, move || selected_week.set(target)).forget();
        })
    };

    let onpointercancel = {
        let track = track.clone();
        Callback::from(move |_: PointerEvent| {
            let drag = swipe.borrow_mut().drag.take();
            if drag.is_some_and(|drag| drag.captured) {
                snap_back(&track, &swipe);
            }
        })
    };

    WeekSwipe { track, onpointerdown, onpointermove, onpointerup, onpointercancel }
}

#[cfg(test)]
mod tests {
    use super::commits;

    const WIDTH: f64 = 400.0;

    #[test]
    fn a_slow_swipe_has_to_cover_a_quarter_of_the_track() {
        assert!(commits(-101.0, 0.0, WIDTH));
        assert!(commits(101.0, 0.0, WIDTH));
        assert!(!commits(-99.0, 0.0, WIDTH));
    }

    #[test]
    fn a_flick_changes_the_week_however_short_it_was() {
        assert!(commits(-20.0, -0.5, WIDTH));
        assert!(commits(20.0, 0.5, WIDTH));
        assert!(!commits(-20.0, -0.2, WIDTH));
    }

    #[test]
    fn a_swipe_flicked_back_is_taken_back() {
        assert!(!commits(-200.0, 0.5, WIDTH));
        assert!(commits(-200.0, 0.2, WIDTH));
    }
}
