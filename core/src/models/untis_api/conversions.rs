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

impl TryFrom<UntisDuration> for TimeRange {
    type Error = chrono::ParseError;

    fn try_from(duration: UntisDuration) -> Result<Self, Self::Error> {
        Ok(Self {
            start: NaiveDateTime::parse_from_str(&duration.start, DATE_TIME_FORMAT)?,
            end: NaiveDateTime::parse_from_str(&duration.end, DATE_TIME_FORMAT)?,
        })
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
            // a sort of resource the app has never seen is shown as a note, by whatever it is called
            _ => {
                let names = [resource.text.unwrap_or_default(), resource.display_name, resource.long_name, resource.short_name];
                Entity::Info(Info { text: names.into_iter().find(|name| !name.is_empty()).unwrap_or_default() })
            }
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

impl TryFrom<UntisGridEntry> for LessonBlock {
    type Error = chrono::ParseError;

    fn try_from(entry: UntisGridEntry) -> Result<Self, Self::Error> {
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

        Ok(Self {
            time_range: TimeRange::try_from(entry.duration)?,
            entities: positions.into_iter().flatten().flatten().flat_map(tracked_entities).collect(),
            kind: LessonKind::from(entry.r#type),
            status: LessonStatus::from(entry.status),
            color_hex: entry.color,
            icons: entry.icons,
            texts: vec![texts],
            link: entry.link.unwrap_or_default(),
        })
    }
}

impl From<UntisDayEntry> for DayTimeTable {
    fn from(entry: UntisDayEntry) -> Self {
        Self {
            date: NaiveDate::parse_from_str(&entry.date, DATE_FORMAT).unwrap_or_default(),
            // an entry without a time it takes place at has no place in a timetable
            lessons: entry.grid_entries.into_iter().filter_map(|entry| LessonBlock::try_from(entry).ok()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_entry(json: &str) -> LessonBlock {
        serde_json::from_str::<UntisGridEntry>(json).unwrap().try_into().unwrap()
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

    /// Untis adds sorts of resources now and then, and one the app doesn't know used to take
    /// the whole app down with it
    #[test]
    fn a_resource_of_an_unknown_sort_is_shown_as_a_note() {
        let lesson = grid_entry(r#"{
            "duration": {"start": "2026-09-14T07:50", "end": "2026-09-14T08:40"},
            "position1": [{"current": {"type": "STUDENT_GROUP", "shortName": "4A_G1", "longName": "Group 1"}}],
            "position2": [{"current": {"type": "SOMETHING", "text": "see notice board", "shortName": "x"}}]
        }"#);

        let notes: Vec<&Entity> = lesson.entities.iter().map(|entity| &entity.inner).collect();
        assert_eq!(notes, vec![
            &Entity::Info(Info { text: "Group 1".to_string() }),
            &Entity::Info(Info { text: "see notice board".to_string() }),
        ]);
    }

    #[test]
    fn an_entry_without_a_readable_time_is_left_out_of_the_day() {
        let day: UntisDayEntry = serde_json::from_str(r#"{"date": "2026-09-14", "gridEntries": [
            {"duration": {"start": "2026-09-14T07:50", "end": "2026-09-14T08:40"}},
            {"duration": {"start": "whenever", "end": "2026-09-14T09:30"}}
        ]}"#).unwrap();

        assert_eq!(DayTimeTable::from(day).lessons.len(), 1);
    }
}
