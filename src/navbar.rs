use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum Tab {
    Timetable,
    Messages,
    Absences,
    Settings,
}

#[derive(Properties, PartialEq)]
pub struct NavProps {
    pub active_tab: Tab,
    pub on_change: Callback<Tab>,
}

#[function_component(NavBar)]
pub fn navbar(props: &NavProps) -> Html {
    let main_tabs = vec![
        (Tab::Timetable, "Timetable"),
        (Tab::Messages, "Messages"),
        (Tab::Absences, "Absences"),
    ];

    html! {
        <nav class="vh-100 bg-dark border-end border-secondary border-opacity-25 p-4 d-flex flex-column justify-content-between sticky-top"
             style="min-width: 240px; width: 240px;">

            <div class="d-flex flex-column">
                <div class="mb-3">
                    <span class="fs-3 fw-bold text-primary px-2">{"Altis"}</span>
                </div>

                <ul class="nav nav-pills flex-column gap-1 border-top border-primary border-opacity-100 pt-4">
                    {for main_tabs.into_iter().map(|(tab, label)| {
                        render_nav_item(tab, label, props)
                    })}
                </ul>
            </div>

            <div class="mt-auto pt-3 border-top border-primary border-opacity-100">
                <ul class="nav nav-pills flex-column">
                    {render_nav_item(Tab::Settings, "Settings", props)}
                </ul>
            </div>
        </nav>
    }
}

fn render_nav_item(tab: Tab, label: &'static str, props: &NavProps) -> Html {
    let cb = props.on_change.clone();
    let is_active = props.active_tab == tab;
    let current_tab = tab.clone();

    let active_class = if is_active {
        "active selected-gradient text-white"
    } else {
        "text-secondary"
    };

    html! {
    <li class="nav-item">
        <button
            onclick={Callback::from(move |_| cb.emit(current_tab.clone()))}
            class={classes!("nav-link", "py-3", "text-start", active_class)}
            style="border: none; background: transparent; width: 100%; transition: all 0.25s;"
        >
            <span class="fw-semibold">{label}</span>
        </button>
    </li>
}
}