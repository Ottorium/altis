//! From what Untis sends to the app's own timetable types.

use super::timetable::*;
use super::*;
use crate::models::timetable::*;
use chrono::{NaiveDate, NaiveDateTime};
use std::collections::BTreeMap;

const DATE_TIME_FORMAT: &str = "%Y-%m-%dT%H:%M";
const DATE_FORMAT: &str = "%Y-%m-%d";

impl From<UntisClassEntry> for Class {
    fn from(entry: UntisClassEntry) -> Self {
        Self {
            id: entry.class_info.id,
            name: entry.class_info.short_name,
            class_teacher: entry.class_teacher1.map(Teacher::from),
            department: entry.department.map(Department::from).unwrap_or_default(),
        }
    }
}

impl From<UntisDepartment> for Department {
    fn from(department: UntisDepartment) -> Self {
        Self {
            id: department.id,
            short_name: department.short_name,
            long_name: department.long_name,
            display_name: department.display_name,
        }
    }
}

impl From<UntisTeacher> for Teacher {
    fn from(teacher: UntisTeacher) -> Self {
        Self {
            id: Some(teacher.id),
            short_name: teacher.short_name,
            long_name: teacher.long_name,
            display_name: teacher.display_name,
        }
    }
}

impl From<UntisDuration> for TimeRange {
    fn from(duration: UntisDuration) -> Self {
        Self {
            start: NaiveDateTime::parse_from_str(&duration.start, DATE_TIME_FORMAT).expect("Failed to parse start date"),
            end: NaiveDateTime::parse_from_str(&duration.end, DATE_TIME_FORMAT).expect("Failed to parse end date"),
        }
    }
}

impl From<UntisResource> for Entity {
    fn from(resource: UntisResource) -> Self {
        match resource.r#type.to_lowercase().as_str() {
            "teacher" => Entity::Teacher(Teacher {
                id: None,
                short_name: resource.short_name,
                long_name: resource.long_name,
                display_name: resource.display_name,
            }),
            "subject" => Entity::Subject(Subject {
                short_name: resource.short_name,
                long_name: resource.long_name,
                display_name: resource.display_name,
            }),
            "room" => Entity::Room(Room { name: resource.short_name }),
            "klasse" | "class" => Entity::Class(Class { name: resource.short_name, ..Default::default() }),
            "lesson_info" | "info" => Entity::Info(Info { text: resource.text.unwrap_or_default() }),
            _ => panic!("Unrecognized resource type: {}", resource.r#type),
        }
    }
}

/// The resource a position holds, and the one it replaced if the lesson was changed
fn tracked_entities(position: UntisPosition) -> impl Iterator<Item = Tracked<Entity>> {
    let current_status = match position.removed {
        Some(_) => ChangeStatus::Changed,
        None => ChangeStatus::Regular,
    };

    let current = position.current.map(|resource| Tracked { inner: Entity::from(resource), status: current_status });
    let removed = position.removed.map(|resource| Tracked { inner: Entity::from(resource), status: ChangeStatus::Removed });
    current.into_iter().chain(removed)
}

impl From<UntisGridEntry> for LessonBlock {
    fn from(entry: UntisGridEntry) -> Self {
        let positions = [
            entry.position1,
            entry.position2,
            entry.position3,
            entry.position4,
            entry.position5,
            entry.position6,
            entry.position7,
        ];

        let mut texts = BTreeMap::from([
            ("notesAll".to_string(), entry.notes_all),
            ("lessonText".to_string(), entry.lesson_text),
            ("substitutionText".to_string(), entry.substitution_text),
        ]);
        let optional_texts = [("statusDetail", entry.status_detail), ("name", entry.name), ("userName", entry.user_name)];
        for (key, text) in optional_texts {
            if let Some(text) = text {
                texts.insert(key.to_string(), text);
            }
        }

        Self {
            time_range: TimeRange::from(entry.duration),
            entities: positions.into_iter().flatten().flatten().flat_map(tracked_entities).collect(),
            kind: LessonKind::from(entry.r#type),
            status: LessonStatus::from(entry.status),
            color_hex: entry.color,
            icons: entry.icons,
            texts: vec![texts],
            link: entry.link.unwrap_or_default(),
        }
    }
}

impl From<UntisDayEntry> for DayTimeTable {
    fn from(entry: UntisDayEntry) -> Self {
        Self {
            date: NaiveDate::parse_from_str(&entry.date, DATE_FORMAT).unwrap_or_default(),
            lessons: entry.grid_entries.into_iter().map(LessonBlock::from).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_entry(json: &str) -> LessonBlock {
        serde_json::from_str::<UntisGridEntry>(json).unwrap().into()
    }

    const SUBSTITUTED: &str = r##"{
        "duration": {"start": "2026-09-14T07:50", "end": "2026-09-14T08:40"},
        "type": "NORMAL_TEACHING_PERIOD", "status": "CHANGED", "color": "#f49f25",
        "position1": [{
            "current": {"type": "TEACHER", "shortName": "JON", "longName": "Jones", "displayName": "JON"},
            "removed": {"type": "TEACHER", "shortName": "SMI", "longName": "Smith", "displayName": "SMI"}
        }],
        "position2": [{"current": {"type": "SUBJECT", "shortName": "M", "longName": "Maths", "displayName": "M"}, "removed": null}],
        "position3": [{"current": {"type": "ROOM", "shortName": "R1", "longName": null, "displayName": null}}],
        "position4": null,
        "lessonText": "bring a calculator", "substitutionText": null, "statusDetail": null, "userName": "smi"
    }"##;

    #[test]
    fn reads_a_grid_entry_with_its_replaced_teacher() {
        let lesson = grid_entry(SUBSTITUTED);

        assert_eq!(lesson.time_range.start.to_string(), "2026-09-14 07:50:00");
        assert_eq!(lesson.status, LessonStatus::Changed);
        assert_eq!(lesson.kind.as_str(), "NORMAL_TEACHING_PERIOD");

        let entities: Vec<(String, &ChangeStatus)> = lesson.entities.iter().map(|e| (e.inner.name(), &e.status)).collect();
        assert_eq!(entities, vec![
            ("JON".to_string(), &ChangeStatus::Changed),
            ("SMI".to_string(), &ChangeStatus::Removed),
            ("M".to_string(), &ChangeStatus::Regular),
            ("R1".to_string(), &ChangeStatus::Regular),
        ]);
        assert_eq!(lesson.current_names(Entity::is_teacher).collect::<Vec<_>>(), vec!["JON".to_string()]);
    }

    #[test]
    fn keeps_the_texts_untis_sent() {
        let texts = &grid_entry(SUBSTITUTED).texts[0];

        assert_eq!(texts["lessonText"], "bring a calculator");
        assert_eq!(texts["substitutionText"], "");
        assert_eq!(texts["userName"], "smi");
        assert!(!texts.contains_key("statusDetail"));
    }
}
