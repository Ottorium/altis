use crate::data_models::clean_models::clean_models::*;
use crate::untis_client::get_classes;
use crate::untis_week::Week;
use std::collections::HashMap;

pub async fn get_all_timetables(
    week: Week,
) -> Result<(HashMap<Entity, WeekTimeTable>, Option<i32>), String> {
    let (classes, pre_selected) = get_classes(Week::current()).await?;
    let class_results = crate::untis_client::get_multiple_timetables(week.clone(), &classes).await?;

    let mut all_timetables: HashMap<Entity, WeekTimeTable> = class_results
        .into_iter()
        .map(|(class, table)| (Entity::Class(class), table))
        .collect();

    let mut entity_lesson_map: HashMap<Entity, Vec<(usize, LessonBlock)>> = HashMap::new();

    for table in all_timetables.values() {
        for (i, day_table) in table.days.iter().enumerate() {
            for lesson in &day_table.lessons {
                for entity_wrapper in &lesson.entities {
                    if entity_wrapper.status == ChangeStatus::Removed {
                        continue;
                    }

                    match &entity_wrapper.data {
                        Entity::Teacher(_) | Entity::Room(_) => {
                            entity_lesson_map
                                .entry(entity_wrapper.data.clone())
                                .or_default()
                                .push((i, lesson.clone()));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    for (entity, lessons) in entity_lesson_map {
        let mut new_table = WeekTimeTable {
            days: vec![DayTimeTable { lessons: vec![] }; 5],
        };

        for (i, lesson) in lessons {
            new_table.days[i].lessons.push(lesson);
        }

        all_timetables.insert(entity, new_table);
    }

    Ok((all_timetables, pre_selected))
}