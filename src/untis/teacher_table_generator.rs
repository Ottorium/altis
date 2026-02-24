use crate::data_models::clean_models::untis::{
    ChangeStatus, Class, DayTimeTable, Entity, LessonBlock, WeekTimeTable,
};
use crate::errors::ApiError;
use std::collections::HashMap;

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

    for (entity, lessons) in entity_lesson_map {
        let mut new_table = WeekTimeTable { days: vec![] };

        for (date, lessons) in lessons {
            new_table.days.push(DayTimeTable { date, lessons })
        }

        all_timetables.insert(entity, new_table);
    }

    Ok((all_timetables, pre_selected))
}
