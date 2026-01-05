use crate::components::timetable::timetable_render::TimeTableRender;
use crate::data_models::clean_models::clean_models::{Entity, WeekTimeTable};
use crate::teacher_table_generator::get_all_timetables;
use crate::untis_week::Week;
use web_sys::HtmlSelectElement;
use yew::prelude::*;
use yew::suspense::use_future_with;

#[function_component(TimetableComponent)]
pub fn timetable() -> HtmlResult {
    let reload_trigger = use_state(|| 0);
    let category = use_state(|| "Class".to_string());
    let selected_name = use_state(|| None::<String>);

    let res = {
        let reload_trigger = reload_trigger.clone();
        use_future_with(*reload_trigger, |_| async move {
            get_all_timetables(Week::current()).await
        })?
    };

    if let Ok((map, Some(id))) = &*res && selected_name.is_none() {
        let initial = map.keys().find(|e| {
            if let Entity::Class(c) = e { c.id == *id } else { false }
        }).map(|e| get_entity_name(e));

        if let Some(name) = initial {
            selected_name.set(Some(name));
        }
    }

    let on_reload = {
        let reload_trigger = reload_trigger.clone();
        let selected_name = selected_name.clone();
        Callback::from(move |_| {
            selected_name.set(None);
            reload_trigger.set(*reload_trigger + 1);
        })
    };

    match &*res {
        Err(err) => Ok(html! { <div class="alert alert-danger m-3">{ err }</div> }),
        Ok((map, _)) => {
            let mut filtered_items: Vec<(&Entity, &WeekTimeTable)> = map.iter()
                .filter(|(entity, _)| match (category.as_str(), entity) {
                    ("Class", Entity::Class(_)) => true,
                    ("Teacher", Entity::Teacher(_)) => true,
                    ("Room", Entity::Room(_)) => true,
                    _ => false,
                })
                .collect();

            filtered_items.sort_by(|(a, _), (b, _)| get_entity_name(a).cmp(&get_entity_name(b)));

            let active_item = filtered_items.iter()
                .find(|(e, _)| Some(get_entity_name(e)) == *selected_name)
                .or(filtered_items.first());

            let on_category_change = {
                let category = category.clone();
                let selected_name = selected_name.clone();
                Callback::from(move |e: Event| {
                    let val = e.target_unchecked_into::<HtmlSelectElement>().value();
                    category.set(val);
                    selected_name.set(None);
                })
            };

            let on_entity_change = {
                let selected_name = selected_name.clone();
                Callback::from(move |e: Event| {
                    let val = e.target_unchecked_into::<HtmlSelectElement>().value();
                    selected_name.set(Some(val));
                })
            };

            Ok(html! {
                <>
                    <div class="sticky-top p-3 mb-3 shadow-lg" style="background-color: #1e1e1e; border-bottom: 1px solid #1f2227;">
                        <div class="d-flex align-items-center">
                             <select
                                class="form-select form-select-sm-md bg-dark text-white border-0 shadow-sm w-auto me-2 select-primary-dropdown-icon"
                                onchange={on_category_change}
                            >
                                <option value="Class" selected={*category == "Class"}>{"Class"}</option>
                                <option value="Teacher" selected={*category == "Teacher"}>{"Teacher"}</option>
                                <option value="Room" selected={*category == "Room"}>{"Room"}</option>
                            </select>

                            <select
                                class="form-select form-select-sm-md bg-dark text-white border-0 shadow-sm w-auto me-2 select-primary-dropdown-icon"
                                onchange={on_entity_change}
                            >
                                {for filtered_items.iter().map(|(entity, _)| {
                                    let name = get_entity_name(entity);
                                    html! {
                                        <option value={name.clone()} selected={active_item.map(|(e, _)| get_entity_name(e) == name).unwrap_or(false)}>
                                            { name }
                                        </option>
                                    }
                                })}
                            </select>

                            <button
                                class="btn btn-outline-primary ms-auto d-flex align-items-center"
                                onclick={on_reload}
                                title="Reload Untis Data"
                            >
                                <i class="bi bi-arrow-clockwise me-1"></i>
                                <span>{"Reload"}</span>
                            </button>
                        </div>
                    </div>

                    <div class="flex-grow-1 m-1">
                        if let Some((_, timetable)) = active_item {
                            <TimeTableRender timetable={(*timetable).clone()} />
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