use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default, Clone, PartialEq, Debug, Serialize, Hash, Eq, Deserialize)]
pub struct Class {
    pub id: i32,
    pub name: String,
    pub class_teacher: Option<Teacher>,
    pub department: Department,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Hash, Eq, Deserialize)]
pub struct Department {
    pub id: i32,
    pub short_name: String,
    pub long_name: String,
    pub display_name: String,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Hash, Eq, Deserialize)]
pub struct Teacher {
    /// only known for class teachers, the timetable entries don't carry it
    pub id: Option<i32>,
    pub short_name: String,
    pub long_name: String,
    pub display_name: String,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Hash, Eq, Deserialize)]
pub struct Room {
    pub name: String,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Hash, Eq, Deserialize)]
pub struct Subject {
    pub short_name: String,
    pub long_name: String,
    pub display_name: String,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Hash, Eq, Deserialize)]
pub struct Info {
    pub text: String,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct WeekTimeTable {
    pub days: Vec<DayTimeTable>,
}

/// The personal timetable of the logged in user (only their own groups/lessons)
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MyTimeTable {
    pub name: String,
    pub timetable: WeekTimeTable,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct DayTimeTable {
    pub date: NaiveDate,
    pub lessons: Vec<LessonBlock>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct LessonBlock {
    pub time_range: TimeRange,
    pub entities: Vec<Tracked<Entity>>,
    #[serde(rename = "type")]
    pub kind: LessonKind,
    pub status: LessonStatus,
    pub color_hex: String,
    pub icons: Vec<String>,
    /// may include "notesAll", "statusDetail", "name", "lessonText", "substitutionText" and
    /// "userName"
    pub texts: Vec<BTreeMap<String, String>>,
    pub link: String,
}

impl LessonBlock {
    pub fn overlaps(&self, other: &LessonBlock) -> bool {
        self.time_range.start < other.time_range.end && other.time_range.start < self.time_range.end
    }

    pub fn is_cancelled(&self) -> bool {
        self.status == LessonStatus::Cancelled
    }

    pub fn is_exam(&self) -> bool {
        self.kind == LessonKind::Exam
    }

    pub fn is_break(&self) -> bool {
        self.kind == LessonKind::Break
    }

    /// The names of the lesson's entities of one sort, leaving out the ones a change removed
    pub fn current_names(&self, wanted: fn(&Entity) -> bool) -> impl Iterator<Item = String> + '_ {
        self.entities
            .iter()
            .filter(move |tracked| tracked.status != ChangeStatus::Removed && wanted(&tracked.inner))
            .map(|tracked| tracked.inner.name())
    }
}

/// Declares an enum over the values of an Untis field the app tells apart, keeping every other
/// value as Untis sent it. It is stored as that very string, so what was cached or remembered
/// before these were enums still reads.
macro_rules! untis_string_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$variant_meta:meta])* $variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
        #[serde(from = "String", into = "String")]
        pub enum $name {
            $($(#[$variant_meta])* $variant,)+
            Other(String),
        }

        impl $name {
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $text,)+
                    Self::Other(text) => text,
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::Other(String::new())
            }
        }

        impl From<String> for $name {
            fn from(text: String) -> Self {
                match text.as_str() {
                    $($text => Self::$variant,)+
                    _ => Self::Other(text),
                }
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.as_str().to_string()
            }
        }
    };
}

untis_string_enum! {
    /// What became of a lesson
    LessonStatus {
        Cancelled = "CANCELLED",
        Changed = "CHANGED",
        /// one that wasn't planned, a replacement for a cancelled lesson for instance
        Additional = "ADDITIONAL",
    }
}

untis_string_enum! {
    /// What sort of lesson it is
    LessonKind {
        Exam = "EXAM",
        /// not from Untis: the gap between two lessons, which the week grid fills in itself
        Break = "Break",
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Hash, Eq, Deserialize)]
pub enum Entity {
    Teacher(Teacher),
    Class(Class),
    Room(Room),
    Subject(Subject),
    Info(Info),
}

impl Entity {
    pub fn name(&self) -> String {
        match self {
            Entity::Class(c) => c.name.clone(),
            Entity::Teacher(t) => t.short_name.clone(),
            Entity::Room(r) => r.name.clone(),
            Entity::Subject(s) => s.short_name.clone(),
            Entity::Info(i) => i.text.clone(),
        }
    }

    /// Which sort of timetable the entity has, `None` for one that has none of its own
    pub fn kind(&self) -> Option<EntityKind> {
        match self {
            Entity::Class(_) => Some(EntityKind::Class),
            Entity::Teacher(_) => Some(EntityKind::Teacher),
            Entity::Room(_) => Some(EntityKind::Room),
            Entity::Subject(_) | Entity::Info(_) => None,
        }
    }

    pub fn is_subject(&self) -> bool {
        matches!(self, Entity::Subject(_))
    }

    pub fn is_teacher(&self) -> bool {
        matches!(self, Entity::Teacher(_))
    }

    pub fn is_room(&self) -> bool {
        matches!(self, Entity::Room(_))
    }

    pub fn is_class(&self) -> bool {
        matches!(self, Entity::Class(_))
    }
}

/// The entities that have a timetable of their own
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub enum EntityKind {
    Class,
    Teacher,
    Room,
}

impl EntityKind {
    pub const ALL: [EntityKind; 3] = [EntityKind::Class, EntityKind::Teacher, EntityKind::Room];

    pub fn as_str(self) -> &'static str {
        match self {
            EntityKind::Class => "Class",
            EntityKind::Teacher => "Teacher",
            EntityKind::Room => "Room",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == name)
    }
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: NaiveDateTime,
    pub end: NaiveDateTime,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Tracked<T> {
    pub inner: T,
    pub status: ChangeStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChangeStatus {
    Regular,
    Changed,
    Removed,
    New,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_status_is_stored_as_the_text_untis_sent() {
        for text in ["CANCELLED", "CHANGED", "ADDITIONAL", "REGULAR", ""] {
            let status = LessonStatus::from(text.to_string());
            assert_eq!(serde_json::to_string(&status).unwrap(), format!("\"{text}\""));
            assert_eq!(serde_json::from_str::<LessonStatus>(&format!("\"{text}\"")).unwrap(), status);
        }

        assert_eq!(LessonStatus::from("CANCELLED".to_string()), LessonStatus::Cancelled);
        assert_eq!(LessonStatus::from("REGULAR".to_string()), LessonStatus::Other("REGULAR".to_string()));
        assert_eq!(LessonKind::from("EXAM".to_string()), LessonKind::Exam);
    }

    /// what a poll remembered before the status and the type were enums has to stay readable,
    /// or the notification bookkeeping of every install would start over
    #[test]
    fn reads_a_lesson_stored_with_plain_strings() {
        let stored = r##"{
            "time_range": {"start": "2026-09-14T08:00:00", "end": "2026-09-14T08:50:00"},
            "entities": [{"inner": {"Room": {"name": "R1"}}, "status": "Regular"}],
            "type": "EXAM", "status": "CANCELLED", "color_hex": "#fff", "icons": [],
            "texts": [{"lessonText": "x"}], "link": ""
        }"##;

        let lesson: LessonBlock = serde_json::from_str(stored).unwrap();
        assert!(lesson.is_exam() && lesson.is_cancelled());

        let written: serde_json::Value = serde_json::to_value(&lesson).unwrap();
        assert_eq!(written["type"], "EXAM");
        assert_eq!(written["status"], "CANCELLED");
    }
}
