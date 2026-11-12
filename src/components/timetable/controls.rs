use super::compare::Compared;
use super::view::TimetableView;
use crate::components::forms::{input_value, select_value};
use altis_core::settings::Favorites;
use altis_core::untis::week::Week;
use chrono::NaiveDate;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

const DATE_INPUT_FORMAT: &str = "%Y-%m-%d";

#[derive(Properties, PartialEq)]
pub struct ControlsProps {
    pub view: TimetableView,
    pub selected_name: Option<String>,
    pub selected_week: Week,
    /// the classes, teachers or rooms there are to pick from
    pub names: Vec<String>,
    pub favorites: Favorites,
    /// the timetables picked for the "Compare" view
    pub compared: Vec<Compared>,
    pub loading: bool,
    pub on_view_change: Callback<TimetableView>,
    pub on_entity_change: Callback<String>,
    pub on_week_change: Callback<Week>,
    pub on_reload: Callback<()>,
    /// the selected class, teacher or room was starred or unstarred
    pub on_toggle_favorite: Callback<String>,
    pub on_open_picker: Callback<()>,
}

/// The bar above the timetable: what to show, of whom, and for which week
#[function_component(TimetableControls)]
pub fn timetable_controls(props: &ControlsProps) -> Html {
    let view = props.view;
    let selected_name = props.selected_name.clone();
    let is_favorite = |name: &str| view.entity_kind().is_some_and(|kind| props.favorites.contains(kind, name));

    let on_view_change = props.on_view_change.reform(|e: Event| TimetableView::from_key(&select_value(&e)).unwrap_or(TimetableView::Me));
    let on_entity_change = props.on_entity_change.reform(|e: Event| select_value(&e));

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

    // favourites come first, so they're found without scrolling through every teacher
    let (favorite_names, other_names): (Vec<&String>, Vec<&String>) = props.names.iter().partition(|name| is_favorite(name));
    let option = |name: &String| html! {
        <option value={name.clone()} selected={selected_name.as_ref() == Some(name)}>{ name }</option>
    };

    let favorite_button = selected_name.clone().filter(|_| view.entity_kind().is_some()).map(|name| {
        let is_favorite = is_favorite(&name);
        let onclick = props.on_toggle_favorite.reform(move |_| name.clone());
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

    let step_week = |week: Week| props.on_week_change.reform(move |_| week.clone());
    let on_date_change = {
        let on_week_change = props.on_week_change.clone();
        Callback::from(move |e: Event| {
            if let Ok(date) = NaiveDate::parse_from_str(&input_value(&e), DATE_INPUT_FORMAT) {
                on_week_change.emit(Week::from_date(date));
            }
        })
    };
    let week_start = props.selected_week.start.format(DATE_INPUT_FORMAT).to_string();

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
                        <span class="select-fit-label">{ view.label() }</span>
                        <select class="form-select form-select-sm-md bg-dark text-white border-0 shadow-sm select-primary-dropdown-icon" onchange={on_view_change}>
                            { for TimetableView::ALL.into_iter().map(|option| html! {
                                <option value={option.key()} selected={view == option}>{ option.label() }</option>
                            })}
                        </select>
                    </div>

                    if !props.names.is_empty() {
                        <div class="select-fit me-2">
                            // with nothing selected the browser shows the first option
                            <span class="select-fit-label">
                                { selected_name.clone().or_else(|| props.names.first().cloned()).unwrap_or_default() }
                            </span>
                            <select ref={entity_select} class="form-select form-select-sm-md bg-dark text-white border-0 shadow-sm select-primary-dropdown-icon" onchange={on_entity_change}>
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

                    if view == TimetableView::Compare {
                        <button type="button" onclick={props.on_open_picker.reform(|_| ())}
                                class="form-select bg-dark text-white border-0 shadow-sm select-primary-dropdown-icon text-start text-truncate me-2"
                                style="width: auto; min-width: 0; max-width: 16rem;">
                            <i class="bi bi-ui-checks me-1"></i>{ compare_label }
                        </button>
                    }
                }

                // the week on a desktop: an arrow either way around the date picker
                <div class="btn-group shadow-sm ms-md-2 d-none d-md-inline-flex" role="group">
                    <button type="button" class="btn btn-outline-primary" onclick={step_week(props.selected_week.previous())}>
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
                            value={week_start.clone()}
                            onchange={on_date_change.clone()}
                        />
                    </div>

                    <button type="button" class="btn btn-outline-primary" onclick={step_week(props.selected_week.next())}>
                        <i class="bi bi-chevron-right"></i>
                    </button>
                </div>

                // and on a phone, where a swipe does the arrows' job: the date picker alone
                <div class="d-block d-md-none">
                    <div class="btn btn-outline-primary position-relative w-100 d-flex align-items-center justify-content-center">
                        <i class="bi bi-calendar3"></i>
                        <input
                            type="date"
                            class="position-absolute opacity-0 w-100 h-100 start-0 top-0"
                            value={week_start}
                            onchange={on_date_change}
                        />
                    </div>
                </div>

                <button class="btn btn-outline-primary ms-auto" onclick={props.on_reload.reform(|_| ())}>
                    <i class="bi bi-arrow-clockwise me-sm-1"></i>
                    <span class="d-none d-sm-inline">{"Reload"}</span>
                </button>
            </div>
        </div>
    }
}
