use crate::book2eat::client::{get_b2e_token, get_menu};
use crate::components::qr_code::QrCode;
use altis_core::data_models::response_models::book2eat::Meal;
use altis_core::untis::untis_week::Week;
use chrono::{Duration, Local, NaiveDate};
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::{MouseEvent, TouchEvent};
use yew::prelude::*;
use yew::suspense::use_future_with;

/// Minimum horizontal drag distance (in px) before a gesture counts as a swipe.
const SWIPE_THRESHOLD_PX: i32 = 50;

#[function_component(Book2EatComponent)]
pub fn book2eat() -> HtmlResult {
    let reload_trigger = use_state(|| 0);
    let current_date = use_state(|| Local::now().date_naive());
    let current_week = Week::from_date(*current_date);

    // Refetches only when the visible week actually changes (i.e. when a
    // swipe crosses a week boundary), not on every single day change.
    let res = {
        let week = current_week.clone();
        use_future_with((week, *reload_trigger), |deps| {
            let (week, _reload) = (*deps).clone();
            async move {
                let (user, token) = get_b2e_token().await.map_err(|e| e.to_string())?;

                get_menu(week, user, token)
                    .await
                    .map_err(|e| e.to_string())
            }
        })?
    };

    // Tracks the pointer/touch start position for the current gesture.
    // Kept outside component state on purpose: it changes on every move but
    // should never itself trigger a re-render.
    let drag_start: Rc<RefCell<Option<(i32, i32)>>> = use_mut_ref(|| None);

    let on_touch_start = {
        let drag_start = drag_start.clone();
        Callback::from(move |e: TouchEvent| {
            if let Some(touch) = e.touches().get(0) {
                *drag_start.borrow_mut() = Some((touch.client_x(), touch.client_y()));
            }
        })
    };

    let on_touch_end = {
        let drag_start = drag_start.clone();
        let current_date = current_date.clone();
        Callback::from(move |e: TouchEvent| {
            if let Some(touch) = e.changed_touches().get(0) {
                if let Some((sx, sy)) = drag_start.borrow_mut().take() {
                    apply_swipe(sx, sy, touch.client_x(), touch.client_y(), *current_date, &current_date);
                }
            }
        })
    };

    let on_mouse_down = {
        let drag_start = drag_start.clone();
        Callback::from(move |e: MouseEvent| {
            *drag_start.borrow_mut() = Some((e.client_x(), e.client_y()));
        })
    };

    let on_mouse_up = {
        let drag_start = drag_start.clone();
        let current_date = current_date.clone();
        Callback::from(move |e: MouseEvent| {
            if let Some((sx, sy)) = drag_start.borrow_mut().take() {
                apply_swipe(sx, sy, e.client_x(), e.client_y(), *current_date, &current_date);
            }
        })
    };

    let on_mouse_leave = {
        let drag_start = drag_start.clone();
        Callback::from(move |_: MouseEvent| {
            *drag_start.borrow_mut() = None;
        })
    };

    match &*res {
        Ok(menu_response) => {
            let selected_date_str = current_date.format("%Y-%m-%d").to_string();

            let todays_meals: Vec<_> = menu_response.data.menue.values()
                .filter(|meal| meal.date == selected_date_str).cloned()
                .collect();

            let day_label = current_date.format("%A, %d %B").to_string();
            let days_in_week = week_days(&current_week);

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
        <style>
            {".b2e-qr-wrapper { flex-shrink: 0; width: min(92vw, 520px); height: min(92vw, 520px); }
              .b2e-swipe-area { touch-action: pan-y; cursor: grab; user-select: none; }
              .b2e-dot { width: 8px; height: 8px; border-radius: 50%; background-color: rgba(255,255,255,0.3); cursor: pointer; transition: background-color 0.2s ease, transform 0.2s ease; }
              .b2e-dot.active { background-color: #fff; transform: scale(1.3); }
              .b2e-meal-list { animation: b2e-fade-in 0.2s ease; }
              .b2e-meal-name { font-size: clamp(0.8rem, 2cqw, 2.5rem); }
              .b2e-meal-type { font-size: clamp(0.5rem, 1.6cqw, 1.5rem); }
              @keyframes b2e-fade-in { from { opacity: 0; transform: translateY(4px); } to { opacity: 1; transform: translateY(0); } }
              @media (min-width: 768px) {
                .b2e-qr-wrapper { width: min(520px, 40vh); height: min(520px, 40vh); }
                .b2e-meal-name { font-size: clamp(0.9rem, min(1.4vw, 2.8vh), 1.6rem); }
                .b2e-meal-type { font-size: clamp(0.7rem, min(1vw, 2vh), 1.1rem); }
              }"}
        </style>

        <div class="d-flex justify-content-center align-items-center bg-light rounded p-4 mb-3 b2e-qr-wrapper">
            <QrCode data={menu_response.data.qr_code.clone()} />
        </div>

        <div class="d-flex flex-column align-items-center mb-2">
            <div class="text-white fw-semibold" style="font-size: clamp(0.9rem, 2.2cqw, 1.4rem);">
                { day_label }
            </div>
            <div class="d-flex gap-2 mt-1">
                { for days_in_week.iter().map(|day| {
                    let is_active = *day == *current_date;
                    let dot_class = if is_active { "b2e-dot active" } else { "b2e-dot" };
                    let day = *day;
                    let current_date = current_date.clone();
                    let onclick = Callback::from(move |_| current_date.set(day));
                    html! { <span class={dot_class} onclick={onclick}></span> }
                }) }
            </div>
        </div>

        <div
            key={selected_date_str}
            class="d-flex flex-column flex-grow-1 justify-content-evenly bg-black w-100 p-2 mx-5 g-1 rounded-4 overflow-auto b2e-meal-list"
        >
            { for sort_meals(todays_meals.clone()).iter().enumerate().map(|(index, meal)| {
                let is_last = index == todays_meals.len() - 1;
                let border_class = if is_last { "" } else { "border-bottom border-secondary" };

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
        Err(err_msg) => Ok(html! {
            <div class="container mt-5">
                <div class="alert alert-danger shadow-sm">
                    <strong>{"Error:"}</strong> { format!(" {}", err_msg) }
                </div>
            </div>
        })
    }
}

/// Applies a swipe/drag gesture: if the horizontal movement exceeds the
/// threshold and dominates over vertical movement, moves to the next or
/// previous day. Left swipe -> next day, right swipe -> previous day.
fn apply_swipe(
    sx: i32,
    sy: i32,
    ex: i32,
    ey: i32,
    current: NaiveDate,
    setter: &UseStateHandle<NaiveDate>,
) {
    let dx = ex - sx;
    let dy = ey - sy;

    if dx.abs() >= SWIPE_THRESHOLD_PX && dx.abs() > dy.abs() {
        let new_date = if dx < 0 {
            current + Duration::days(1)
        } else {
            current - Duration::days(1)
        };
        setter.set(new_date);
    }
}

/// Returns the 7 dates (Monday..Sunday) belonging to the given week.
fn week_days(week: &Week) -> Vec<NaiveDate> {
    let start = NaiveDate::parse_from_str(&week.start, "%Y-%m-%d")
        .unwrap_or_else(|_| Local::now().date_naive());

    (0..7).map(|i| start + Duration::days(i)).collect()
}

fn sort_meals(todays_meals: Vec<Meal>) -> Vec<Meal> {
    let mut todays_meals = todays_meals.clone();
    todays_meals.sort_by_key(|x| { x.typ_name.clone().chars().next() });
    todays_meals.reverse();
    todays_meals
}