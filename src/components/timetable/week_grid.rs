use super::day_heading;
use super::lesson_block::render_lesson_group;
use super::lesson_modal::LessonModal;
use super::week_layout::{fill_breaks, group_by_time, time_slots, TimeSlot};
use crate::components::now_line::NowLine;
use crate::hooks::use_now;
use altis_core::models::ascii_timetable;
use altis_core::models::timetable::{DayTimeTable, Entity, LessonBlock, WeekTimeTable};
use altis_core::settings::VisualSettings;
use chrono::{Datelike, NaiveTime};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct WeekGridProps {
    pub timetable: WeekTimeTable,
    pub visual: VisualSettings,
    /// a teacher, class or room was picked in the lesson details, to show its timetable
    pub on_entity_select: Callback<Entity>,
}

/// The days of the week that are shown, in order, with the lessons in the colours set for them
fn shown_days(timetable: &WeekTimeTable, visual: &VisualSettings) -> Vec<DayTimeTable> {
    let mut days: Vec<DayTimeTable> = timetable
        .days
        .iter()
        .filter(|day| visual.weekday_override.should_show(day.date.weekday(), !day.lessons.is_empty()))
        .cloned()
        .collect();
    days.sort_by_key(|day| day.date);

    for lesson in days.iter_mut().flat_map(|day| &mut day.lessons) {
        visual.apply_color_override(lesson);
    }
    days
}

/// A week's timetable: a column per day, the lessons sized and placed by their times
#[function_component(WeekGrid)]
pub fn week_grid(props: &WeekGridProps) -> Html {
    let selected_group = use_state(|| None::<Vec<LessonBlock>>);
    let now = use_now();

    let on_group_click = {
        let selected_group = selected_group.clone();
        Callback::from(move |lessons: Vec<LessonBlock>| {
            // a break has no details to show
            if lessons.iter().any(|lesson| !lesson.is_break()) {
                selected_group.set(Some(lessons));
            }
        })
    };

    let on_close = {
        let selected_group = selected_group.clone();
        Callback::from(move |_| selected_group.set(None))
    };

    let on_entity_click = {
        let selected_group = selected_group.clone();
        let on_entity_select = props.on_entity_select.clone();
        Callback::from(move |entity| {
            selected_group.set(None);
            on_entity_select.emit(entity);
        })
    };

    let days = shown_days(&props.timetable, &props.visual);

    if props.visual.force_ascii_timetable {
        return html! {
            <div class="d-flex flex-grow-1 flex-column">
                <pre>{ ascii_timetable::render(&WeekTimeTable { days }) }</pre>
            </div>
        };
    }

    let lessons: Vec<LessonBlock> = days.iter().flat_map(|day| day.lessons.clone()).collect();
    let first_start = lessons.iter().map(|lesson| lesson.time_range.start.time()).min();
    let last_end = lessons.iter().map(|lesson| lesson.time_range.end.time()).max();
    let (Some(first_start), Some(last_end)) = (first_start, last_end) else {
        return html! {
            <div class="d-flex flex-grow-1 flex-column">
                {"No lessons!"}
            </div>
        };
    };

    // the grid runs from `start` to `end`, and everything in it is placed in percent of that
    let (start, end) = props.visual.padded_range(first_start, last_end);
    let total_seconds = (end - start).num_seconds() as f64;
    let percent_of_day = |seconds: i64| seconds as f64 / total_seconds * 100.0;

    let render_time_slot = |slot: &TimeSlot| {
        let top = percent_of_day((slot.start - start).num_seconds());
        let height = percent_of_day((slot.end - slot.start).num_seconds());
        let label = |show: bool, time: NaiveTime| if show { time.format("%H:%M").to_string() } else { String::new() };

        html! {
            <div style={format!("position: absolute; top: {top}%; height: {height}%; width: 100%;")}
                  class={format!("d-flex flex-column justify-content-between align-items-end border-bottom {} m-0", if slot.after_break { "border-top" } else { "" })}>
                <div class="small pe-1">{ label(slot.label_start, slot.start) }</div>
                <div class="small pe-1">{ label(slot.label_end, slot.end) }</div>
            </div>
        }
    };

    let render_day = |day: &DayTimeTable| {
        let now_top = (day.date == now.date() && (start..=end).contains(&now.time())).then(|| percent_of_day((now.time() - start).num_seconds()));
        html! {
            <div class="flex-grow-1 border-start position-relative flex" style="flex-basis: 0; min-width: 0; overflow: hidden;">
                { for group_by_time(fill_breaks(day.lessons.clone(), start)).iter().map(|group| {
                    render_lesson_group(group, end - start, now, on_group_click.clone())
                })}
                if let Some(top) = now_top {
                    <NowLine {top} />
                }
            </div>
        }
    };

    html! {
        <>
            if let Some(lessons) = (*selected_group).clone() {
                <LessonModal {lessons} {on_close} {on_entity_click} />
            }

            <div
                class="d-flex flex-grow-1 flex-column h-100 w-100 overflow-hidden"
            >
                <div class="d-flex w-100 bg-dark border-bottom">
                    <div style="width: 60px;" class="flex-shrink-0"></div>
                    <div class="d-flex flex-grow-1">
                        { for days.iter().map(|day| html! {
                            <div class="flex-grow-1 text-center border-start pb-1" style="flex-basis: 0;">
                                { day_heading(day.date) }
                            </div>
                        })}
                    </div>
                </div>

                <div class="d-flex flex-grow-1 w-100">
                    <div style="width: 60px; position: relative;" class="d-flex flex-column flex-shrink-0">
                        { for time_slots(&lessons).iter().map(render_time_slot) }
                    </div>
                    <div class="d-flex flex-grow-1">
                        { for days.iter().map(render_day) }
                    </div>
                </div>
            </div>
        </>
    }
}
