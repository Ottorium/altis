use web_sys::HtmlSelectElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ControlsProps {
    pub category: String,
    pub selected_name: Option<String>,
    pub filtered_names: Vec<String>,
    pub on_category_change: Callback<String>,
    pub on_entity_change: Callback<String>,
    pub on_reload: Callback<()>,
}

#[function_component(TimetableControls)]
pub fn timetable_controls(props: &ControlsProps) -> Html {
    let category = props.category.clone();
    let filtered_names = props.filtered_names.clone();
    let selected_name = props.selected_name.clone();
    let on_reload = props.on_reload.clone();

    let on_cat_change = {
        let cb = props.on_category_change.clone();
        Callback::from(move |e: Event| {
            let val = e.target_unchecked_into::<HtmlSelectElement>().value();
            cb.emit(val);
        })
    };

    let on_ent_change = {
        let cb = props.on_entity_change.clone();
        Callback::from(move |e: Event| {
            let val = e.target_unchecked_into::<HtmlSelectElement>().value();
            cb.emit(val);
        })
    };

    html! {
        <div class="sticky-top p-3 mb-3 shadow-lg" style="background-color: #1e1e1e; border-bottom: 1px solid #1f2227;">
            <div class="d-flex align-items-center">
                <select class="form-select form-select-sm-md bg-dark text-white border-0 shadow-sm w-auto me-2 select-primary-dropdown-icon" onchange={on_cat_change}>
                    <option value="Class" selected={category == "Class"}>{"Class"}</option>
                    <option value="Teacher" selected={category == "Teacher"}>{"Teacher"}</option>
                    <option value="Room" selected={category == "Room"}>{"Room"}</option>
                </select>

                <select class="form-select form-select-sm-md bg-dark text-white border-0 shadow-sm w-auto me-2 select-primary-dropdown-icon" onchange={on_ent_change}>
                    {for filtered_names.iter().map(|name| {
                        html! {
                            <option value={name.clone()} selected={selected_name.as_ref() == Some(name)}>
                                { name }
                            </option>
                        }
                    })}
                </select>

                <button class="btn btn-outline-primary ms-auto" onclick={move |_| on_reload.emit(())}>
                    <i class="bi bi-arrow-clockwise me-1"></i>
                    <span>{"Reload"}</span>
                </button>
            </div>
        </div>
    }
}
