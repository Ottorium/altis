mod compare;
mod controls;
mod free_rooms;
mod lesson_block;
mod lesson_modal;
mod page;
mod panel;
mod swipe;
mod week_grid;
mod week_layout;
mod weeks;

pub use compare::Compared;
pub use page::TimetablePage;

use altis_core::settings::LessonTextSettings;
use chrono::{Datelike, NaiveDate};
use yew::prelude::*;

/// The custom properties the stylesheet writes the lessons' text by, `None` while the text is
/// written the way it always was
fn lesson_text_style(text: &LessonTextSettings) -> Option<String> {
    if text.is_default_style() {
        return None;
    }

    let mut style = format!(" --lesson-font-scale: {};", text.font_scale());
    if let Some(family) = text.font.css_family() {
        style += &format!(" --lesson-font-family: {family};");
    }
    if text.bold {
        style += " --lesson-font-weight: bold;";
    }
    Some(style)
}

/// The weekday and the date a day's column is headed with
fn day_heading(date: NaiveDate) -> Html {
    html! {
        <>
            <div class="fw-bold">{ date.weekday().to_string() }</div>
            <div class="small">{ date.format("%d.%m").to_string() }</div>
        </>
    }
}
