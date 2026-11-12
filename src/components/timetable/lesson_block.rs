//! How the lessons that take place at the same time are drawn in the week grid.

use altis_core::models::timetable::{ChangeStatus, Entity, LessonBlock, LessonStatus};
use altis_core::settings::is_dark_color;
use chrono::{NaiveDateTime, TimeDelta};
use web_sys::MouseEvent;
use yew::prelude::*;

/// How a lesson that is over is drawn, and the part of a running one that is
const PAST_STYLE: &str = "filter: grayscale(0.4); opacity: 0.65;";

/// The time a group of lessons covers, which the lessons in it are placed against
struct GroupSpan {
    start: NaiveDateTime,
    seconds: f64,
}

impl GroupSpan {
    fn of(lessons: &[LessonBlock]) -> Option<Self> {
        let start = lessons.iter().map(|lesson| lesson.time_range.start).min()?;
        let end = lessons.iter().map(|lesson| lesson.time_range.end).max()?;
        Some(Self { start, seconds: (end - start).num_seconds() as f64 })
    }

    fn end(&self) -> NaiveDateTime {
        self.start + TimeDelta::seconds(self.seconds as i64)
    }

    /// How far into the group a stretch of time reaches, in percent of the group
    fn percent(&self, duration: TimeDelta) -> f64 {
        (duration.num_seconds() as f64 / self.seconds) * 100.0
    }
}

/// Where a lesson sits across its group, in percent of the group's width
struct Lane {
    left: f64,
    width: f64,
}

/// Which lessons a phone shows first when not all of a group fit next to each other
fn priority(lesson: &LessonBlock) -> u8 {
    if lesson.is_exam() {
        return 0;
    }
    match lesson.status {
        LessonStatus::Changed => 1,
        LessonStatus::Additional => 2,
        LessonStatus::Other(_) => 3,
        LessonStatus::Cancelled => 4,
    }
}

/// The most important lessons of a group, as many as fit without overlapping - so two single
/// lessons over a cancelled double one both show
fn most_important(lessons: &[LessonBlock]) -> Vec<&LessonBlock> {
    let mut by_priority: Vec<&LessonBlock> = lessons.iter().collect();
    by_priority.sort_by_key(|lesson| (priority(lesson), lesson.time_range.start));

    let mut shown: Vec<&LessonBlock> = Vec::new();
    for lesson in by_priority {
        if !shown.iter().any(|other| other.overlaps(lesson)) {
            shown.push(lesson);
        }
    }
    shown
}

/// Draws a group of lessons that overlap as one block of the day's column, `day_duration` being
/// the time the whole column stands for. A desktop shows all of them side by side; a phone shows
/// the most important ones and only counts the rest.
pub fn render_lesson_group(lessons: &[LessonBlock], day_duration: TimeDelta, now: NaiveDateTime, on_group_click: Callback<Vec<LessonBlock>>) -> Html {
    let Some(span) = GroupSpan::of(lessons) else { return html! {} };
    let height = (span.seconds / day_duration.num_seconds() as f64) * 100.0;

    let onclick = {
        let lessons = lessons.to_vec();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            on_group_click.emit(lessons.clone());
        })
    };

    let shown_on_phone = most_important(lessons);
    let hidden_on_phone = lessons.len() - shown_on_phone.len();
    let whole_width = Lane { left: 0.0, width: 100.0 };

    html! {
        <div class="w-100 group-block" style={format!("height: {height}%; position: relative; cursor: pointer;")} {onclick}>
            <div class="d-none d-md-block h-100 w-100">
                { render_side_by_side(lessons, &span, now) }
            </div>
            <div class="d-flex d-md-none h-100 w-100 mobile-p-half">
                <div style={format!("width: {}%; height: 100%; position: relative;", if hidden_on_phone > 0 { 80 } else { 100 })}>
                    { for shown_on_phone.iter().map(|lesson| render_lesson(lesson, &span, &whole_width, true, now)) }
                </div>
                if hidden_on_phone > 0 {
                    <div class="d-flex flex-column justify-content-center align-items-center text-white rounded ms-1 bg-primary"
                         style={format!("width: 20%; height: 100%; font-size: 0.8rem; opacity: 0.8; z-index: 10; {}", if span.end() <= now { PAST_STYLE } else { "" })}>
                        <span class="text-black fw-bold">{ hidden_on_phone }</span>
                    </div>
                }
            </div>
        </div>
    }
}

/// Every lesson of the group in a lane of its own, reusing a lane once the lesson in it is over
fn render_side_by_side(lessons: &[LessonBlock], span: &GroupSpan, now: NaiveDateTime) -> Html {
    let mut sorted = lessons.to_vec();
    sorted.sort_by_key(|lesson| lesson.time_range.start);

    // when the lesson in each lane ends
    let mut lane_ends: Vec<NaiveDateTime> = Vec::new();
    let lanes: Vec<usize> = sorted
        .iter()
        .map(|lesson| {
            let lane = lane_ends.iter().position(|&end| lesson.time_range.start >= end).unwrap_or(lane_ends.len());
            match lane_ends.get_mut(lane) {
                Some(end) => *end = lesson.time_range.end,
                None => lane_ends.push(lesson.time_range.end),
            }
            lane
        })
        .collect();

    let width = 100.0 / lane_ends.len().max(1) as f64;
    sorted.iter().zip(lanes).map(|(lesson, lane)| render_lesson(lesson, span, &Lane { left: lane as f64 * width, width }, false, now)).collect()
}

fn render_lesson(lesson: &LessonBlock, span: &GroupSpan, lane: &Lane, on_phone: bool, now: NaiveDateTime) -> Html {
    let (start, end) = (lesson.time_range.start, lesson.time_range.end);
    let top = span.percent(start - span.start);
    let height = span.percent(end - start);
    let Lane { left, width } = lane;

    let hex = lesson.color_hex.trim_start_matches('#');
    let mut style = format!("background-color: #{hex};");
    let text_class = if is_dark_color(hex) { "text-white" } else { "text-black" };
    let mut class = format!("rounded {text_class} text-center h-100 w-100 d-flex flex-column align-items-center justify-content-center overflow-hidden");

    match lesson.status {
        LessonStatus::Cancelled => {
            class += " border border-4 border-danger";
            style += "opacity: 0.33; background-image: repeating-linear-gradient(45deg, transparent, transparent 10px, rgba(255,0,0,0.1) 10px, rgba(255,0,0,0.1) 20px);";
        }
        LessonStatus::Changed => class += " border border-4 border-info",
        LessonStatus::Additional => class += " border border-4 border-success",
        LessonStatus::Other(_) if lesson.is_exam() => class += " border border-4 border-warning",
        LessonStatus::Other(_) => {}
    }

    let layer = |extra_style: String| html! {
        <div style={format!("position: absolute; top: {top}%; left: {left}%; height: {height}%; width: {width}%; padding: {}; {extra_style}", if on_phone { "0" } else { "0.2rem" })} class="lesson-container">
            <div class={class.clone()} style={style.clone()}>
                <div class="dynamic-text">
                    { render_entities(lesson, Entity::is_subject) } <br/>
                    { render_entities(lesson, Entity::is_teacher) } <br/>
                    { render_entities(lesson, Entity::is_room) }
                </div>
            </div>
        </div>
    };

    if now <= start {
        layer(String::new())
    } else if now >= end {
        layer(PAST_STYLE.to_string())
    } else {
        // lesson is in progress: stack a grayed copy clipped to the part that's already over
        let past_percent = (now - start).num_seconds() as f64 / (end - start).num_seconds() as f64 * 100.0;
        html! {
            <>
                { layer(format!("clip-path: inset({past_percent}% 0 0 0);")) }
                { layer(format!("clip-path: inset(0 0 {}% 0); {PAST_STYLE}", 100.0 - past_percent)) }
            </>
        }
    }
}

/// The lesson's entities of one sort, marked by what a change did to them: the one that was
/// replaced struck through, ahead of the one that replaced it
fn render_entities(lesson: &LessonBlock, wanted: fn(&Entity) -> bool) -> Html {
    let mut entities: Vec<_> = lesson.entities.iter().filter(|entity| wanted(&entity.inner)).collect();
    entities.sort_by_key(|entity| entity.status != ChangeStatus::Removed);

    entities
        .into_iter()
        .map(|entity| {
            let style = match entity.status {
                ChangeStatus::Removed => "background: #ffcccc; color: #b30000; padding: 0 2px;",
                ChangeStatus::New => "background: #ccffcc; color: #006600; padding: 0 2px;",
                ChangeStatus::Changed => "background: #ffffcc; color: #8a6d3b; padding: 0 2px;",
                ChangeStatus::Regular => "",
            };

            if entity.status == ChangeStatus::Removed {
                html! { <><del style={style}>{ entity.inner.name() }</del>{ " " }</> }
            } else {
                html! { <><span style={style}>{ entity.inner.name() }</span>{ " " }</> }
            }
        })
        .collect::<Html>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use altis_core::models::timetable::{LessonKind, TimeRange};
    use chrono::NaiveDate;

    fn lesson(start: u32, end: u32, status: LessonStatus) -> LessonBlock {
        let date = NaiveDate::from_ymd_opt(2026, 9, 14).unwrap();
        LessonBlock {
            time_range: TimeRange { start: date.and_hms_opt(start, 0, 0).unwrap(), end: date.and_hms_opt(end, 0, 0).unwrap() },
            status,
            ..Default::default()
        }
    }

    fn regular() -> LessonStatus {
        LessonStatus::from("REGULAR".to_string())
    }

    /// two single lessons replacing a cancelled double one are both more important than it
    #[test]
    fn a_phone_shows_what_takes_place_rather_than_what_was_cancelled() {
        let lessons = [lesson(8, 10, LessonStatus::Cancelled), lesson(8, 9, LessonStatus::Additional), lesson(9, 10, regular())];

        let shown = most_important(&lessons);
        assert_eq!(shown.len(), 2);
        assert!(shown.iter().all(|lesson| !lesson.is_cancelled()));
    }

    #[test]
    fn an_exam_goes_before_everything_else() {
        let exam = LessonBlock { kind: LessonKind::Exam, ..lesson(8, 9, regular()) };
        let lessons = [lesson(8, 9, LessonStatus::Changed), exam];

        let shown = most_important(&lessons);
        assert_eq!(shown.len(), 1);
        assert!(shown[0].is_exam());
    }

    #[test]
    fn a_group_spans_from_its_first_start_to_its_last_end() {
        let lessons = [lesson(9, 10, regular()), lesson(8, 11, regular())];

        let span = GroupSpan::of(&lessons).unwrap();
        assert_eq!(span.seconds, 3.0 * 3600.0);
        assert_eq!(span.percent(TimeDelta::minutes(90)), 50.0);
        assert_eq!(span.end(), lessons[1].time_range.end);
        assert!(GroupSpan::of(&[]).is_none());
    }
}
