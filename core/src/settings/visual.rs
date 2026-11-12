use super::default_true;
use crate::models::timetable::{ChangeStatus, Entity, LessonBlock};
use chrono::{NaiveTime, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const ALL_WEEKDAYS: [Weekday; 7] = [Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri, Weekday::Sat, Weekday::Sun];
const WORK_WEEKDAYS: [Weekday; 5] = [Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri];

/// Below this the text on a colour is written in white rather than black
const DARK_LUMINANCE: f64 = 140.0;

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct VisualSettings {
    #[serde(default)]
    pub force_ascii_timetable: bool,
    #[serde(default)]
    pub time_padding: TimePadding,
    #[serde(default)]
    pub weekday_override: WeekdayOverride,
    /// the colour to draw a subject in, as `#rrggbb` by the subject's name
    #[serde(default)]
    pub subject_color_overrides: BTreeMap<String, String>,
    #[serde(default)]
    pub lesson_text: LessonTextSettings,
    #[serde(default)]
    pub lesson_display: LessonDisplaySettings,
}

impl VisualSettings {
    /// The time range a day is drawn over: the lessons' own, widened to the configured padding.
    /// Padding only ever widens - a lesson outside it is still shown.
    pub fn padded_range(&self, lessons_start: NaiveTime, lessons_end: NaiveTime) -> (NaiveTime, NaiveTime) {
        match self.time_padding.enabled {
            true => (lessons_start.min(self.time_padding.from), lessons_end.max(self.time_padding.until)),
            false => (lessons_start, lessons_end),
        }
    }

    /// The colour set for a subject, whichever case its name was entered in
    pub fn subject_color(&self, subject_name: &str) -> Option<&String> {
        let wanted = subject_name.trim();
        if wanted.is_empty() {
            return None;
        }
        self.subject_color_overrides.iter().find(|(name, _)| name.trim().eq_ignore_ascii_case(wanted)).map(|(_, color)| color)
    }

    /// Sets the colour of a subject, replacing the one set for it in whichever case
    pub fn set_subject_color(&mut self, subject_name: &str, color: String) {
        self.subject_color_overrides.retain(|name, _| !name.eq_ignore_ascii_case(subject_name));
        self.subject_color_overrides.insert(subject_name.to_string(), color);
    }

    /// The colour set for the lesson's subject, by any of the names Untis has for it. A subject
    /// the lesson still has goes before one a change removed.
    pub fn lesson_color(&self, lesson: &LessonBlock) -> Option<&String> {
        let mut subjects: Vec<_> = lesson
            .entities
            .iter()
            .filter_map(|entity| match &entity.inner {
                Entity::Subject(subject) => Some((&entity.status, subject)),
                _ => None,
            })
            .collect();
        subjects.sort_by_key(|(status, _)| **status == ChangeStatus::Removed);

        subjects.into_iter().find_map(|(_, subject)| {
            [&subject.short_name, &subject.display_name, &subject.long_name].into_iter().find_map(|name| self.subject_color(name))
        })
    }

    /// Draws the lesson in the colour set for its subject, if one is
    pub fn apply_color_override(&self, lesson: &mut LessonBlock) {
        if let Some(color) = self.lesson_color(lesson) {
            lesson.color_hex = color.trim_start_matches('#').to_string();
        }
    }
}

/// Whether a colour (`#rrggbb` or `rrggbb`) is dark enough to need light text on it
pub fn is_dark_color(hex: &str) -> bool {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return false;
    }

    let channel = |at: usize| f64::from(u8::from_str_radix(&hex[at..at + 2], 16).unwrap_or(255));
    let luminance = 0.299 * channel(0) + 0.587 * channel(2) + 0.114 * channel(4);
    luminance < DARK_LUMINANCE
}

/// Which days of the week the timetable shows
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct WeekdayOverride {
    /// show a day only if it has lessons, whatever `always_show` says
    #[serde(default)]
    pub only_lessons: bool,
    #[serde(default = "default_always_show")]
    pub always_show: Vec<Weekday>,
}

fn default_always_show() -> Vec<Weekday> {
    WORK_WEEKDAYS.to_vec()
}

impl Default for WeekdayOverride {
    fn default() -> Self {
        Self { only_lessons: false, always_show: default_always_show() }
    }
}

impl WeekdayOverride {
    pub fn should_show(&self, weekday: Weekday, has_lessons: bool) -> bool {
        has_lessons || (!self.only_lessons && self.is_always_shown(weekday))
    }

    pub fn is_always_shown(&self, weekday: Weekday) -> bool {
        self.always_show.contains(&weekday)
    }

    pub fn toggle_weekday(&mut self, weekday: Weekday) {
        match self.always_show.iter().position(|shown| *shown == weekday) {
            Some(position) => {
                self.always_show.remove(position);
            }
            None => {
                self.always_show.push(weekday);
                self.always_show.sort_by_key(Weekday::num_days_from_monday);
            }
        }
    }
}

/// Draws the timetable over at least this range, so a short day doesn't stretch its lessons over
/// the whole screen and the grid keeps the same scale from week to week. The times are kept while
/// it is switched off, so turning it back on doesn't mean entering them again.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TimePadding {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_pad_from")]
    pub from: NaiveTime,
    #[serde(default = "default_pad_until")]
    pub until: NaiveTime,
}

/// the first period's start and the tenth's end, the school day most weeks fit into
fn default_pad_from() -> NaiveTime {
    NaiveTime::from_hms_opt(7, 50, 0).unwrap_or_default()
}

fn default_pad_until() -> NaiveTime {
    NaiveTime::from_hms_opt(16, 35, 0).unwrap_or_default()
}

impl Default for TimePadding {
    fn default() -> Self {
        Self { enabled: false, from: default_pad_from(), until: default_pad_until() }
    }
}

/// A line of what is written on a lesson in the week grid
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum LessonLine {
    Subject,
    Teacher,
    Room,
    Class,
    /// when the lesson starts and ends
    Time,
}

impl LessonLine {
    /// The lines in the order they are written in
    pub const ALL: [LessonLine; 5] = [LessonLine::Subject, LessonLine::Teacher, LessonLine::Room, LessonLine::Class, LessonLine::Time];

    pub fn label(self) -> &'static str {
        match self {
            LessonLine::Subject => "Subject",
            LessonLine::Teacher => "Teacher",
            LessonLine::Room => "Room",
            LessonLine::Class => "Class",
            LessonLine::Time => "Time",
        }
    }
}

/// The typeface the lessons are written in
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum LessonFont {
    /// the one the rest of the app uses
    #[default]
    Default,
    Serif,
    Monospace,
}

impl LessonFont {
    pub const ALL: [LessonFont; 3] = [LessonFont::Default, LessonFont::Serif, LessonFont::Monospace];

    pub fn label(self) -> &'static str {
        match self {
            LessonFont::Default => "Default",
            LessonFont::Serif => "Serif",
            LessonFont::Monospace => "Monospace",
        }
    }

    /// What CSS calls the typeface, `None` for the app's own
    pub fn css_family(self) -> Option<&'static str> {
        match self {
            LessonFont::Default => None,
            LessonFont::Serif => Some("serif"),
            LessonFont::Monospace => Some("monospace"),
        }
    }
}

/// The text is never scaled beyond these, in percent, whatever was stored: smaller is
/// unreadable, larger doesn't fit a lesson
const FONT_SCALE_RANGE: (u32, u32) = (50, 200);
const DEFAULT_FONT_SCALE_PERCENT: u32 = 100;

fn default_lesson_lines() -> Vec<LessonLine> {
    vec![LessonLine::Subject, LessonLine::Teacher, LessonLine::Room]
}

fn default_font_scale_percent() -> u32 {
    DEFAULT_FONT_SCALE_PERCENT
}

/// What is written on a lesson in the week grid, and how
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct LessonTextSettings {
    /// the lines written on a lesson, top to bottom
    #[serde(default = "default_lesson_lines")]
    pub lines: Vec<LessonLine>,
    /// subjects and teachers by the long name Untis has for them rather than the abbreviation
    #[serde(default)]
    pub long_names: bool,
    #[serde(default)]
    pub font: LessonFont,
    /// the size of the text, in percent of what it is by default
    #[serde(default = "default_font_scale_percent")]
    pub font_scale_percent: u32,
    #[serde(default)]
    pub bold: bool,
}

impl Default for LessonTextSettings {
    fn default() -> Self {
        Self { lines: default_lesson_lines(), long_names: false, font: LessonFont::default(), font_scale_percent: DEFAULT_FONT_SCALE_PERCENT, bold: false }
    }
}

impl LessonTextSettings {
    pub fn has_line(&self, line: LessonLine) -> bool {
        self.lines.contains(&line)
    }

    /// Writes the line on the lessons or takes it off them. The lines keep their order.
    pub fn toggle_line(&mut self, line: LessonLine) {
        self.lines = LessonLine::ALL.into_iter().filter(|other| self.has_line(*other) != (*other == line)).collect();
    }

    /// How much larger or smaller than usual the text is written, 1 being as usual
    pub fn font_scale(&self) -> f64 {
        let (smallest, largest) = FONT_SCALE_RANGE;
        f64::from(self.font_scale_percent.clamp(smallest, largest)) / 100.0
    }

    /// Whether the text is written the way it always was, in which case it needs no styling
    pub fn is_default_style(&self) -> bool {
        self.font == LessonFont::Default && self.font_scale() == 1.0 && !self.bold
    }
}

/// How the lessons of a timetable are drawn
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct LessonDisplaySettings {
    /// greys out the lessons that are over, and the part of the running one that is
    #[serde(default = "default_true")]
    pub dim_past_lessons: bool,
    /// draws a line at the current time
    #[serde(default = "default_true")]
    pub show_now_line: bool,
    /// leaves the cancelled lessons out rather than striking them through
    #[serde(default)]
    pub hide_cancelled: bool,
}

impl Default for LessonDisplaySettings {
    fn default() -> Self {
        Self { dim_past_lessons: true, show_now_line: true, hide_cancelled: false }
    }
}

impl LessonDisplaySettings {
    /// Whether a lesson is drawn at all
    pub fn shows(&self, lesson: &LessonBlock) -> bool {
        !(self.hide_cancelled && lesson.is_cancelled())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::timetable::{Subject, Tracked};

    fn at(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).unwrap()
    }

    #[test]
    fn padding_widens_the_day_but_never_narrows_it() {
        let settings = VisualSettings {
            time_padding: TimePadding { enabled: true, from: at(7, 50), until: at(17, 25) },
            ..Default::default()
        };

        // a short day is stretched to the configured range
        assert_eq!(settings.padded_range(at(9, 45), at(11, 25)), (at(7, 50), at(17, 25)));
        // lessons outside it still widen the day further
        assert_eq!(settings.padded_range(at(7, 0), at(18, 15)), (at(7, 0), at(18, 15)));
    }

    #[test]
    fn without_padding_the_lessons_decide() {
        let settings = VisualSettings::default();

        assert_eq!(settings.padded_range(at(9, 45), at(11, 25)), (at(9, 45), at(11, 25)));
    }

    /// switched off it keeps its times, so they are there again when it is switched back on
    #[test]
    fn a_disabled_padding_does_nothing() {
        let settings = VisualSettings {
            time_padding: TimePadding { enabled: false, from: at(7, 50), until: at(17, 25) },
            ..Default::default()
        };

        assert_eq!(settings.padded_range(at(9, 45), at(11, 25)), (at(9, 45), at(11, 25)));
    }

    fn subject(short_name: &str, long_name: &str, status: ChangeStatus) -> Tracked<Entity> {
        let subject = Subject { short_name: short_name.to_string(), long_name: long_name.to_string(), ..Default::default() };
        Tracked { inner: Entity::Subject(subject), status }
    }

    fn colors(overrides: &[(&str, &str)]) -> VisualSettings {
        VisualSettings {
            subject_color_overrides: overrides.iter().map(|(name, color)| (name.to_string(), color.to_string())).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn finds_a_subject_colour_by_any_of_its_names_in_any_case() {
        let settings = colors(&[("m", "#112233"), ("Deutsch", "#445566")]);
        let lesson = |entity| LessonBlock { entities: vec![entity], ..Default::default() };

        assert_eq!(settings.lesson_color(&lesson(subject("M", "", ChangeStatus::Regular))).map(String::as_str), Some("#112233"));
        assert_eq!(settings.lesson_color(&lesson(subject("D", "deutsch", ChangeStatus::Regular))).map(String::as_str), Some("#445566"));
        assert_eq!(settings.lesson_color(&lesson(subject("E", "", ChangeStatus::Regular))), None);
    }

    #[test]
    fn the_subject_a_lesson_still_has_decides_its_colour() {
        let settings = colors(&[("M", "#112233"), ("D", "#445566")]);
        let mut lesson = LessonBlock {
            entities: vec![subject("M", "", ChangeStatus::Removed), subject("D", "", ChangeStatus::Changed)],
            ..Default::default()
        };

        settings.apply_color_override(&mut lesson);
        assert_eq!(lesson.color_hex, "445566");
    }

    #[test]
    fn a_subject_has_one_colour_however_its_name_is_written() {
        let mut settings = colors(&[("m", "#112233")]);

        settings.set_subject_color("M", "#445566".to_string());
        assert_eq!(settings.subject_color_overrides.len(), 1);
        assert_eq!(settings.subject_color("m").map(String::as_str), Some("#445566"));
    }

    #[test]
    fn tells_dark_colours_from_light_ones() {
        assert!(is_dark_color("#112233"));
        assert!(is_dark_color("000000"));
        assert!(!is_dark_color("#ffeeaa"));
        // nothing to go by, so the text stays dark
        assert!(!is_dark_color(""));
    }

    #[test]
    fn toggling_a_weekday_keeps_the_week_in_order() {
        let mut weekdays = WeekdayOverride::default();

        weekdays.toggle_weekday(Weekday::Sun);
        weekdays.toggle_weekday(Weekday::Wed);
        weekdays.toggle_weekday(Weekday::Sat);
        assert_eq!(weekdays.always_show, vec![Weekday::Mon, Weekday::Tue, Weekday::Thu, Weekday::Fri, Weekday::Sat, Weekday::Sun]);
    }

    #[test]
    fn a_day_with_lessons_is_always_shown() {
        let only_lessons = WeekdayOverride { only_lessons: true, always_show: vec![Weekday::Mon] };

        assert!(only_lessons.should_show(Weekday::Sat, true));
        assert!(!only_lessons.should_show(Weekday::Mon, false));
        assert!(WeekdayOverride::default().should_show(Weekday::Mon, false));
        assert!(!WeekdayOverride::default().should_show(Weekday::Sat, false));
    }

    #[test]
    fn settings_from_before_the_lesson_settings_draw_lessons_as_they_always_were() {
        let settings: VisualSettings = serde_json::from_str(r#"{"force_ascii_timetable": true}"#).unwrap();

        assert_eq!(settings.lesson_text.lines, vec![LessonLine::Subject, LessonLine::Teacher, LessonLine::Room]);
        assert!(settings.lesson_text.is_default_style() && !settings.lesson_text.long_names);
        assert_eq!(settings.lesson_display, LessonDisplaySettings { dim_past_lessons: true, show_now_line: true, hide_cancelled: false });
    }

    #[test]
    fn the_lines_of_a_lesson_keep_their_order_however_they_are_switched() {
        let mut text = LessonTextSettings::default();

        text.toggle_line(LessonLine::Time);
        text.toggle_line(LessonLine::Teacher);
        text.toggle_line(LessonLine::Class);
        assert_eq!(text.lines, vec![LessonLine::Subject, LessonLine::Room, LessonLine::Class, LessonLine::Time]);

        text.toggle_line(LessonLine::Teacher);
        assert_eq!(text.lines, vec![LessonLine::Subject, LessonLine::Teacher, LessonLine::Room, LessonLine::Class, LessonLine::Time]);
    }

    #[test]
    fn the_text_size_stays_within_reason() {
        let scale = |percent| LessonTextSettings { font_scale_percent: percent, ..Default::default() }.font_scale();

        assert_eq!(scale(125), 1.25);
        assert_eq!(scale(0), 0.5);
        assert_eq!(scale(10_000), 2.0);
    }

    #[test]
    fn any_change_to_the_font_is_a_style_to_apply() {
        assert!(LessonTextSettings::default().is_default_style());
        assert!(!LessonTextSettings { bold: true, ..Default::default() }.is_default_style());
        assert!(!LessonTextSettings { font: LessonFont::Serif, ..Default::default() }.is_default_style());
        assert!(!LessonTextSettings { font_scale_percent: 110, ..Default::default() }.is_default_style());
    }

    #[test]
    fn cancelled_lessons_are_only_left_out_when_asked() {
        let cancelled = LessonBlock { status: crate::models::timetable::LessonStatus::Cancelled, ..Default::default() };

        assert!(LessonDisplaySettings::default().shows(&cancelled));
        let hiding = LessonDisplaySettings { hide_cancelled: true, ..Default::default() };
        assert!(!hiding.shows(&cancelled));
        assert!(hiding.shows(&LessonBlock::default()));
    }
}
