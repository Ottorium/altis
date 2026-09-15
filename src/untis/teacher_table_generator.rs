use altis_core::data_models::clean_models::untis::{
    ChangeStatus, Class, DayTimeTable, Entity, LessonBlock, WeekTimeTable,
};
use altis_core::errors::ApiError;
use std::collections::{BTreeSet, HashMap};

pub fn gen_all_timetables(
    class_results: HashMap<Class, WeekTimeTable>,
    pre_selected: Option<i32>,
) -> Result<(HashMap<Entity, WeekTimeTable>, Option<i32>), ApiError> {
    let mut all_timetables: HashMap<Entity, WeekTimeTable> = class_results
        .into_iter()
        .map(|(class, table)| (Entity::Class(class), table))
        .collect();

    let mut entity_lesson_map: HashMap<Entity, HashMap<chrono::NaiveDate, Vec<LessonBlock>>> =
        HashMap::new();

    for table in all_timetables.values() {
        for day_table in table.days.iter() {
            for lesson in &day_table.lessons {
                for entity_wrapper in &lesson.entities {
                    if entity_wrapper.status == ChangeStatus::Removed {
                        continue;
                    }

                    match &entity_wrapper.inner {
                        Entity::Teacher(_) | Entity::Room(_) => {
                            entity_lesson_map
                                .entry(entity_wrapper.inner.clone())
                                .or_default()
                                .entry(day_table.date)
                                .or_default()
                                .push(lesson.clone());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // every day of the week gets an entry, even without lessons, so the weekday settings can still show it
    let all_dates: BTreeSet<chrono::NaiveDate> = all_timetables.values()
        .flat_map(|table| table.days.iter().map(|day| day.date))
        .collect();

    for (entity, mut lessons) in entity_lesson_map {
        let days = all_dates.iter()
            .map(|&date| DayTimeTable { date, lessons: lessons.remove(&date).unwrap_or_default() })
            .collect();

        all_timetables.insert(entity, WeekTimeTable { days });
    }

    Ok((all_timetables, pre_selected))
}
