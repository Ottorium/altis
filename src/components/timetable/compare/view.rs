use super::{Comparison, Day};
use crate::components::timetable::day_heading;
use crate::hooks::use_now;
use altis_core::models::timetable::{Entity, LessonBlock, LessonStatus};
use altis_core::settings::is_dark_color;
use chrono::{NaiveDateTime, NaiveTime, Timelike};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct CompareViewProps {
    pub comparison: Comparison,
}

#[function_component(CompareView)]
pub fn compare_view(props: &CompareViewProps) -> Html {
    let Comparison { names, missing, days, start, end } = &props.comparison;
    let now = use_now();

    let missing_note = (!missing.is_empty()).then(|| html! {
        <div class="small text-secondary mb-2">
            <i class="bi bi-exclamation-circle me-1"></i>
            { format!("Not in this week: {}", missing.join(", ")) }
        </div>
    });

    let total = (*end - *start).num_seconds() as f64;
    if !days.iter().any(Day::has_lessons) || total <= 0.0 {
        return html! { <div class="p-3">{ missing_note }{ "No lessons!" }</div> };
    }
    // how far down the grid a time is, in percent
    let percent = |time: NaiveTime| (time - *start).num_seconds() as f64 / total * 100.0;
    let position = |lesson: &LessonBlock| {
        let (from, until) = (percent(lesson.time_range.start.time()), percent(lesson.time_range.end.time()));
        format!("top: {from}%; height: {}%;", until - from)
    };

    let hours: Vec<NaiveTime> =
        (start.hour()..=end.hour()).filter_map(|hour| NaiveTime::from_hms_opt(hour, 0, 0)).filter(|hour| start <= hour && hour <= end).collect();

    let legend = names.iter().enumerate().map(|(index, name)| html! {
        <span class="badge compare-legend">
            <span class="compare-lane-number">{ index + 1 }</span>{ name }
        </span>
    }).collect::<Html>();

    let render_day = |day: &Day| {
        let now_top = (day.date == now.date() && (*start..=*end).contains(&now.time())).then(|| percent(now.time()));
        html! {
            <div class="compare-day">
                { for hours.iter().map(|&hour| html! {
                    <div class="compare-hour-line" style={format!("top: {}%;", percent(hour))}></div>
                })}
                <div class="d-flex h-100">
                    { for day.lanes.iter().map(|lessons| html! {
                        <div class="compare-lane">
                            { for lessons.iter().map(|lesson| render_lesson(lesson, position(lesson), now)) }
                        </div>
                    })}
                </div>
                if let Some(top) = now_top {
                    <div class="compare-now-line" style={format!("top: {top}%;")}></div>
                }
            </div>
        }
    };

    html! {
        <div class="d-flex flex-column p-2 gap-2" style="min-height: 100%;">
            <div class="d-flex flex-wrap align-items-center gap-1">{ legend }</div>
            { missing_note }

            <div class="d-flex flex-column flex-grow-1" style="min-height: 24rem;">
                <div class="d-flex border-bottom">
                    <div class="compare-time-column"></div>
                    { for days.iter().map(|day| html! {
                        <div class="compare-day-head text-center pb-1">
                            { day_heading(day.date) }
                            <div class="d-flex compare-lane-numbers">
                                { for (1..=day.lanes.len()).map(|lane| html! { <span>{ lane }</span> }) }
                            </div>
                        </div>
                    })}
                </div>
                <div class="d-flex flex-grow-1">
                    <div class="compare-time-column position-relative">
                        // the label sits below its line, so the last hour's would hang out of the grid
                        { for hours.iter().filter(|&hour| hour < end).map(|&hour| html! {
                            <div class="compare-hour-label small text-secondary" style={format!("top: {}%;", percent(hour))}>
                                { hour.format("%H:%M").to_string() }
                            </div>
                        })}
                    </div>
                    { for days.iter().map(render_day) }
                </div>
            </div>
        </div>
    }
}

/// A lesson in its lane: the subject and, where there is room for it, the room
fn render_lesson(lesson: &LessonBlock, position: String, now: NaiveDateTime) -> Html {
    let first_name = |wanted: fn(&Entity) -> bool| lesson.current_names(wanted).find(|name| !name.is_empty());
    let subject = first_name(Entity::is_subject).unwrap_or_else(|| lesson.kind.as_str().to_string());
    let room = first_name(Entity::is_room);

    let color = lesson.color_hex.trim_start_matches('#');
    let text_class = if is_dark_color(color) { "text-white" } else { "text-black" };
    let status_class = match lesson.status {
        LessonStatus::Cancelled => Some("compare-lesson-cancelled"),
        LessonStatus::Changed => Some("compare-lesson-changed"),
        LessonStatus::Additional => Some("compare-lesson-additional"),
        LessonStatus::Other(_) => lesson.is_exam().then_some("compare-lesson-exam"),
    };
    let is_over = lesson.time_range.end <= now;
    let title = format!("{subject} {}-{}", lesson.time_range.start.format("%H:%M"), lesson.time_range.end.format("%H:%M"));

    html! {
        <div class={classes!("compare-lesson", is_over.then_some("compare-lesson-past"))} style={position} {title}>
            <div class={classes!("compare-lesson-inner", text_class, status_class)} style={format!("background-color: #{color};")}>
                <span class="fw-semibold">{ subject }</span>
                if let Some(room) = room {
                    <span class="compare-lesson-detail">{ room }</span>
                }
            </div>
        </div>
    }
}
