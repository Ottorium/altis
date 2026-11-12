use crate::book2eat_client::get_menu;
use crate::components::qr_code::QrCode;
use altis_core::models::book2eat::Meal;
use altis_core::untis::week::Week;
use chrono::{Duration, Local};
use web_sys::{MouseEvent, TouchEvent};
use yew::prelude::*;
use yew::suspense::use_future_with;

/// Minimum horizontal drag distance (in px) before a gesture counts as a swipe.
const SWIPE_THRESHOLD_PX: i32 = 50;

const MEAL_DATE_FORMAT: &str = "%Y-%m-%d";

/// A point on the screen, as (x, y)
type Point = (i32, i32);

/// The canteen's menu of a day along with the QR code it scans, a swipe from day to day
#[function_component(Book2EatPage)]
pub fn book2eat() -> HtmlResult {
    let current_date = use_state(|| Local::now().date_naive());
    let current_week = Week::from_date(*current_date);

    // Refetches only when the visible week actually changes (i.e. when a
    // swipe crosses a week boundary), not on every single day change.
    let menu = use_future_with(current_week.clone(), |week| async move { get_menu(&week).await.map_err(|e| e.to_string()) })?;

    // Where the current gesture started. Kept outside component state on purpose: it should
    // never itself trigger a re-render.
    let drag_start = use_mut_ref(|| None::<Point>);

    let start_drag = {
        let drag_start = drag_start.clone();
        move |point: Option<Point>| *drag_start.borrow_mut() = point
    };
    let end_drag = {
        let current_date = current_date.clone();
        move |end: Point| {
            if let Some(start) = drag_start.borrow_mut().take()
                && let Some(days) = swiped_days(start, end) {
                current_date.set(*current_date + Duration::days(days));
            }
        }
    };

    let on_touch_start = {
        let start_drag = start_drag.clone();
        Callback::from(move |e: TouchEvent| {
            if let Some(touch) = e.touches().get(0) {
                start_drag(Some((touch.client_x(), touch.client_y())));
            }
        })
    };
    let on_touch_end = {
        let end_drag = end_drag.clone();
        Callback::from(move |e: TouchEvent| {
            if let Some(touch) = e.changed_touches().get(0) {
                end_drag((touch.client_x(), touch.client_y()));
            }
        })
    };
    let on_mouse_down = {
        let start_drag = start_drag.clone();
        Callback::from(move |e: MouseEvent| start_drag(Some((e.client_x(), e.client_y()))))
    };
    let on_mouse_up = Callback::from(move |e: MouseEvent| end_drag((e.client_x(), e.client_y())));
    let on_mouse_leave = Callback::from(move |_: MouseEvent| start_drag(None));

    let menu = match &*menu {
        Ok(menu) => &menu.data,
        Err(message) => {
            return Ok(html! {
                <div class="container mt-5">
                    <div class="alert alert-danger shadow-sm">
                        <strong>{"Error:"}</strong> { format!(" {message}") }
                    </div>
                </div>
            });
        }
    };

    let selected_date = current_date.format(MEAL_DATE_FORMAT).to_string();
    let meals = by_course(menu.menue.values().filter(|meal| meal.date == selected_date).cloned().collect());
    let last_meal = meals.len().saturating_sub(1);

    Ok(html! {
        <div
            class="container pt-4 pb-2 d-flex flex-column align-items-center w-100 h-100 b2e-swipe-area"
            style="overflow: hidden;"
            ontouchstart={on_touch_start}
            ontouchend={on_touch_end}
            onmousedown={on_mouse_down}
            onmouseup={on_mouse_up}
            onmouseleave={on_mouse_leave}
        >
            <div class="d-flex justify-content-center align-items-center bg-light rounded p-4 mb-3 b2e-qr-wrapper">
                <QrCode data={menu.qr_code.clone()} />
            </div>

            <div class="d-flex flex-column align-items-center mb-2">
                <div class="text-white fw-semibold" style="font-size: clamp(0.9rem, 2.2cqw, 1.4rem);">
                    { current_date.format("%A, %d %B").to_string() }
                </div>
                <div class="d-flex gap-2 mt-1">
                    { for current_week.days().map(|day| {
                        let is_shown = day == *current_date;
                        let current_date = current_date.clone();
                        let onclick = Callback::from(move |_| current_date.set(day));
                        html! { <span class={classes!("b2e-dot", is_shown.then_some("active"))} {onclick}></span> }
                    }) }
                </div>
            </div>

            <div
                key={selected_date}
                class="d-flex flex-column flex-grow-1 justify-content-evenly bg-black w-100 p-2 mx-5 g-1 rounded-4 overflow-auto b2e-meal-list"
            >
                { for meals.iter().enumerate().map(|(index, meal)| {
                    let border_class = (index != last_meal).then_some("border-bottom border-secondary");

                    html! {
                        <div class={classes!(border_class, "shadow-sm", "text-center", "text-white", "py-2", "w-100")}>
                            <div class="fw-bold b2e-meal-name">
                                { &meal.name }
                            </div>
                            <div class="text-secondary b2e-meal-type">
                                { &meal.typ_name }
                            </div>
                        </div>
                    }
                }) }
            </div>
        </div>
    })
}

/// How many days a gesture moves on, `None` if it is no swipe: the horizontal movement has to
/// exceed the threshold and dominate over the vertical one. Swiping left goes to the next day.
fn swiped_days((start_x, start_y): Point, (end_x, end_y): Point) -> Option<i64> {
    let (dx, dy) = (end_x - start_x, end_y - start_y);
    let is_swipe = dx.abs() >= SWIPE_THRESHOLD_PX && dx.abs() > dy.abs();

    is_swipe.then_some(if dx < 0 { 1 } else { -1 })
}

/// The day's meals in the order they are eaten in, which happens to be the reverse alphabetical
/// order of what the canteen calls the courses
fn by_course(mut meals: Vec<Meal>) -> Vec<Meal> {
    meals.sort_by_key(|meal| meal.typ_name.chars().next());
    meals.reverse();
    meals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_swipe_to_the_left_goes_to_the_next_day() {
        assert_eq!(swiped_days((200, 100), (100, 110)), Some(1));
        assert_eq!(swiped_days((100, 100), (200, 110)), Some(-1));
    }

    #[test]
    fn a_short_or_mostly_vertical_movement_is_no_swipe() {
        assert_eq!(swiped_days((100, 100), (60, 100)), None);
        assert_eq!(swiped_days((100, 100), (40, 200)), None);
    }

    #[test]
    fn lists_the_courses_in_reverse_alphabetical_order() {
        let meal = |course: &str| Meal { typ_name: course.to_string(), ..Default::default() };
        let courses: Vec<String> = by_course(vec![meal("Menü 1"), meal("Suppe"), meal("Vegetarisch")]).into_iter().map(|meal| meal.typ_name).collect();

        assert_eq!(courses, vec!["Vegetarisch", "Suppe", "Menü 1"]);
    }
}
