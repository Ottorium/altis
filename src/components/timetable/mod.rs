mod compare;
mod controls;
mod free_rooms;
mod lesson_block;
mod lesson_modal;
mod page;
mod panel;
mod swipe;
mod view;
mod week_grid;
mod week_layout;
mod weeks;

pub use compare::Compared;
pub use page::TimetablePage;

use chrono::{Datelike, NaiveDate};
use yew::prelude::*;

/// The weekday and the date a day's column is headed with
fn day_heading(date: NaiveDate) -> Html {
    html! {
        <>
            <div class="fw-bold">{ date.weekday().to_string() }</div>
            <div class="small">{ date.format("%d.%m").to_string() }</div>
        </>
    }
}
