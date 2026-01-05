use crate::data_models::clean_models::clean_models::Class;
use crate::untis_client;
use crate::untis_week::Week;
use web_sys::HtmlSelectElement;
use yew::prelude::*;
use yew::suspense::use_future_with;

#[function_component(TimetableComponent)]
pub fn timetable() -> HtmlResult {
    let reload_trigger = use_state(|| 0);

    let res = {
        let reload_trigger = reload_trigger.clone();
        use_future_with(*reload_trigger, |_| async move {
            untis_client::get_classes(Week::current()).await
        })?
    };

    let on_reload = {
        let reload_trigger = reload_trigger.clone();
        Callback::from(move |_| {
            reload_trigger.set(*reload_trigger + 1);
        })
    };

    let category = use_state(|| "Class".to_string());
    let selected_name = use_state(|| None::<String>);

    match &*res {
        Err(err) => Ok(html! { <div class="alert alert-danger m-3">{ err }</div> }),
        Ok(data) => {
            let active_class = data.iter()
                .find(|c| Some(c.name.clone()) == *selected_name)
                .or(data.first());

            let on_category_change = {
                let category = category.clone();
                Callback::from(move |e: Event| {
                    let val = e.target_unchecked_into::<HtmlSelectElement>().value();
                    category.set(val);
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
                                {for data.iter().map(|c| html! {
                                    <option value={c.name.clone()}
                                            selected={active_class.map(|a| &a.name == &c.name).unwrap_or(false)}>
                                        { &c.name }
                                    </option>
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
                        if let Some(class) = active_class {
                            <ClassDetail class={class.clone()} />
                        } else {
                            <p class="text-light"> {"No selection made"} </p>
                        }
                    </div>
                </>
            })
        }
    }
}

#[derive(Properties, PartialEq, Clone)]
pub struct ClassDetailProps {
    pub class: Class,
}

#[function_component(ClassDetail)]
fn class_detail(props: &ClassDetailProps) -> Html {
    let class = &props.class;
    html! {
        <div class="class-detail-card">
            <div class="detail-content">
                <pre>{ format!("{:#?}", class) }</pre>
            </div>
        </div>
    }
}