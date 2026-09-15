use crate::components::timetable::group_modal::GroupDetailModal;
use crate::components::timetable::lessons_render_helper::generate_lessons_html;
use crate::persistence_manager::PersistenceManager;
use altis_core::data_models::clean_models::untis::{DayTimeTable, Entity, LessonBlock, TimeRange, WeekTimeTable};
use chrono::{Datelike, Local, NaiveDateTime, NaiveTime};
use gloo_timers::callback::Interval;
use yew::{Callback, Html, Properties, function_component, html, use_effect_with, use_state};

#[derive(Properties, PartialEq, Clone)]
pub struct TimeTableRenderProps {
    pub timetable: WeekTimeTable,
    /// a teacher, class or room was picked in the lesson details, to show its timetable
    pub on_entity_select: Callback<Entity>,
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

    let on_entity_click = {
        let selected_group = selected_group.clone();
        let on_entity_select = props.on_entity_select.clone();
        Callback::from(move |entity| {
            selected_group.set(None);
            on_entity_select.emit(entity);
        })
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

    let min_time = lessons.iter().map(|l| l.time_range.start.time()).min().unwrap();
    let max_time = lessons.iter().map(|l| l.time_range.end.time()).max().unwrap();
    let total_duration = (max_time - min_time).num_seconds() as f64;
    let slots = time_slots(&lessons);

    html! {
        <>
            { if let Some(lessons) = (*selected_group).clone() {
                html! { <GroupDetailModal {lessons} on_close={on_close} {on_entity_click} /> }
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
                        { for slots.iter().map(|slot| {
                            let top = ((slot.start - min_time).num_seconds() as f64 / total_duration) * 100.0;
                            let height = ((slot.end - slot.start).num_seconds() as f64 / total_duration) * 100.0;
                            let label = |show: bool, time: NaiveTime| if show { time.format("%H:%M").to_string() } else { String::new() };

                            html! {
                                <div style={format!("position: absolute; top: {top}%; height: {height}%; width: 100%;")}
                                      class={format!("d-flex flex-column justify-content-between align-items-end border-bottom {} m-0", if slot.after_break { "border-top" } else { "" })}>
                                    <div class="small pe-1">{ label(slot.label_start, slot.start) }</div>
                                    <div class="small pe-1">{ label(slot.label_end, slot.end) }</div>
                                </div>
                            }
                        })}
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

/// A section of the time column during which some lesson takes place
#[derive(Debug, PartialEq)]
struct TimeSlot {
    start: NaiveTime,
    end: NaiveTime,
    /// whether the start/end time is written inside this slot, times shared by two slots are only written in one of them
    label_start: bool,
    label_end: bool,
    /// no slot ends where this one starts
    after_break: bool,
}

/// Splits the time column at every lesson start and end, so each of those times gets a label
fn time_slots(lessons: &[LessonBlock]) -> Vec<TimeSlot> {
    let mut bounds: Vec<NaiveTime> = lessons.iter()
        .flat_map(|l| [l.time_range.start.time(), l.time_range.end.time()])
        .collect();
    bounds.sort();
    bounds.dedup();

    let spans: Vec<(NaiveTime, NaiveTime)> = bounds.windows(2)
        .map(|w| (w[0], w[1]))
        .filter(|&(s, e)| lessons.iter().any(|l| l.time_range.start.time() <= s && e <= l.time_range.end.time()))
        .collect();

    // a time shared by two slots is written in the longer one, so it doesn't collide with the labels of a short slot
    (0..spans.len()).map(|i| {
        let (start, end) = spans[i];
        let duration = end - start;
        let prev = i.checked_sub(1).map(|p| spans[p]).filter(|p| p.1 == start);
        let next = spans.get(i + 1).filter(|n| n.0 == end);
        TimeSlot {
            start,
            end,
            label_start: prev.is_none_or(|(s, e)| e - s < duration),
            label_end: next.is_none_or(|(s, e)| *e - *s <= duration),
            after_break: i > 0 && prev.is_none(),
        }
    }).collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn lesson(day: u32, start: (u32, u32), end: (u32, u32)) -> LessonBlock {
        let date = NaiveDate::from_ymd_opt(2026, 9, day).unwrap();
        LessonBlock {
            time_range: TimeRange {
                start: date.and_hms_opt(start.0, start.1, 0).unwrap(),
                end: date.and_hms_opt(end.0, end.1, 0).unwrap(),
            },
            ..Default::default()
        }
    }

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    /// every time written in the column, in order
    fn labels(slots: &[TimeSlot]) -> Vec<NaiveTime> {
        slots.iter()
            .flat_map(|s| [s.label_start.then_some(s.start), s.label_end.then_some(s.end)])
            .flatten()
            .collect()
    }

    #[test]
    fn shows_latest_end_of_the_week() {
        let slots = time_slots(&[
            lesson(14, (8, 0), (8, 45)),
            lesson(14, (8, 45), (9, 30)),
            lesson(15, (8, 0), (8, 45)),
            lesson(16, (8, 0), (10, 15)),
        ]);
        assert_eq!(labels(&slots), vec![t(8, 0), t(8, 45), t(9, 30), t(10, 15)]);
    }

    #[test]
    fn shows_every_end_between_starts() {
        let slots = time_slots(&[
            lesson(14, (8, 0), (8, 45)),
            lesson(15, (8, 0), (9, 30)),
            lesson(14, (10, 0), (10, 45)),
        ]);
        assert_eq!(labels(&slots), vec![t(8, 0), t(8, 45), t(9, 30), t(10, 0), t(10, 45)]);
        assert_eq!(slots.iter().map(|s| s.after_break).collect::<Vec<_>>(), vec![false, false, true]);
    }

    #[test]
    fn shows_overlapping_lessons() {
        let slots = time_slots(&[
            lesson(14, (8, 0), (9, 0)),
            lesson(15, (8, 30), (9, 30)),
        ]);
        assert_eq!(labels(&slots), vec![t(8, 0), t(8, 30), t(9, 0), t(9, 30)]);
    }

    #[test]
    fn writes_shared_times_in_the_longer_slot() {
        // the double lesson creates a 5 minute slot between 9:30 and 9:35
        let slots = time_slots(&[
            lesson(14, (8, 45), (9, 30)),
            lesson(14, (9, 35), (10, 20)),
            lesson(15, (8, 45), (9, 35)),
        ]);
        let short = slots.iter().find(|s| s.start == t(9, 30)).unwrap();
        assert!(!short.label_start && !short.label_end);
        assert_eq!(labels(&slots), vec![t(8, 45), t(9, 30), t(9, 35), t(10, 20)]);
    }
}
