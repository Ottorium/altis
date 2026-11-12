use altis_core::settings::Tab;
use yew::prelude::*;

fn icon(tab: Tab) -> &'static str {
    match tab {
        Tab::Timetable => "bi-calendar3",
        Tab::Messages => "bi-chat-dots",
        Tab::Absences => "bi-person-exclamation",
        Tab::Book2Eat => "bi-fork-knife",
        Tab::Settings => "bi-gear",
    }
}

#[derive(Properties, PartialEq)]
pub struct NavProps {
    /// the tabs on offer, in order
    pub tabs: Vec<Tab>,
    pub active_tab: Tab,
    pub on_change: Callback<Tab>,
}

/// The app's navigation: a sidebar on a desktop, a bar along the bottom on a phone
#[function_component(NavBar)]
pub fn navbar(props: &NavProps) -> Html {
    // the sidebar sets the settings apart, at its bottom
    let (settings, main): (Vec<Tab>, Vec<Tab>) = props.tabs.iter().partition(|tab| **tab == Tab::Settings);

    html! {
        <>
            <nav class="d-none d-md-flex vh-100 shadow p-4 flex-column justify-content-between sticky-top"
                 style="min-width: 240px; width: 240px; background-color: #1a1a1a; border-right: 1px solid #1e1e1e;">
                <div class="d-flex flex-column">
                    <div class="mb-3">
                        <span class="fs-3 fw-bold text-primary px-2">{"Altis"}</span>
                    </div>

                    <ul class="nav nav-pills flex-column gap-1 border-top border-primary border-opacity-100 pt-4">
                        { for main.into_iter().map(|tab| render_sidebar_item(tab, props)) }
                    </ul>
                </div>
                <div class="nav nav-pills flex-column gap-1 border-top border-primary border-opacity-100 pt-4">
                    { for settings.into_iter().map(|tab| render_sidebar_item(tab, props)) }
                </div>
            </nav>

            <nav class="d-md-none order-last px-2 shadow-lg" style="background-color: #1a1a1a; border-top: 1px solid rgba(255,255,255,0.1); z-index: 1050;">
                <div class="d-flex justify-content-around py-2">
                    { for props.tabs.iter().map(|tab| render_bottom_bar_item(*tab, props)) }
                </div>
            </nav>
        </>
    }
}

fn render_sidebar_item(tab: Tab, props: &NavProps) -> Html {
    let active_class = match props.active_tab == tab {
        true => "active selected-gradient text-white",
        false => "text-secondary",
    };

    html! {
        <li class="nav-item">
            <button
                onclick={props.on_change.reform(move |_| tab)}
                class={classes!("nav-link", "py-3", "text-start", "d-flex", "align-items-center", active_class)}
                style="border: none; background: transparent; width: 100%; transition: all 0.25s;"
            >
                <i class={classes!("bi", icon(tab), "me-3", "fs-5")}></i>
                <span class="fw-semibold">{ tab.label() }</span>
            </button>
        </li>
    }
}

fn render_bottom_bar_item(tab: Tab, props: &NavProps) -> Html {
    let active_class = match props.active_tab == tab {
        true => "text-primary fw-bold",
        false => "text-secondary opacity-75 fw-medium",
    };

    html! {
        <button
            onclick={props.on_change.reform(move |_| tab)}
            class={classes!("btn", "border-0", "p-0", "flex-grow-1", active_class)}
            style="background: transparent; transition: color 0.2s ease;"
        >
            <div class="d-flex flex-column align-items-center justify-content-center py-2">
                <i class={classes!("bi", icon(tab))} style="font-size: 1.6rem; line-height: 1;"></i>
                <span style="font-size: 0.7rem; letter-spacing: 0.03rem; margin-top: 4px;" class="text-uppercase">
                    { tab.label() }
                </span>
            </div>
        </button>
    }
}
