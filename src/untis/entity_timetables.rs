use altis_core::models::timetable::{ChangeStatus, Class, DayTimeTable, Entity, LessonBlock, WeekTimeTable};
use chrono::NaiveDate;
use std::collections::{BTreeSet, HashMap};

/// The timetable of everything that has one in a week: the classes', and the teachers' and rooms'
/// put together from them.
///
/// Students may not open a teacher's or a room's timetable in Untis, but they may open every
/// class's - and every lesson in those names its teachers and rooms.
pub fn entity_timetables(class_timetables: HashMap<Class, WeekTimeTable>) -> HashMap<Entity, WeekTimeTable> {
    let mut timetables: HashMap<Entity, WeekTimeTable> =
        class_timetables.into_iter().map(|(class, timetable)| (Entity::Class(class), timetable)).collect();

    // class by class in the order of their names, so the lessons a teacher or a room has at the
    // same time come in the same order every time, rather than in whatever order the map hands
    // the classes out - which decided which of them was drawn on the left, or on top
    let mut classes: Vec<(&Entity, &WeekTimeTable)> = timetables.iter().collect();
    classes.sort_by_key(|(class, _)| class.name());

    let mut lessons_of: HashMap<Entity, HashMap<NaiveDate, Vec<LessonBlock>>> = HashMap::new();
    for day in classes.into_iter().flat_map(|(_, timetable)| &timetable.days) {
        for lesson in &day.lessons {
            let teachers_and_rooms = lesson.entities.iter().filter(|entity| {
                entity.status != ChangeStatus::Removed && (entity.inner.is_teacher() || entity.inner.is_room())
            });
            for entity in teachers_and_rooms {
                lessons_of.entry(entity.inner.clone()).or_default().entry(day.date).or_default().push(lesson.clone());
            }
        }
    }

    // every day of the week gets an entry, even without lessons, so the weekday settings can still show it
    let dates: BTreeSet<NaiveDate> = timetables.values().flat_map(|timetable| timetable.days.iter().map(|day| day.date)).collect();

    for (entity, mut lessons) in lessons_of {
        let days = dates.iter().map(|&date| DayTimeTable { date, lessons: lessons.remove(&date).unwrap_or_default() }).collect();
        timetables.insert(entity, WeekTimeTable { days });
    }

    timetables
}

#[cfg(test)]
mod tests {
    use super::*;
    use altis_core::models::timetable::{Room, Teacher, TimeRange, Tracked};

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, day).unwrap()
    }

    fn teacher(name: &str) -> Entity {
        Entity::Teacher(Teacher { short_name: name.to_string(), ..Default::default() })
    }

    fn room(name: &str) -> Entity {
        Entity::Room(Room { name: name.to_string() })
    }

    fn lesson(day: u32, entities: Vec<(Entity, ChangeStatus)>) -> LessonBlock {
        LessonBlock {
            time_range: TimeRange { start: date(day).and_hms_opt(8, 0, 0).unwrap(), end: date(day).and_hms_opt(8, 50, 0).unwrap() },
            entities: entities.into_iter().map(|(inner, status)| Tracked { inner, status }).collect(),
            ..Default::default()
        }
    }

    fn class(name: &str, lessons: Vec<LessonBlock>) -> (Class, WeekTimeTable) {
        let days = [14, 15]
            .into_iter()
            .map(|day| DayTimeTable { date: date(day), lessons: lessons.iter().filter(|l| l.time_range.start.date() == date(day)).cloned().collect() })
            .collect();
        (Class { name: name.to_string(), ..Default::default() }, WeekTimeTable { days })
    }

    #[test]
    fn a_teacher_gets_every_lesson_they_appear_in() {
        let regular = ChangeStatus::Regular;
        let timetables = entity_timetables(HashMap::from([
            class("4A", vec![lesson(14, vec![(teacher("SMI"), regular.clone()), (room("R1"), regular.clone())])]),
            class("4B", vec![lesson(15, vec![(teacher("SMI"), regular.clone()), (room("R2"), regular)])]),
        ]));

        // two classes, one teacher, two rooms
        assert_eq!(timetables.len(), 5);
        let smith = &timetables[&teacher("SMI")];
        assert_eq!(smith.days.iter().map(|day| day.lessons.len()).collect::<Vec<_>>(), vec![1, 1]);
        // a day without lessons is still a day of the week
        let r1 = &timetables[&room("R1")];
        assert_eq!(r1.days.iter().map(|day| day.lessons.len()).collect::<Vec<_>>(), vec![1, 0]);
    }

    #[test]
    fn a_replaced_teacher_does_not_get_the_lesson() {
        let substituted = lesson(14, vec![(teacher("JON"), ChangeStatus::Changed), (teacher("SMI"), ChangeStatus::Removed)]);
        let timetables = entity_timetables(HashMap::from([class("4A", vec![substituted])]));

        assert!(timetables.contains_key(&teacher("JON")));
        assert!(!timetables.contains_key(&teacher("SMI")));
    }

    /// a teacher with two classes at once: the two lessons overlap, and their order is what
    /// decides how they are drawn
    #[test]
    fn simultaneous_lessons_come_in_the_order_of_their_classes() {
        let with_class = |name: &str| {
            let class = Class { name: name.to_string(), ..Default::default() };
            lesson(14, vec![(teacher("SMI"), ChangeStatus::Regular), (Entity::Class(class), ChangeStatus::Regular)])
        };

        // a map hands its entries out in an order of its own, which differs from map to map
        for _ in 0..20 {
            let timetables = entity_timetables(HashMap::from([
                class("4C", vec![with_class("4C")]),
                class("4A", vec![with_class("4A")]),
                class("4B", vec![with_class("4B")]),
            ]));

            let classes: Vec<String> = timetables[&teacher("SMI")].days[0].lessons.iter().flat_map(|lesson| lesson.current_names(Entity::is_class)).collect();
            assert_eq!(classes, vec!["4A", "4B", "4C"]);
        }
    }
}
