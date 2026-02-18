use crate::data_models::clean_models::untis::*;
use crate::data_models::response_models::untis_response_models::*;
use crate::data_models::response_models::untis_timetables::*;
use chrono::{NaiveDate, NaiveDateTime};
use std::collections::HashMap;

impl From<UntisClassEntry> for Class {
    fn from(entry: UntisClassEntry) -> Self {
        Self {
            id: entry.class_info.id,
            name: entry.class_info.short_name,
            class_teacher: entry.class_teacher1.map(Teacher::from),
            department: Department::from(entry.department),
        }
    }
}

impl From<UntisDepartment> for Department {
    fn from(d: UntisDepartment) -> Self {
        Self {
            id: d.id,
            short_name: d.short_name,
            long_name: d.long_name,
            display_name: d.display_name,
        }
    }
}

impl From<UntisTeacher> for Teacher {
    fn from(t: UntisTeacher) -> Self {
        Self {
            id: Some(t.id),
            short_name: t.short_name,
            long_name: t.long_name,
            display_name: t.display_name,
        }
    }
}

impl From<UntisDuration> for TimeRange {
    fn from(duration: UntisDuration) -> Self {
        let format = "%Y-%m-%dT%H:%M";
        let start_naive = NaiveDateTime::parse_from_str(&duration.start, format)
            .expect("Failed to parse start date");
        let end_naive =
            NaiveDateTime::parse_from_str(&duration.end, format).expect("Failed to parse end date");

        TimeRange {
            start: start_naive,
            end: end_naive,
        }
    }
}
impl From<UntisResource> for Entity {
    fn from(res: UntisResource) -> Self {
        match res.r#type.to_lowercase().as_str() {
            "teacher" => Entity::Teacher(Teacher {
                id: None, // Untis doesn't provide teacher IDs in grid entries
                short_name: res.short_name,
                long_name: res.long_name,
                display_name: res.display_name,
            }),
            "subject" => Entity::Subject(Subject {
                short_name: res.short_name,
                long_name: res.long_name,
                display_name: res.display_name,
            }),
            "room" => Entity::Room(Room {
                name: res.short_name,
            }),
            "klasse" | "class" => Entity::Class(Class {
                id: 0,
                name: res.short_name,
                class_teacher: None,
                department: Department::default(),
            }),
            "lesson_info" | "info" => Entity::Info(Info {
                text: res.text.unwrap_or("".to_string()),
            }),
            _ => panic!("Unrecognized resource type: {}", res.r#type),
        }
    }
}

impl From<UntisPosition> for Vec<Tracked<Entity>> {
    fn from(pos: UntisPosition) -> Self {
        let mut tracked_entities = Vec::new();

        if let Some(current) = pos.current {
            tracked_entities.push(Tracked {
                status: if pos.removed.is_some() {
                    ChangeStatus::Changed
                } else {
                    ChangeStatus::Regular
                },
                inner: Entity::from(current),
            });
        }

        if let Some(removed_res) = pos.removed {
            tracked_entities.push(Tracked {
                inner: Entity::from(removed_res),
                status: ChangeStatus::Removed,
            });
        }

        tracked_entities
    }
}

impl From<UntisGridEntry> for LessonBlock {
    fn from(entry: UntisGridEntry) -> Self {
        let mut entities = Vec::new();

        let positions = entry
            .position1
            .into_iter()
            .chain(entry.position2)
            .chain(entry.position3)
            .chain(entry.position4)
            .chain(entry.position5)
            .chain(entry.position6)
            .chain(entry.position7)
            .flatten();

        for pos in positions {
            entities.extend(Vec::<Tracked<Entity>>::from(pos));
        }

        let mut texts_map = HashMap::new();
        texts_map.insert("notesAll".to_string(), entry.notes_all);
        texts_map.insert("lessonText".to_string(), entry.lesson_text);
        texts_map.insert("substitutionText".to_string(), entry.substitution_text);

        if let Some(sd) = entry.status_detail {
            texts_map.insert("statusDetail".to_string(), sd);
        }
        if let Some(n) = entry.name {
            texts_map.insert("name".to_string(), n);
        }
        if let Some(un) = entry.user_name {
            texts_map.insert("userName".to_string(), un);
        }

        Self {
            time_range: TimeRange::from(entry.duration),
            entities,
            r#type: entry.r#type,
            status: entry.status,
            color_hex: entry.color,
            icons: entry.icons,
            texts: vec![texts_map],
            link: entry.link.unwrap_or_default(),
        }
    }
}
impl From<UntisDayEntry> for DayTimeTable {
    fn from(entry: UntisDayEntry) -> Self {
        Self {
            date: NaiveDate::parse_from_str(entry.date.as_str(), "%Y-%m-%d").unwrap_or_default(),
            lessons: entry
                .grid_entries
                .into_iter()
                .map(LessonBlock::from)
                .collect(),
        }
    }
}
