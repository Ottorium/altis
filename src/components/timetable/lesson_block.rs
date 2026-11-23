//! How the lessons that take place at the same time are drawn in the week grid.

use altis_core::models::timetable::{ChangeStatus, Entity, LessonBlock, LessonStatus};
use altis_core::settings::{is_dark_color, LessonLine, LessonTextSettings, VisualSettings};
use chrono::{NaiveDateTime, TimeDelta};
use web_sys::MouseEvent;
use yew::prelude::*;

/// How a lesson that is over is drawn, and the part of a running one that is
const PAST_CLASS: &str = "lesson-past";

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
pub fn render_lesson_group(lessons: &[LessonBlock], day_duration: TimeDelta, now: NaiveDateTime, visual: &VisualSettings, on_group_click: Callback<Vec<LessonBlock>>) -> Html {
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
    let dims_the_past = visual.lesson_display.dim_past_lessons;

    html! {
        <div class="w-100 group-block" style={format!("height: {height}%;")} {onclick}>
            <div class="d-none d-md-block h-100 w-100">
                { render_side_by_side(lessons, &span, now, visual) }
            </div>
            <div class="d-flex d-md-none h-100 w-100 mobile-p-half">
                <div class="h-100 position-relative" style={format!("width: {}%;", if hidden_on_phone > 0 { 80 } else { 100 })}>
                    { for shown_on_phone.iter().map(|lesson| render_lesson(lesson, &span, &whole_width, true, now, visual)) }
                </div>
                if hidden_on_phone > 0 {
                    <div class={classes!("d-flex", "flex-column", "justify-content-center", "align-items-center", "h-100", "rounded", "ms-1", "bg-primary", "hidden-count",
                                         (dims_the_past && span.end() <= now).then_some(PAST_CLASS))}>
                        <span class="text-black fw-bold">{ hidden_on_phone }</span>
                    </div>
                }
            </div>
        </div>
    }
}

/// Every lesson of the group in a lane of its own, reusing a lane once the lesson in it is over
fn render_side_by_side(lessons: &[LessonBlock], span: &GroupSpan, now: NaiveDateTime, visual: &VisualSettings) -> Html {
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
    sorted.iter().zip(lanes).map(|(lesson, lane)| render_lesson(lesson, span, &Lane { left: lane as f64 * width, width }, false, now, visual)).collect()
}

fn render_lesson(lesson: &LessonBlock, span: &GroupSpan, lane: &Lane, on_phone: bool, now: NaiveDateTime, visual: &VisualSettings) -> Html {
    let (start, end) = (lesson.time_range.start, lesson.time_range.end);
    let top = span.percent(start - span.start);
    let height = span.percent(end - start);
    let Lane { left, width } = lane;

    let hex = lesson.color_hex.trim_start_matches('#');
    let color_style = format!("--lesson-color: #{hex};");
    let text_class = if is_dark_color(hex) { "text-white" } else { "text-black" };
    let status_class = match lesson.status {
        LessonStatus::Cancelled => Some("lesson-cancelled"),
        LessonStatus::Changed => Some("lesson-changed"),
        LessonStatus::Additional => Some("lesson-additional"),
        LessonStatus::Other(_) => lesson.is_exam().then_some("lesson-exam"),
    };

    // every line is sized to fit across the lesson, and all of them down it, see `.lesson-line`
    let lines: Vec<(usize, Html)> = visual.lesson_text.lines.iter().map(|line| render_line(lesson, *line, &visual.lesson_text)).filter(|(length, _)| *length > 0).collect();
    let fit_style = format!("--lesson-lines: {};", lines.len().max(1));

    // `clip` keeps part of the lesson: the part that is over, or the part that isn't yet
    let layer = |past: bool, clip: Option<String>| html! {
        <div style={format!("top: {top}%; left: {left}%; height: {height}%; width: {width}%;{}", clip.map(|clip| format!(" clip-path: {clip};")).unwrap_or_default())}
             class={classes!("lesson-container", on_phone.then_some("on-phone"), past.then_some(PAST_CLASS))}>
            <div class={classes!("lesson", text_class, status_class)} style={color_style.clone()}>
                <div class="dynamic-text" style={fit_style.clone()}>
                    { for lines.iter().map(|(length, line)| html! { <div class="lesson-line" style={format!("--lesson-chars: {length};")}>{ line.clone() }</div> }) }
                </div>
            </div>
        </div>
    };

    if now <= start || !visual.lesson_display.dim_past_lessons {
        layer(false, None)
    } else if now >= end {
        layer(true, None)
    } else {
        // lesson is in progress: stack a grayed copy clipped to the part that's already over
        let past_percent = (now - start).num_seconds() as f64 / (end - start).num_seconds() as f64 * 100.0;
        html! {
            <>
                { layer(false, Some(format!("inset({past_percent}% 0 0 0)"))) }
                { layer(true, Some(format!("inset(0 0 {}% 0)", 100.0 - past_percent))) }
            </>
        }
    }
}

/// One of the lines written on a lesson, along with how many characters it is long
fn render_line(lesson: &LessonBlock, line: LessonLine, text: &LessonTextSettings) -> (usize, Html) {
    let wanted: fn(&Entity) -> bool = match line {
        LessonLine::Subject => Entity::is_subject,
        LessonLine::Teacher => Entity::is_teacher,
        LessonLine::Room => Entity::is_room,
        LessonLine::Class => Entity::is_class,
        // a break is drawn like a lesson without anything on it, and stays that way
        LessonLine::Time if lesson.is_break() => return (0, html! {}),
        LessonLine::Time => {
            let time = format!("{}-{}", lesson.time_range.start.format("%H:%M"), lesson.time_range.end.format("%H:%M"));
            return (time.chars().count(), html! { time });
        }
    };
    let (length, names) = render_entities(lesson, wanted, |name| text.hides(line, name), text.long_names);
    match line == LessonLine::Room && text.mark_rooms && length > 0 {
        // the pin takes up about as much as a character and a half
        true => (length + 2, html! { <><i class="bi bi-geo-alt-fill lesson-room-pin" role="img" aria-label="Room"></i>{ names }</> }),
        false => (length, names),
    }
}

/// The lesson's entities of one sort, marked by what a change did to them: the one that was
/// replaced struck through, ahead of the one that replaced it. Leaves out the ones `hidden` says
/// to, by their short name. Along with how many characters they take up.
fn render_entities(lesson: &LessonBlock, wanted: fn(&Entity) -> bool, hidden: impl Fn(&str) -> bool, long_names: bool) -> (usize, Html) {
    let mut entities: Vec<_> = lesson.entities.iter().filter(|entity| wanted(&entity.inner) && !hidden(&entity.inner.name()) && !hidden(&entity.inner.long_name())).collect();
    entities.sort_by_key(|entity| entity.status != ChangeStatus::Removed);

    let names: Vec<_> = entities.into_iter().map(|entity| (entity, if long_names { entity.inner.long_name() } else { entity.inner.name() })).filter(|(_, name)| !name.is_empty()).collect();
    // the names and a space between each two
    let length = names.iter().map(|(_, name)| name.chars().count() + 1).sum::<usize>().saturating_sub(1);

    let html = names
        .into_iter()
        .enumerate()
        .map(|(index, (entity, name))| {
            let class = match entity.status {
                ChangeStatus::Removed => Some("entity-removed"),
                ChangeStatus::New => Some("entity-new"),
                ChangeStatus::Changed => Some("entity-changed"),
                ChangeStatus::Regular => None,
            };
            let space = if index > 0 { " " } else { "" };

            if entity.status == ChangeStatus::Removed {
                html! { <>{ space }<del class={class}>{ name }</del></> }
            } else {
                html! { <>{ space }<span class={class}>{ name }</span></> }
            }
        })
        .collect::<Html>();
    (length, html)
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
