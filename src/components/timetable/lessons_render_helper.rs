use crate::data_models::clean_models::clean_models::{ChangeStatus, Entity, LessonBlock};
use chrono::{NaiveDateTime, TimeDelta};
use yew::{html, Html};

pub fn generate_lessons_html(lessons: &Vec<LessonBlock>, time_range: TimeDelta) -> Html {
    if lessons.is_empty() {
        return html! {};
    }

    let earliest = lessons.iter().map(|l| l.time_range.start).min().unwrap();
    let latest = lessons.iter().map(|l| l.time_range.end).max().unwrap();
    let group_duration = (latest - earliest).num_seconds() as f64;
    let total = time_range.num_seconds() as f64;

    let mut sorted_lessons = lessons.clone();
    sorted_lessons.sort_by_key(|l| l.time_range.start);

    let mut lanes = Vec::new();
    let mut lesson_lane_assignments = Vec::new();

    for lesson in &sorted_lessons {
        let mut assigned_lane = None;

        for (idx, lane_end_time) in lanes.iter_mut().enumerate() {
            // If this lesson starts after the last one in this lane ended, take that spot
            if lesson.time_range.start >= *lane_end_time {
                *lane_end_time = lesson.time_range.end;
                assigned_lane = Some(idx);
                break;
            }
        }

        if assigned_lane.is_none() {
            lanes.push(lesson.time_range.end);
            assigned_lane = Some(lanes.len() - 1);
        }
        lesson_lane_assignments.push(assigned_lane.unwrap());
    }

    let total_lanes = lanes.len().max(1) as f64;
    let width = 100.0 / total_lanes;
    let height_style = format!("height: {}%; position: relative;", (group_duration / total) * 100.0);

    html! {
        <div class="w-100" style={height_style}>
            { for sorted_lessons.iter().zip(lesson_lane_assignments.iter()).map(|(lesson, lane_idx)| {
                let offset_x = (*lane_idx as f64) * width;
                generate_lesson_html(lesson, group_duration, earliest, width, offset_x)
            })}
        </div>
    }
}

fn generate_lesson_html(lesson: &LessonBlock, group_duration: f64, group_start: NaiveDateTime, width: f64, offset_x: f64) -> Html {
    let start_offset = (lesson.time_range.start - group_start).num_seconds() as f64;
    let duration = (lesson.time_range.end - lesson.time_range.start).num_seconds() as f64;

    let top_pct = (start_offset / group_duration) * 100.0;
    let height_pct = (duration / group_duration) * 100.0;

    let outer_style = format!("position: absolute; top: {top_pct}%; left: {offset_x}%; height: {height_pct}%; width: {width}%;");

    let mut inner_style = format!("background-color: #{};", lesson.color_hex);
    let mut border_classes = "rounded text-black text-center h-100 w-100 d-flex flex-column align-items-center justify-content-center overflow-hidden".to_string();

    if lesson.status == "CANCELLED" {
        border_classes += " border border-4 border-danger opacity-50";
        inner_style += "background-image: repeating-linear-gradient(45deg, transparent, transparent 10px, rgba(255,0,0,0.1) 10px, rgba(255,0,0,0.1) 20px);";
    } else if lesson.status == "CHANGED" {
        border_classes += " border border-4 border-info";
    } else if lesson.status == "ADDITIONAL" {
        border_classes += " border border-4 border-success";
    } else if lesson.r#type == "EXAM" {
        border_classes += " border border-4 border-warning";
    }

    html! {
        <div style={outer_style} class="p-custom">
            <style>
                { "
                .p-custom { padding: 0.2rem; }
                .border-info { border-color: #00d4ff !important; box-shadow: inset 0 0 15px #00d4ff; }
                .border-success { border-color: #00ab5b !important; box-shadow: inset 0 0 15px #00ab5b; }
                .border-warning { border-color: #ffcc00 !important; box-shadow: inset 0 0 15px #ffcc00; }
                .border-danger { border-color: #ff4d4d !important; }
                " }
            </style>
            <div class={border_classes} style={inner_style}>
                <span>
                    { render_entities(lesson, |e| matches!(e, Entity::Subject(..))) }
                    <br/>
                    { render_entities(lesson, |e| matches!(e, Entity::Teacher(..))) }
                    <br/>
                    { render_entities(lesson, |e| matches!(e, Entity::Room(..))) }
                </span>
            </div>
        </div>
    }
}
fn render_entities(lesson: &LessonBlock, variant_match: fn(&Entity) -> bool) -> Html {
    let mut filtered_entities: Vec<_> = lesson.entities.iter()
        .filter(|tracked| variant_match(&tracked.inner))
        .collect();

    filtered_entities.sort_by_key(|tracked| {
        if tracked.status == ChangeStatus::Removed { 0 } else { 1 }
    });

    let len = filtered_entities.len();

    filtered_entities.into_iter().enumerate().map(|(i, tracked)| {
        let name = tracked.inner.name();
        let separator = if i < len - 1 { ", " } else { "" };

        let style = match tracked.status {
            ChangeStatus::Removed => "background-color: #ffcccc; color: #b30000; padding: 0 2px;",
            ChangeStatus::New => "background-color: #ccffcc; color: #006600; padding: 0 2px;",
            ChangeStatus::Changed => "background-color: #ffffcc; color: #8a6d3b; padding: 0 2px;",
            ChangeStatus::Regular => "",
        };

        if tracked.status == ChangeStatus::Removed {
            html! {
                <><del style={style}>{ name }</del>{ separator }</>
            }
        } else {
            html! {
                <><span style={style}>{ name }</span>{ separator }</>
            }
        }
    }).collect::<Html>()
}