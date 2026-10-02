use crate::components::timetable::compare::Compared;
use altis_core::settings::Favorites;
use altis_core::untis::untis_week::Week;
use chrono::NaiveDate;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;

/// What can be shown, as the value the timetable uses and the label the picker shows
const CATEGORIES: [(&str, &str); 6] = [
    ("Me", "Me"),
    ("Class", "Class"),
    ("Teacher", "Teacher"),
    ("Room", "Room"),
    ("AvailableRooms", "Available Rooms"),
    ("Compare", "Compare"),
];

fn category_label(category: &str) -> &'static str {
    CATEGORIES.iter()
        .find(|(value, _)| *value == category)
        .map_or("", |(_, label)| *label)
}

#[derive(Properties, PartialEq)]
pub struct ControlsProps {
    pub category: String,
    pub selected_name: Option<String>,
    pub selected_week: Week,
    pub filtered_names: Vec<String>,
    pub favorites: Favorites,
    /// the timetables picked for the "Compare" category
    pub compared: Vec<Compared>,
    pub loading: bool,
    pub on_category_change: Callback<String>,
    pub on_entity_change: Callback<String>,
    pub on_week_change: Callback<Week>,
    pub on_reload: Callback<()>,
    /// the selected class, teacher or room was starred or unstarred
    pub on_toggle_favorite: Callback<String>,
    pub on_open_picker: Callback<()>,
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

    let entity_select = use_node_ref();
    {
        let entity_select = entity_select.clone();
        let selected_name = selected_name.clone();
        // the `selected` attribute stops deciding what a select shows once the user picked an option
        // by hand, and moving options in and out of the favourites group can leave WebKit showing
        // another one, so the shown entry is set directly after every render
        use_effect(move || {
            if let (Some(select), Some(name)) = (entity_select.cast::<HtmlSelectElement>(), selected_name)
                && select.value() != name {
                select.set_value(&name);
            }
        });
    }

    let on_ent_change = {
        let cb = props.on_entity_change.clone();
        Callback::from(move |e: Event| {
            let val = e.target_unchecked_into::<HtmlSelectElement>().value();
            cb.emit(val);
        })
    };

    // favourites come first, so they're found without scrolling through every teacher
    let (favorite_names, other_names): (Vec<&String>, Vec<&String>) = filtered_names.iter()
        .partition(|name| props.favorites.contains(&category, name));
    let option = |name: &String| html! {
        <option value={name.clone()} selected={selected_name.as_ref() == Some(name)}>{ name }</option>
    };

    let favorite_button = selected_name.clone()
        .filter(|_| matches!(category.as_str(), "Class" | "Teacher" | "Room"))
        .map(|name| {
            let is_favorite = props.favorites.contains(&category, &name);
            let onclick = {
                let cb = props.on_toggle_favorite.clone();
                Callback::from(move |_| cb.emit(name.clone()))
            };
            html! {
                <button type="button" class="btn btn-link p-1 me-2 flex-shrink-0" {onclick}
                        title={if is_favorite { "Remove from favourites" } else { "Add to favourites" }}>
                    <i class={classes!("bi", "fs-5", if is_favorite { "bi-star-fill text-primary" } else { "bi-star text-secondary" })}></i>
                </button>
            }
        });

    let compare_label = match props.compared.is_empty() {
        true => "Pick timetables".to_string(),
        false => props.compared.iter().map(Compared::label).collect::<Vec<_>>().join(", "),
    };
    let on_pick = {
        let cb = props.on_open_picker.clone();
        Callback::from(move |_| cb.emit(()))
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
                if props.loading {
                    <div class="d-flex align-items-center text-secondary px-2 me-2" role="status">
                        <div class="spinner-border spinner-border-sm text-primary me-2"></div>
                        <span>{"Loading..."}</span>
                    </div>
                } else {
                    <div class="select-fit me-2">
                        <span class="select-fit-label">{ category_label(&category) }</span>
                        <select class="form-select form-select-sm-md bg-dark text-white border-0 shadow-sm select-primary-dropdown-icon" onchange={on_cat_change}>
                            {for CATEGORIES.iter().map(|(value, label)| html! {
                                <option value={*value} selected={category == *value}>{ *label }</option>
                            })}
                        </select>
                    </div>

                    if !filtered_names.is_empty() {
                        <div class="select-fit me-2">
                            // with nothing selected the browser shows the first option
                            <span class="select-fit-label">
                                { selected_name.clone().or_else(|| filtered_names.first().cloned()).unwrap_or_default() }
                            </span>
                            <select ref={entity_select} class="form-select form-select-sm-md bg-dark text-white border-0 shadow-sm select-primary-dropdown-icon" onchange={on_ent_change}>
                                if favorite_names.is_empty() {
                                    { for other_names.into_iter().map(option) }
                                } else {
                                    <optgroup label="Favourites">{ for favorite_names.into_iter().map(option) }</optgroup>
                                    <optgroup label="Others">{ for other_names.into_iter().map(option) }</optgroup>
                                }
                            </select>
                        </div>
                        { favorite_button }
                    }

                    if category == "Compare" {
                        <button type="button" onclick={on_pick}
                                class="form-select bg-dark text-white border-0 shadow-sm select-primary-dropdown-icon text-start text-truncate me-2"
                                style="width: auto; min-width: 0; max-width: 16rem;">
                            <i class="bi bi-ui-checks me-1"></i>{ compare_label }
                        </button>
                    }
                }

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
                            class="position-absolute opacity-0 w-100 h-100 start-0 top-0"
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