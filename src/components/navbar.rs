use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub enum Tab {
    Timetable,
    Messages,
    Absences,
    Settings,
    Letto,
    Book2Eat,
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
        (Tab::Letto, "Letto"),
        (Tab::Book2Eat, "Book2Eat"),
    ];

    let mut main_tabs_mobile = main_tabs.clone();
    main_tabs_mobile.push((Tab::Settings, "Settings"));

    html! {
        <>
            // DESKTOP SIDEBAR
            <nav class="d-none d-md-flex vh-100 bg-dark border-end border-secondary border-opacity-25 p-4 flex-column justify-content-between sticky-top"
                 style="min-width: 240px; width: 240px;">
                <div class="d-flex flex-column">
                    <div class="mb-3">
                        <span class="fs-3 fw-bold text-primary px-2">{"Altis"}</span>
                    </div>

                    <ul class="nav nav-pills flex-column gap-1 border-top border-primary border-opacity-100 pt-4">
                        {for main_tabs.into_iter().map(|(tab, label)| {
                            render_nav_item(tab.clone(), label, props)
                        })}
                    </ul>
                </div>
                <div class="nav nav-pills flex-column gap-1 border-top border-primary border-opacity-100 pt-4">
                    {render_nav_item(Tab::Settings, "Settings", props)}
                </div>
            </nav>

            // MOBILE BOTTOM NAV
            <nav class="d-md-none fixed-bottom bg-dark border-top border-secondary border-opacity-25 px-2">
                <div class="d-flex justify-content-around">
                    {for main_tabs_mobile.into_iter().map(|(tab, label)| {
                        render_mobile_nav_item(tab, label, props)
                    })}
                </div>
            </nav>
        </>
    }
}

fn get_icon(tab: &Tab) -> &'static str {
    match tab {
        Tab::Timetable => "bi-calendar3",
        Tab::Messages => "bi-chat-dots",
        Tab::Absences => "bi-person-exclamation",
        Tab::Settings => "bi-gear",
        Tab::Letto => "bi-mortarboard",
        Tab::Book2Eat => "bi bi-fork-knife",
    }
}

fn render_nav_item(tab: Tab, label: &'static str, props: &NavProps) -> Html {
    let cb = props.on_change.clone();
    let is_active = props.active_tab == tab;
    let current_tab = tab.clone();
    let icon_class = get_icon(&tab);

    let active_class = if is_active {
        "active selected-gradient text-white"
    } else {
        "text-secondary"
    };

    html! {
        <li class="nav-item">
            <button
                onclick={Callback::from(move |_| cb.emit(current_tab.clone()))}
                class={classes!("nav-link", "py-3", "text-start", "d-flex", "align-items-center", active_class)}
                style="border: none; background: transparent; width: 100%; transition: all 0.25s;"
            >
                <i class={classes!("bi", icon_class, "me-3", "fs-5")}></i>
                <span class="fw-semibold">{label}</span>
            </button>
        </li>
    }
}

fn render_mobile_nav_item(tab: Tab, label: &'static str, props: &NavProps) -> Html {
    let cb = props.on_change.clone();
    let is_active = props.active_tab == tab;
    let current_tab = tab.clone();
    let icon_class = get_icon(&tab);

    let active_classes = if is_active {
        "text-primary fw-bold"
    } else {
        "text-secondary opacity-75 fw-medium"
    };

    html! {
        <button
            onclick={Callback::from(move |_| cb.emit(current_tab.clone()))}
            class={classes!("btn", "border-0", "p-0", "flex-grow-1", active_classes)}
            style="background: transparent; transition: color 0.2s ease;"
        >
            <div class="d-flex flex-column align-items-center justify-content-center py-2">
                <i class={classes!("bi", icon_class)} style="font-size: 1.6rem; line-height: 1;"></i>
                <span style="font-size: 0.7rem; letter-spacing: 0.03rem; margin-top: 4px;" class="text-uppercase">
                    {label}
                </span>
            </div>
        </button>
    }
}