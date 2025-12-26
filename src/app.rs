use crate::navbar::*;
use yew::prelude::*;

#[function_component(App)]
pub fn app() -> Html {
    let active_tab = use_state(|| Tab::Timetable);

    let on_nav_change = {
        let active_tab = active_tab.clone();
        Callback::from(move |tab: Tab| active_tab.set(tab))
    };

    let content = match *active_tab {
        Tab::Timetable => html! { <TimetableComponent /> },
        Tab::Messages => html! { <MessagesComponent /> },
        Tab::Absences => html! { <AbsencesComponent /> },
        Tab::Settings => html! { <SettingsComponent /> },
    };

    html! {
        <>
            <div class="d-flex flex-column flex-md-row">
                <NavBar active_tab={(*active_tab).clone()} on_change={on_nav_change} />
                <main class="flex-grow-1 p-4 mb-5 mb-md-0">
                    {content}
                </main>
            </div>
        </>
    }
}

#[function_component(TimetableComponent)]
fn timetable() -> Html { html! { <div>{"Timetable View"}</div> } }

#[function_component(MessagesComponent)]
fn messages() -> Html { html! { <div>{"Messages View"}</div> } }

#[function_component(AbsencesComponent)]
fn absences() -> Html { html! { <div>{"Absences View"}</div> } }

#[function_component(SettingsComponent)]
fn settings() -> Html { html! { <div>{"Settings View"}</div> } }