use crate::components::timetable::group_modal::GroupDetailModal;
use crate::components::timetable::lessons_render_helper::generate_lessons_html;
use crate::data_models::clean_models::untis::{DayTimeTable, LessonBlock, TimeRange, WeekTimeTable};
use crate::persistence_manager::PersistenceManager;
use chrono::{Datelike, Local, NaiveDateTime, NaiveTime};
use gloo_timers::callback::Interval;
use yew::{Callback, Html, Properties, function_component, html, use_effect_with, use_state};

#[derive(Properties, PartialEq, Clone)]
pub struct TimeTableRenderProps {
    pub timetable: WeekTimeTable,
}

#[function_component(TimeTableRender)]
pub fn time_table_render(props: &TimeTableRenderProps) -> Html {
    let selected_group = use_state(|| None::<Vec<LessonBlock>>);
    let now = use_state(current_time);

    {
        let now = now.clone();
        use_effect_with((), move |_| {
            let interval = Interval::new(30_000, move || now.set(current_time()));
            move || drop(interval)
        });
    }

    let on_group_click = {
        let selected_group = selected_group.clone();
        Callback::from(move |lessons: Vec<LessonBlock>| {
            if lessons.iter().any(|l| l.r#type != "Break") {
                selected_group.set(Some(lessons));
            }
        })
    };

    let on_close = {
        let selected_group = selected_group.clone();
        Callback::from(move |_| selected_group.set(None))
    };


    let visual_settings = PersistenceManager::get_settings().ok().flatten()
        .map(|settings| settings.visual_settings).unwrap_or_default();
    let mut days: Vec<DayTimeTable> = props.timetable.days.iter()
        .filter(|day| visual_settings.weekday_override.should_show(day.date.weekday(), !day.lessons.is_empty()))
        .cloned().collect();
    days.sort_by_key(|x| x.date);

    for day in &mut days {
        for lesson in &mut day.lessons {
            if let Some(color) = visual_settings.get_lesson_color_override(lesson) {
                lesson.color_hex = color.trim_start_matches('#').to_string();
            }
        }
    }

    if visual_settings.force_ascii_timetable {
        let timetable = WeekTimeTable { days };
        return html! {
            <div class="d-flex flex-grow-1 flex-column">
                <pre>
                    { timetable.to_string_pretty(true, true, true, true, true) }
                </pre>
            </div>
        };
    }

    let lessons: Vec<LessonBlock> = days.iter().flat_map(|dtt| dtt.lessons.clone()).collect();
    if lessons.is_empty() {
        return html! {
            <div class="d-flex flex-grow-1 flex-column">
                {"No lessons!"}
            </div>
        };
    }

    let mut start_times = lessons.iter().map(|l| l.time_range.start.time()).collect::<Vec<_>>();
    let mut end_times = lessons.iter().map(|l| l.time_range.end.time()).collect::<Vec<_>>();
    start_times.sort();
    end_times.sort();
    start_times.dedup();
    end_times.dedup();

    let min_time = *start_times.first().unwrap();
    let max_time = *end_times.last().unwrap();
    let total_duration = (max_time - min_time).num_seconds() as f64;

    let mut start_end_times: Vec<(NaiveTime, NaiveTime)> = Vec::new();
    let mut ei = 0;
    let mut it = start_times.iter().peekable();
    while let Some(&s) = it.next() {
        while ei < end_times.len() && s >= end_times[ei] { ei += 1; }
        if ei >= end_times.len() { break; }
        if let Some(&&next_s) = it.peek() && end_times[ei] > next_s { continue; }
        start_end_times.push((s, end_times[ei]));
    }

    html! {
        <>
            { if let Some(lessons) = (*selected_group).clone() {
                html! { <GroupDetailModal {lessons} on_close={on_close} /> }
            } else { html! {} } }

            <div
                class="d-flex flex-grow-1 flex-column h-100 w-100 overflow-hidden"
            >
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
                    <div style="width: 60px; position: relative;" class="d-flex flex-column flex-shrink-0">
                        {{
                            let mut last_end: Option<NaiveTime> = None;
                            start_end_times.iter().map(|(s, e)| {
                                let top = ((*s - min_time).num_seconds() as f64 / total_duration) * 100.0;
                                let height = ((*e - *s).num_seconds() as f64 / total_duration) * 100.0;

                                let show_start = last_end != Some(*s);
                                let p = last_end;
                                last_end = Some(*e);

                                html! {
                                    <div style={format!("position: absolute; top: {top}%; height: {height}%; width: 100%;")}
                                          class={format!("d-flex flex-column justify-content-between align-items-end border-bottom {} m-0", if show_start && p.is_some() { "border-top" } else { "" })}>
                                        <div class="small pe-1">
                                            { if show_start { s.format("%H:%M").to_string() } else { "".to_string() } }
                                        </div>
                                        <div class="small pe-1">{ e.format("%H:%M").to_string() }</div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()
                        }}
                    </div>
                    <div class="d-flex flex-grow-1">
                        { for days.iter().map(|day| {
                            let now_top = (day.date == now.date() && (min_time..=max_time).contains(&now.time()))
                                .then(|| ((now.time() - min_time).num_seconds() as f64 / total_duration) * 100.0);
                            html! {
                                <div class="flex-grow-1 border-start position-relative flex" style="flex-basis: 0; min-width: 0; overflow: hidden;">
                                    { for group_by_time(fill_breaks(day.lessons.clone(), min_time)).iter().map(|lessons| {
                                        generate_lessons_html(lessons, max_time - min_time, *now, on_group_click.clone())
                                    })}
                                    if let Some(top) = now_top {
                                        <div style={format!("position: absolute; top: {top}%; left: 0; right: 0; height: 2px; transform: translateY(-1px); background: #ff3b30; z-index: 20; pointer-events: none;")}>
                                            <div style="position: absolute; left: 0; top: -4px; width: 0; height: 0; border-top: 5px solid transparent; border-bottom: 5px solid transparent; border-left: 7px solid #ff3b30;"></div>
                                            <div style="position: absolute; right: 0; top: -4px; width: 0; height: 0; border-top: 5px solid transparent; border-bottom: 5px solid transparent; border-right: 7px solid #ff3b30;"></div>
                                        </div>
                                    }
                                </div>
                            }
                        })}
                    </div>
                </div>
            </div>
        </>
    }
}

fn current_time() -> NaiveDateTime {
    Local::now().naive_local()
}

fn group_by_time(mut lessons: Vec<LessonBlock>) -> Vec<Vec<LessonBlock>> {
    lessons.sort_by_key(|l| l.time_range.start);
    let mut remaining = lessons;
    let mut res = vec![];

    while !remaining.is_empty() {
        let mut group = vec![remaining.remove(0)];

        let mut i = 0;
        while i < remaining.len() {
            let mut overlaps_any = false;
            for member in &group {
                if remaining[i].overlaps(member) {
                    overlaps_any = true;
                    break;
                }
            }

            if overlaps_any {
                group.push(remaining.remove(i));
                i = 0;
            } else {
                i += 1;
            }
        }

        group.sort_by(|a, b| {
            let duration_a = a.time_range.end - a.time_range.start;
            let duration_b = b.time_range.end - b.time_range.start;

            duration_b.cmp(&duration_a)
                .then(a.time_range.start.cmp(&b.time_range.start))
        });

        res.push(group);
    }

    // we can get away with using [0] because the groups don't overlap
    res.sort_by_key(|g| g[0].time_range.start);
    res
}

fn fill_breaks(mut lessons: Vec<LessonBlock>, earliest: NaiveTime) -> Vec<LessonBlock> {
    lessons.sort_by_key(|l| l.time_range.start);
    let mut result = Vec::new();
    let mut iter = lessons.into_iter().peekable();

    if let Some(first) = iter.peek()
        && first.time_range.start.time() != earliest {
        result.push(LessonBlock {
            time_range: TimeRange { start: first.time_range.start.date().and_time(earliest), end: first.time_range.start },
            r#type: "Break".into(),
            ..Default::default()
        });
    }

    while let Some(curr) = iter.next() {
        let end = curr.time_range.end;
        result.push(curr);
        if let Some(next) = iter.peek()
            && end < next.time_range.start {
            result.push(LessonBlock {
                time_range: TimeRange { start: end, end: next.time_range.start },
                r#type: "Break".into(),
                ..Default::default()
            });
        }
    }
    result
}
