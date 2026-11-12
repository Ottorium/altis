//! How a day's lessons are laid out in the week grid.

use altis_core::models::timetable::{LessonBlock, LessonKind, TimeRange};
use chrono::{NaiveDateTime, NaiveTime};

/// A section of the time column during which some lesson takes place
#[derive(Debug, PartialEq)]
pub struct TimeSlot {
    pub start: NaiveTime,
    pub end: NaiveTime,
    /// whether the start/end time is written inside this slot, times shared by two slots are only written in one of them
    pub label_start: bool,
    pub label_end: bool,
    /// no slot ends where this one starts
    pub after_break: bool,
}

/// Splits the time column at every lesson start and end, so each of those times gets a label
pub fn time_slots(lessons: &[LessonBlock]) -> Vec<TimeSlot> {
    let mut bounds: Vec<NaiveTime> = lessons.iter().flat_map(|lesson| [lesson.time_range.start.time(), lesson.time_range.end.time()]).collect();
    bounds.sort();
    bounds.dedup();

    let is_covered = |start: NaiveTime, end: NaiveTime| lessons.iter().any(|lesson| lesson.time_range.start.time() <= start && end <= lesson.time_range.end.time());
    let spans: Vec<(NaiveTime, NaiveTime)> = bounds.windows(2).map(|pair| (pair[0], pair[1])).filter(|&(start, end)| is_covered(start, end)).collect();

    // a time shared by two slots is written in the longer one, so it doesn't collide with the labels of a short slot
    (0..spans.len())
        .map(|index| {
            let (start, end) = spans[index];
            let duration = end - start;
            let previous = index.checked_sub(1).map(|previous| spans[previous]).filter(|previous| previous.1 == start);
            let next = spans.get(index + 1).filter(|next| next.0 == end);
            TimeSlot {
                start,
                end,
                label_start: previous.is_none_or(|(start, end)| end - start < duration),
                label_end: next.is_none_or(|(start, end)| *end - *start <= duration),
                after_break: index > 0 && previous.is_none(),
            }
        })
        .collect()
}

/// Groups the lessons that overlap, directly or through another one, so each group can be drawn
/// as one block of lessons side by side. The groups come in the order of the day, the lessons of
/// a group longest first.
pub fn group_by_time(mut lessons: Vec<LessonBlock>) -> Vec<Vec<LessonBlock>> {
    lessons.sort_by_key(|lesson| lesson.time_range.start);
    let mut remaining = lessons;
    let mut groups = vec![];

    while !remaining.is_empty() {
        let mut group = vec![remaining.remove(0)];

        // a lesson that joins the group can pull in ones that were passed over before, so the
        // search starts over whenever one does
        let mut index = 0;
        while index < remaining.len() {
            if group.iter().any(|member| remaining[index].overlaps(member)) {
                group.push(remaining.remove(index));
                index = 0;
            } else {
                index += 1;
            }
        }

        group.sort_by(|a, b| {
            let duration = |lesson: &LessonBlock| lesson.time_range.end - lesson.time_range.start;
            duration(b).cmp(&duration(a)).then(a.time_range.start.cmp(&b.time_range.start))
        });
        groups.push(group);
    }

    // the groups don't overlap, so the start of any of their lessons puts them in order
    groups.sort_by_key(|group| group[0].time_range.start);
    groups
}

fn break_between(start: NaiveDateTime, end: NaiveDateTime) -> LessonBlock {
    LessonBlock { time_range: TimeRange { start, end }, kind: LessonKind::Break, ..Default::default() }
}

/// Puts a break into every gap between two lessons and before the first one, so the day's blocks
/// stack up to the height of the grid without having to be positioned
pub fn fill_breaks(mut lessons: Vec<LessonBlock>, earliest: NaiveTime) -> Vec<LessonBlock> {
    lessons.sort_by_key(|lesson| lesson.time_range.start);
    let mut filled = Vec::new();
    let mut lessons = lessons.into_iter().peekable();

    if let Some(first) = lessons.peek()
        && first.time_range.start.time() != earliest {
        filled.push(break_between(first.time_range.start.date().and_time(earliest), first.time_range.start));
    }

    while let Some(lesson) = lessons.next() {
        let end = lesson.time_range.end;
        filled.push(lesson);
        if let Some(next) = lessons.peek()
            && end < next.time_range.start {
            filled.push(break_between(end, next.time_range.start));
        }
    }
    filled
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

    fn times(lessons: &[LessonBlock]) -> Vec<(NaiveTime, NaiveTime)> {
        lessons.iter().map(|lesson| (lesson.time_range.start.time(), lesson.time_range.end.time())).collect()
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

    #[test]
    fn groups_lessons_that_overlap_through_another_one() {
        let groups = group_by_time(vec![
            lesson(14, (10, 0), (10, 45)),
            lesson(14, (8, 0), (8, 45)),
            // overlaps the first period, and through the double period the second one does too
            lesson(14, (8, 45), (9, 30)),
            lesson(14, (8, 0), (9, 30)),
        ]);

        assert_eq!(groups.len(), 2);
        // the longest lesson of a group comes first
        assert_eq!(times(&groups[0]), vec![(t(8, 0), t(9, 30)), (t(8, 0), t(8, 45)), (t(8, 45), t(9, 30))]);
        assert_eq!(times(&groups[1]), vec![(t(10, 0), t(10, 45))]);
    }

    #[test]
    fn fills_the_gaps_of_a_day_with_breaks() {
        let filled = fill_breaks(vec![lesson(14, (10, 0), (10, 45)), lesson(14, (8, 0), (8, 45)), lesson(14, (8, 45), (9, 30))], t(7, 50));

        assert_eq!(times(&filled), vec![
            (t(7, 50), t(8, 0)),
            (t(8, 0), t(8, 45)),
            (t(8, 45), t(9, 30)),
            (t(9, 30), t(10, 0)),
            (t(10, 0), t(10, 45)),
        ]);
        assert_eq!(filled.iter().map(LessonBlock::is_break).collect::<Vec<_>>(), vec![true, false, false, true, false]);
    }

    #[test]
    fn a_day_that_starts_with_the_grid_needs_no_break_before_it() {
        let filled = fill_breaks(vec![lesson(14, (8, 0), (8, 45))], t(8, 0));
        assert_eq!(filled.len(), 1);
    }
}
