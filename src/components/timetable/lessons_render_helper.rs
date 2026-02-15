use crate::data_models::clean_models::clean_models::{ChangeStatus, Entity, LessonBlock};
use chrono::{NaiveDateTime, TimeDelta};
use web_sys::MouseEvent;
use yew::{html, Callback, Html};

pub fn generate_lessons_html(
    lessons: &[LessonBlock],
    time_range: TimeDelta,
    on_group_click: Callback<Vec<LessonBlock>>,
) -> Html {
    if lessons.is_empty() { return html! {}; }

    let (start, end) = (lessons.iter().map(|l| l.time_range.start).min().unwrap(),
                        lessons.iter().map(|l| l.time_range.end).max().unwrap());
    let (group_duration, total) = ((end - start).num_seconds() as f64, time_range.num_seconds() as f64);

    let lessons_to_emit = lessons.to_vec();
    let onclick = Callback::from(move |e: MouseEvent| {
        e.stop_propagation();
        on_group_click.emit(lessons_to_emit.clone());
    });

    let priority = lessons.iter().find(|l| l.r#type == "EXAM")
        .or_else(|| lessons.iter().find(|l| l.status == "CHANGED"))
        .or_else(|| lessons.iter().find(|l| l.status == "ADDITIONAL"))
        .or_else(|| lessons.iter().find(|l| l.status != "CANCELLED"))
        .unwrap_or(&lessons[0]);

    html! {
        <div class="w-100 group-block" style={format!("height: {}%; position: relative; cursor: pointer;", (group_duration / total) * 100.0)} {onclick}>
            <style>
                { ".lesson-container { container-type: size; }
                   .dynamic-text { font-size: clamp(0.7rem, 19cqw, 1.1rem); line-height: 1.1; width: 100%; word-wrap: break-word; }
                   .border-info { border-color: #00d4ff !important; box-shadow: inset 0 0 15px #00d4ff; }
                   .border-success { border-color: #00ab5b !important; box-shadow: inset 0 0 15px #00ab5b; }
                   .border-warning { border-color: #ffcc00 !important; box-shadow: inset 0 0 15px #ffcc00; }
                   .border-danger { border-color: #ff4d4d !important; }
                   .mobile-p-half { padding: 0.125rem !important; }" }
            </style>
            <div class="d-none d-md-block h-100 w-100">
                { render_lanes(lessons, group_duration, start) }
            </div>
            <div class="d-flex d-md-none h-100 w-100 mobile-p-half">
                <div style={format!("width: {}%; height: 100%; position: relative;", if lessons.len() > 1 { 80 } else { 100 })}>
                    { render_lesson(priority, group_duration, start, 100.0, 0.0, true) }
                </div>
                if lessons.len() > 1 {
                    <div class="d-flex flex-column justify-content-center align-items-center text-white rounded ms-1 bg-primary"
                         style="width: 20%; height: 100%; font-size: 0.8rem; opacity: 0.8; z-index: 10;">
                        <span class="text-black fw-bold">{ lessons.len() - 1 }</span>
                    </div>
                }
            </div>
        </div>
    }
}

fn render_lanes(lessons: &[LessonBlock], group_duration: f64, start: NaiveDateTime) -> Html {
    let mut lanes: Vec<NaiveDateTime> = Vec::new();
    let mut sorted = lessons.to_vec();
    sorted.sort_by_key(|l| l.time_range.start);

    let assignments: Vec<usize> = sorted.iter().map(|l| {
        let idx = lanes.iter().position(|&e| l.time_range.start >= e).unwrap_or(lanes.len());
        if idx == lanes.len() { lanes.push(l.time_range.end); } else { lanes[idx] = l.time_range.end; }
        idx
    }).collect();

    let width = 100.0 / lanes.len().max(1) as f64;
    sorted.iter()
        .zip(assignments)
        .map(|(l, idx)| render_lesson(l, group_duration, start, width, idx as f64 * width, false))
        .collect()
}

fn render_lesson(lesson: &LessonBlock, group_duration: f64, group_start: NaiveDateTime, width: f64, x_offset: f64, is_mobile: bool) -> Html {
    let top = ((lesson.time_range.start - group_start).num_seconds() as f64 / group_duration) * 100.0;
    let h = ((lesson.time_range.end - lesson.time_range.start).num_seconds() as f64 / group_duration) * 100.0;

    let mut style = format!("background-color: #{};", lesson.color_hex);
    let mut cls = "rounded text-black text-center h-100 w-100 d-flex flex-column align-items-center justify-content-center overflow-hidden".to_string();

    match lesson.status.as_str() {
        "CANCELLED" => {
            cls += " border border-4 border-danger opacity-50";
            style += "background-image: repeating-linear-gradient(45deg, transparent, transparent 10px, rgba(255,0,0,0.1) 10px, rgba(255,0,0,0.1) 20px);";
        },
        "CHANGED" => cls += " border border-4 border-info",
        "ADDITIONAL" => cls += " border border-4 border-success",
        _ if lesson.r#type == "EXAM" => cls += " border border-4 border-warning",
        _ => {}
    }

    html! {
        <div style={format!("position: absolute; top: {top}%; left: {x_offset}%; height: {h}%; width: {width}%; padding: {};", if is_mobile { "0" } else { "0.2rem" })} class="lesson-container">
            <div class={cls} style={style}>
                <div class="dynamic-text">
                    { render_entity(lesson, |e| matches!(e, Entity::Subject(..))) } <br/>
                    { render_entity(lesson, |e| matches!(e, Entity::Teacher(..))) } <br/>
                    { render_entity(lesson, |e| matches!(e, Entity::Room(..))) }
                </div>
            </div>
        </div>
    }
}

fn render_entity(lesson: &LessonBlock, filter: fn(&Entity) -> bool) -> Html {
    let mut ents: Vec<_> = lesson.entities.iter().filter(|t| filter(&t.inner)).collect();
    ents.sort_by_key(|t| t.status != ChangeStatus::Removed);

    ents.into_iter().map(|t| {
        let style = match t.status {
            ChangeStatus::Removed => "background: #ffcccc; color: #b30000; padding: 0 2px;",
            ChangeStatus::New => "background: #ccffcc; color: #006600; padding: 0 2px;",
            ChangeStatus::Changed => "background: #ffffcc; color: #8a6d3b; padding: 0 2px;",
            _ => "",
        };

        if t.status == ChangeStatus::Removed {
            html! { <><del style={style}>{ t.inner.name() }</del>{ " " }</> }
        } else {
            html! { <><span style={style}>{ t.inner.name() }</span>{ " " }</> }
        }
    }).collect::<Html>()
}