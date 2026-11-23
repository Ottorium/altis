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
            <nav class="d-none d-md-flex shadow p-4 flex-column justify-content-between sticky-top side-nav" aria-label="Main">
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

            <nav class="d-md-none order-last px-2 shadow-lg bottom-nav" aria-label="Main">
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
                class={classes!("nav-link", "side-nav-item", "py-3", "text-start", "d-flex", "align-items-center", active_class)}
                aria-current={(props.active_tab == tab).then_some("page")}
            >
                <i class={classes!("bi", icon(tab), "me-3", "fs-5")} aria-hidden="true"></i>
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
            class={classes!("btn", "border-0", "p-0", "flex-grow-1", "bottom-nav-item", active_class)}
            aria-current={(props.active_tab == tab).then_some("page")}
        >
            <div class="d-flex flex-column align-items-center justify-content-center py-2">
                <i class={classes!("bi", icon(tab))} aria-hidden="true"></i>
                <span class="text-uppercase bottom-nav-label">
                    { tab.label() }
                </span>
            </div>
        </button>
    }
}
