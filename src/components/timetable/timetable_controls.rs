use crate::untis_week::Week;
use chrono::NaiveDate;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ControlsProps {
    pub category: String,
    pub selected_name: Option<String>,
    pub selected_week: Week,
    pub filtered_names: Vec<String>,
    pub on_category_change: Callback<String>,
    pub on_entity_change: Callback<String>,
    pub on_week_change: Callback<Week>,
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

    let on_prev_week = {
        let cb = props.on_week_change.clone();
        let current_week = props.selected_week.clone();
        Callback::from(move |_| {
            cb.emit(current_week.previous());
        })
    };

    let on_next_week = {
        let cb = props.on_week_change.clone();
        let current_week = props.selected_week.clone();
        Callback::from(move |_| {
            cb.emit(current_week.next());
        })
    };

    let on_date_change = {
        let cb = props.on_week_change.clone();
        Callback::from(move |e: Event| {
            let val = e.target_unchecked_into::<HtmlInputElement>().value();
            if let Ok(date) = NaiveDate::parse_from_str(&val, "%Y-%m-%d") {
                cb.emit(Week::from_date(date));
            }
        })
    };

    html! {
        <div class="sticky-top p-3 mb-1 shadow-lg" style="background-color: #1e1e1e; border-bottom: 1px solid #1f2227;">
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

                // DESKTOP DATE SELECTOR
                <div class="btn-group shadow-sm ms-md-2 d-none d-md-inline-flex" role="group">
                    <button type="button" class="btn btn-outline-primary" onclick={on_prev_week.clone()}>
                        <i class="bi bi-chevron-left"></i>
                    </button>

                    <div
                        class="btn btn-outline-primary position-relative d-flex align-items-center justify-content-center px-3"
                        style="min-width: 45px;"
                    >
                        <span>{ props.selected_week.to_string() }</span>

                        <input
                            type="date"
                            class="position-absolute opacity-0 w-100 h-100 start-0 top-0 text-sm"
                            style="cursor: pointer;"
                            value={ props.selected_week.start.clone() }
                            onchange={on_date_change.clone()}
                        />
                    </div>

                    <button type="button" class="btn btn-outline-primary" onclick={on_next_week}>
                        <i class="bi bi-chevron-right"></i>
                    </button>
                </div>

                // MOBILE DATE PICKER
                <div class="d-block d-md-none">
                    <div class="btn btn-outline-primary position-relative w-100 d-flex align-items-center justify-content-center">
                        <i class="bi bi-calendar3"></i>
                        <input
                            type="date"
                            class="position-absolute opacity-0 start-0 top-0"
                            value={ props.selected_week.start.clone() }
                            onchange={on_date_change}
                        />
                    </div>
                </div>

                <button class="btn btn-outline-primary ms-auto" onclick={let on_reload = on_reload.clone(); move |_| on_reload.emit(())}>
                    <i class="bi bi-arrow-clockwise me-sm-1"></i>
                    <span class="d-none d-sm-inline">{"Reload"}</span>
                </button>
            </div>
        </div>
    }
}