use wasm_bindgen_futures::spawn_local;
use crate::components::absences::*;
use crate::components::auth_wrapper::AuthWrapper;
use crate::components::book2eat::*;
use crate::components::letto::*;
use crate::components::messages::*;
use crate::components::navbar::*;
use crate::components::settings::*;
use crate::components::timetable::*;
use yew::prelude::*;
use crate::persistence_manager::PersistenceManager;
use crate::untis_client::UntisClient;

#[function_component(App)]
pub fn app() -> Html {

    let active_tab = use_state(|| Tab::Timetable);

    let on_nav_change = {
        let active_tab = active_tab.clone();
        Callback::from(move |tab: Tab| active_tab.set(tab))
    };

    let content = match *active_tab {
        Tab::Timetable => html! { <TimetableComponent /> },
        Tab::Messages => html! { <AuthWrapper><MessagesComponent /></AuthWrapper> },
        Tab::Absences => html! { <AuthWrapper><AbsencesComponent /></AuthWrapper> },
        Tab::Settings => html! { <SettingsComponent /> },
        Tab::Letto => html! { <LettoComponent /> },
        Tab::Book2Eat => html! { <Book2EatComponent /> },
    };

    if let Some(s) = PersistenceManager::get_settings().ok() {
        spawn_local(async move {
            let _ = UntisClient::get_session_into_cookies(
                s.school_name,
                s.username,
                s.auth_secret,
            ).await;
        });
    }


    html! {
        <>
            <div class="d-flex flex-column flex-md-row">
                <NavBar active_tab={(*active_tab).clone()} on_change={on_nav_change} />
                <main class="flex-grow-1 mb-5 mb-md-0 bg-dark text-white">
                    {content}
                </main>
            </div>
        </>
    }
}