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

    let mut lessons_of: HashMap<Entity, HashMap<NaiveDate, Vec<LessonBlock>>> = HashMap::new();
    for day in timetables.values().flat_map(|timetable| &timetable.days) {
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
}
