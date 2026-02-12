use crate::data_models::clean_models::clean_models::{DayTimeTable, Entity, LessonBlock, TimeRange, WeekTimeTable};
use chrono::{Datelike, TimeDelta};
use yew::{function_component, html, Html, Properties};
use crate::persistence_manager::PersistenceManager;

#[derive(Properties, PartialEq, Clone)]
pub struct TimeTableRenderProps {
    pub timetable: WeekTimeTable,
}

#[function_component(TimeTableRender)]
pub fn time_table_render(props: &TimeTableRenderProps) -> Html {

    if PersistenceManager::get_settings().is_ok_and(|x| x.is_some_and(|x| x.visual_settings.force_ascii_timetable)) {
        return html! {
            <pre>
                { props.timetable.to_string_pretty(true, true, true, true, true) }
            </pre>
        }
    }

    let mut days: Vec<DayTimeTable> = props.timetable.days.clone();
    days.sort_by_key(|x| x.date);
    let lessons: Vec<LessonBlock> = days.iter().flat_map(|dtt| dtt.lessons.clone()).collect();
    if lessons.is_empty() {
        return html! { "No lessons!" };
    }

    let week_start_time = lessons.iter().map(|l| l.time_range.start).min().unwrap();
    let week_end_time = lessons.iter().map(|l| l.time_range.end).max().unwrap();

    html! {
        <>
            <div class="d-flex w-100 bg-dark border-bottom">
                <div style="width: 60px;" class="flex-shrink-0"></div>
                <div class="d-flex flex-grow-1">
                    { for days.iter().map(|day| {
                        let weekday = day.date.weekday().to_string();
                        let date_str = day.date.format("%d.%m").to_string();
                        html! {
                            <div class="flex-grow-1 text-center border-start pb-1" style="flex-basis: 0;">
                                <div class="fw-bold">{ weekday }</div>
                                <div class="small">{ date_str }</div>
                            </div>
                        }
                    })}
                </div>
            </div>


            <div class="d-flex flex-grow-1 w-100">
                <div style="width: 60px" class="d-flex flex-column justify-content-between align-items-end">
                    <div class="small pe-1">{ week_start_time.format("%H:%M").to_string() }</div>
                    <div class="small pe-1">{ week_end_time.format("%H:%M").to_string() }</div>
                </div>
                <div class="d-flex flex-grow-1">
                    { for days.iter().map(|day| html! {
                        <div
                            class="flex-grow-1 border-start position-relative flex"
                            style="flex-basis: 0; min-width: 0; overflow: hidden;"
                        >
                            { for group_by_time(fill_breaks(day.lessons.clone())).iter().map(|lesson| {
                                generate_lessons_html(lesson, week_end_time.time() - week_start_time.time())
                            })}
                        </div>
                    })}
                </div>
            </div>
        </>
    }
}

fn group_by_time(mut lessons: Vec<LessonBlock>) -> Vec<Vec<LessonBlock>> {
    lessons.sort_by_key(|l| l.time_range.start);
    let mut remaining = lessons;
    let mut res = vec![];
    while remaining.len() > 0 {
        let curr = remaining.pop().unwrap();
        let mut v = vec![curr.clone()];

        let mut i = 0;
        while i < remaining.len() {
            if remaining[i].overlaps(&curr) {
                v.push(remaining[i].clone());
                remaining.remove(i);
                continue;
            }
            i += 1;
        }

        res.push(v);
    }
    res.sort_by_key(|l| l.iter().min_by(|x, y|
        x.time_range.start.cmp(&y.time_range.start)).unwrap().time_range.start);
    res
}

fn fill_breaks(mut lessons: Vec<LessonBlock>) -> Vec<LessonBlock> {
    lessons.sort_by_key(|l| l.time_range.start);
    let mut result = Vec::new();
    let mut iter = lessons.into_iter().peekable();

    while let Some(curr) = iter.next() {
        let end = curr.time_range.end;
        result.push(curr);
        if let Some(next) = iter.peek() {
            if end < next.time_range.start {
                result.push(LessonBlock {
                    time_range: TimeRange { start: end, end: next.time_range.start },
                    r#type: "Break".into(),
                    ..Default::default()
                });
            }
        }
    }
    result
}

fn generate_lessons_html(lessons: &Vec<LessonBlock>, time_range: TimeDelta) -> Html {
    let earliest = lessons.iter().map(|l| l.time_range.start).min().unwrap();
    let latest = lessons.iter().map(|l| l.time_range.end).max().unwrap();
    let group_duration = (latest - earliest).num_seconds() as f64;
    let total = time_range.num_seconds() as f64;

    let height_style = format!("height: {}%;", (group_duration / total) * 100.0);
    let width = 100.0 / (lessons.len() as f64);

    html! {
        <div class="d-flex w-100" style={height_style}>
            { for lessons.iter().map(|lesson| {
                generate_lesson_html(lesson, group_duration, width)
            })}
        </div>
    }
}

fn generate_lesson_html(lesson: &LessonBlock, group_duration: f64, width: f64) -> Html {
    let duration = (lesson.time_range.end - lesson.time_range.start).num_seconds() as f64;
    let height_pct = (duration / group_duration) * 100.0;

    let outer_style = format!("height: {height_pct}%; width: {width}%;");
    let inner_style = format!("background-color: #{}; height: 100%; width: 100%;", lesson.color_hex);

    html! {
        <div style={outer_style} class="p-1">
            <div class="rounded text-black text-center h-100 w-100" style={inner_style}>
                { lesson.entities.iter()
                    .find(|x| matches!(x.inner, Entity::Subject(..)))
                    .map(|x| x.inner.name())
                    .unwrap_or_default()
                }
            </div>
        </div>
    }
}