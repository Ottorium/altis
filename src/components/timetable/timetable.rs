use crate::components::timetable::timetable_controls::TimetableControls;
use crate::components::timetable::timetable_render::TimeTableRender;
use crate::data_models::clean_models::untis::{Entity, WeekTimeTable};
use crate::untis::untis_week::Week;
use yew::prelude::*;
use yew::suspense::use_future_with;
use crate::untis::cached_untis_client::CachedUntisClient;

#[function_component(TimetableComponent)]
pub fn timetable() -> HtmlResult {
    let reload_trigger = use_state(|| 0);
    let category = use_state(|| "Class".to_string());
    let selected_name = use_state(|| None::<String>);
    let selected_week = use_state(Week::current);

    let res = {
        let trigger = *reload_trigger;
        let selected_week = selected_week.clone();
        use_future_with(trigger, |_| async move {
            CachedUntisClient::new()?.get_all_timetables((*selected_week).clone()).await
        })?
    };

    match &*res {
        Err(err) => Ok(html! { <div class="alert alert-danger m-3">{ err.to_string() }</div> }),
        Ok((map, initial_id)) => {
            if selected_name.is_none()
                && let Some(id) = initial_id {
                    let initial = map.keys().find(|e| {
                        if let Entity::Class(c) = e { c.id == *id } else { false }
                    }).map(|e| e.name());
                    selected_name.set(initial);
                }

            let filtered_data: Vec<(&Entity, &WeekTimeTable)> = map.iter()
                .filter(|(entity, _)| match (category.as_str(), entity) {
                    ("Class", Entity::Class(_)) => true,
                    ("Teacher", Entity::Teacher(_)) => true,
                    ("Room", Entity::Room(_)) => true,
                    _ => false,
                })
                .collect();

            let mut names: Vec<String> = filtered_data.iter().map(|(e, _)| e.name()).collect();
            names.sort();

            let active_timetable = filtered_data.iter()
                .find(|(e, _)| Some(e.name()) == *selected_name)
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
                    let _ = CachedUntisClient::clear_cache();
                    selected_name.set(None);
                    trigger.set(*trigger + 1);
                })
            };

            let on_entity_change = {
                let selected_name = selected_name.clone();
                Callback::from(move |name| selected_name.set(Some(name)))
            };

            let on_week_change = {
                let selected_week = selected_week.clone();
                let trigger = reload_trigger.clone();
                Callback::from(move |week| { selected_week.set(week); trigger.set(*trigger + 1); })
            };

            let on_swipe_next = {
                let selected_week = selected_week.clone();
                let trigger = reload_trigger.clone();
                Callback::from(move |_| {
                    let next = (*selected_week).next();
                    selected_week.set(next);
                    trigger.set(*trigger + 1);
                })
            };

            let on_swipe_prev = {
                let selected_week = selected_week.clone();
                let trigger = reload_trigger.clone();
                Callback::from(move |_| {
                    let prev = (*selected_week).previous();
                    selected_week.set(prev);
                    trigger.set(*trigger + 1);
                })
            };

            Ok(html! {
                <div class="d-flex flex-column flex-grow-1 h-100">
                    <TimetableControls
                        category={(*category).clone()}
                        selected_name={(*selected_name).clone()}
                        selected_week={(*selected_week).clone()}
                        filtered_names={names}
                        on_category_change={on_category_change}
                        on_entity_change={on_entity_change}
                        on_week_change={on_week_change}
                        on_reload={on_reload}
                    />
                    <div class="d-flex flex-column flex-grow-1 w-100" style="overflow-y: auto;">
                        if let Some(tt) = active_timetable {
                            <TimeTableRender
                                timetable={tt}
                                on_next={on_swipe_next}
                                on_prev={on_swipe_prev}
                            />
                        } else {
                            <p class="text-light"> {"No selection made"} </p>
                        }
                    </div>
                </div>
            })
        }
    }
}