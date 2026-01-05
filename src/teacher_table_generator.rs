use crate::data_models::clean_models::clean_models::*;
use crate::untis_client::get_classes;
use crate::untis_week::Week;
use std::collections::HashMap;

pub async fn get_all_timetables(
    week: Week,
) -> Result<(HashMap<Entity, WeekTimeTable>, Option<i32>), String> {
    let (classes, pre_selected) = get_classes(Week::current()).await?;
    let res = crate::untis_client::get_multiple_timetables(week.clone(), &classes).await?;
    let mut all_timetables: HashMap<Entity, WeekTimeTable> = HashMap::new();
    for (class, table) in res.iter() {
        all_timetables.insert(Entity::Class(class.clone()), table.clone());
    }
    let class_timetables: Vec<WeekTimeTable> = res.into_values().collect();
    let lessons = class_timetables
        .iter()
        .flat_map(|timetable| timetable.days.clone())
        .flat_map(|day| day.lessons)
        .collect::<Vec<_>>();

    let teachers = lessons
        .iter()
        .flat_map(|lesson| &lesson.entities)
        .filter(|e| e.status != ChangeStatus::Removed)
        .filter_map(|e| {
            if let Entity::Teacher(t) = e.data.clone() {
                Some(t)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let rooms = lessons
        .iter()
        .flat_map(|lesson| &lesson.entities)
        .filter(|e| e.status != ChangeStatus::Removed)
        .filter_map(|e| {
            if let Entity::Room(r) = e.data.clone() {
                Some(r)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    for t in teachers {
        let mut table = WeekTimeTable {
            days: vec![DayTimeTable { lessons: vec![] }; 5],
        };

        for tt in &class_timetables {
            for (i, dt) in tt.days.iter().enumerate() {
                table.days[i].lessons.extend(
                    dt.lessons
                        .iter()
                        .filter(|l| {
                            l.entities.iter().any(|e| {
                                if let Entity::Teacher(x) = e.data.clone() {
                                    x == t
                                } else {
                                    false
                                }
                            })
                        })
                        .cloned(),
                );
            }
        }

        all_timetables.insert(Entity::Teacher(t), table);
    }

    for r in rooms {
        let mut table = WeekTimeTable {
            days: vec![DayTimeTable { lessons: vec![] }; 5],
        };

        for tt in &class_timetables {
            for (i, dt) in tt.days.iter().enumerate() {
                table.days[i].lessons.extend(
                    dt.lessons
                        .iter()
                        .filter(|l| {
                            l.entities.iter().any(|e| {
                                if let Entity::Room(x) = e.data.clone() {
                                    x == r
                                } else {
                                    false
                                }
                            })
                        })
                        .cloned(),
                );
            }
        }

        all_timetables.insert(Entity::Room(r), table);
    }

    Ok((all_timetables, pre_selected))
}
