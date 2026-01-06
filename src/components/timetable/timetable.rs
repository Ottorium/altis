use crate::components::timetable::timetable_controls::TimetableControls;
use crate::components::timetable::timetable_render::TimeTableRender;
use crate::data_models::clean_models::clean_models::{Entity, WeekTimeTable};
use crate::teacher_table_generator::get_all_timetables;
use crate::untis_week::Week;
use yew::prelude::*;
use yew::suspense::use_future_with;

#[function_component(TimetableComponent)]
pub fn timetable() -> HtmlResult {
    let reload_trigger = use_state(|| 0);
    let category = use_state(|| "Class".to_string());
    let selected_name = use_state(|| None::<String>);

    let res = {
        let trigger = *reload_trigger;
        use_future_with(trigger, |_| async move {
            get_all_timetables(Week::current()).await
        })?
    };

    match &*res {
        Err(err) => Ok(html! { <div class="alert alert-danger m-3">{ err.to_string() }</div> }),
        Ok((map, initial_id)) => {
            if selected_name.is_none() {
                if let Some(id) = initial_id {
                    let initial = map.keys().find(|e| {
                        if let Entity::Class(c) = e { c.id == *id } else { false }
                    }).map(get_entity_name);
                    selected_name.set(initial);
                }
            }

            let filtered_data: Vec<(&Entity, &WeekTimeTable)> = map.iter()
                .filter(|(entity, _)| match (category.as_str(), entity) {
                    ("Class", Entity::Class(_)) => true,
                    ("Teacher", Entity::Teacher(_)) => true,
                    ("Room", Entity::Room(_)) => true,
                    _ => false,
                })
                .collect();

            let mut names: Vec<String> = filtered_data.iter().map(|(e, _)| get_entity_name(e)).collect();
            names.sort();

            let active_timetable = filtered_data.iter()
                .find(|(e, _)| Some(get_entity_name(e)) == *selected_name)
                .or(filtered_data.first())
                .map(|(_, t)| (*t).clone());

            let on_category_change = {
                let category = category.clone();
                let selected_name = selected_name.clone();
                Callback::from(move |cat| {
                    category.set(cat);
                    selected_name.set(None);
                })
            };

            let on_reload = {
                let trigger = reload_trigger.clone();
                let selected_name = selected_name.clone();
                Callback::from(move |_| {
                    selected_name.set(None);
                    trigger.set(*trigger + 1);
                })
            };

            let on_entity_change = {
                let selected_name = selected_name.clone();
                Callback::from(move |name| selected_name.set(Some(name)))
            };

            Ok(html! {
                <>
                    <TimetableControls 
                        category={(*category).clone()}
                        selected_name={(*selected_name).clone()}
                        filtered_names={names}
                        on_category_change={on_category_change}
                        on_entity_change={on_entity_change}
                        on_reload={on_reload}
                    />
                    <div class="flex-grow-1 m-1">
                        if let Some(tt) = active_timetable {
                            <TimeTableRender timetable={tt} />
                        } else {
                            <p class="text-light"> {"No selection made"} </p>
                        }
                    </div>
                </>
            })
        }
    }
}

fn get_entity_name(entity: &Entity) -> String {
    match entity {
        Entity::Class(c) => c.name.clone(),
        Entity::Teacher(t) => t.short_name.clone(),
        Entity::Room(r) => r.name.clone(),
        Entity::Subject(s) => s.short_name.clone(),
    }
}