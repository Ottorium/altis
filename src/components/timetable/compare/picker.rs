use super::Compared;
use crate::components::forms::on_text_input;
use crate::components::modal::Modal;
use altis_core::models::timetable::EntityKind;
use altis_core::settings::Favorites;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ComparePickerProps {
    /// the names to pick from per kind of entity, empty while the week is still loading
    pub options: Vec<(EntityKind, Vec<String>)>,
    pub favorites: Favorites,
    pub compared: Vec<Compared>,
    pub on_change: Callback<Vec<Compared>>,
    pub on_close: Callback<()>,
}

fn section_title(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Class => "Classes",
        EntityKind::Teacher => "Teachers",
        EntityKind::Room => "Rooms",
    }
}

/// Whether a name is one the search is looking for. `needle` is the search in lower case.
fn matches(name: &str, needle: &str) -> bool {
    needle.is_empty() || name.to_lowercase().contains(needle)
}

/// The names of a kind the search is looking for, as timetables to pick
fn matching<'a>(kind: EntityKind, names: impl IntoIterator<Item = &'a String>, needle: &str) -> Vec<Compared> {
    names.into_iter().filter(|name| matches(name, needle)).map(|name| Compared::Entity { kind, name: name.clone() }).collect()
}

/// The dialog the timetables to compare are picked in
#[function_component(ComparePicker)]
pub fn compare_picker(props: &ComparePickerProps) -> Html {
    let filter = use_state(String::new);

    let needle = filter.trim().to_lowercase();

    // a chip per timetable, which picks or unpicks it
    let chip = |item: Compared| {
        let is_picked = props.compared.contains(&item);
        let onclick = {
            let (compared, on_change, item) = (props.compared.clone(), props.on_change.clone(), item.clone());
            Callback::from(move |_| {
                let mut next = compared.clone();
                match next.iter().position(|picked| *picked == item) {
                    Some(position) => {
                        next.remove(position);
                    }
                    None => next.push(item.clone()),
                }
                on_change.emit(next);
            })
        };
        html! {
            <button type="button" {onclick}
                    class={classes!("btn", "btn-sm", "rounded-pill", if is_picked { "btn-primary" } else { "btn-outline-secondary" })}>
                <i class={classes!("bi", item.icon(), "me-1")}></i>{ item.label() }
            </button>
        }
    };
    let section = |title: &str, items: Vec<Compared>| {
        if items.is_empty() {
            return html! {};
        }
        html! {
            <div class="mb-3">
                <div class="small fw-bold text-secondary mb-1">{ title.to_string() }</div>
                <div class="d-flex flex-wrap gap-1">{ for items.into_iter().map(chip) }</div>
            </div>
        }
    };

    let favorites: Vec<Compared> = EntityKind::ALL.into_iter().flat_map(|kind| matching(kind, props.favorites.names(kind), &needle)).collect();
    let me: Vec<Compared> = matches("me", &needle).then_some(Compared::Me).into_iter().collect();

    let footer = html! {
        <button type="button" class="btn btn-primary" onclick={props.on_close.reform(|_| ())}>{ "Done" }</button>
    };

    html! {
        <Modal title="Compare Timetables" on_close={props.on_close.clone()} large=true scrollable=true {footer}>
            <div class="small text-secondary mb-2">{ "Pick two or more to show side by side." }</div>
            <input type="search" class="form-control form-control-sm bg-dark text-light border-secondary mb-3"
                   placeholder="Search" value={(*filter).clone()} oninput={on_text_input(&filter)} />
            { section("Comparing", props.compared.clone()) }
            { section("Me", me) }
            { section("Favourites", favorites) }
            if props.options.is_empty() {
                <div class="small text-secondary">{ "Classes, teachers and rooms show up here once the week has loaded." }</div>
            }
            { for props.options.iter().map(|(kind, names)| section(section_title(*kind), matching(*kind, names, &needle))) }
        </Modal>
    }
}
