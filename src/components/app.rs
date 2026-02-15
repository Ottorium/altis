use crate::authorization_untis_client;
use crate::components::absences::*;
use crate::components::auth_wrapper::AuthWrapper;
use crate::components::book2eat::*;
use crate::components::letto::*;
use crate::components::loading::*;
use crate::components::messages::*;
use crate::components::navbar::*;
use crate::components::settings::settings::*;
use crate::components::timetable::timetable::*;
use crate::persistence_manager::PersistenceManager;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[function_component(App)]
pub fn app() -> Html {

    let active_tab = use_state(|| Tab::Timetable);

    let on_nav_change = {
        let active_tab = active_tab.clone();
        Callback::from(move |tab: Tab| active_tab.set(tab))
    };

    let content = match *active_tab {
        Tab::Timetable => html! { <AuthWrapper><TimetableComponent /></AuthWrapper> },
        Tab::Messages => html! { <AuthWrapper><MessagesComponent /></AuthWrapper> },
        Tab::Absences => html! { <AuthWrapper><AbsencesComponent /></AuthWrapper> },
        Tab::Settings => html! { <SettingsComponent /> },
        Tab::Letto => html! { <LettoComponent /> },
        Tab::Book2Eat => html! { <Book2EatComponent /> },
    };

    if let Some(s) = PersistenceManager::get_settings().ok() && let Some(s) = s {
        spawn_local(async move {
            let _ = authorization_untis_client::get_session_into_cookies(
                s.auth_settings.school_name,
                s.auth_settings.username,
                s.auth_settings.auth_secret,
            ).await;
        });
    }


    html! {
        <>
            <div class="d-flex flex-column flex-md-row vh-100 overflow-hidden bg-dark text-white">
                <NavBar active_tab={(*active_tab).clone()} on_change={on_nav_change} />
                <main class="d-flex flex-column flex-grow-1 overflow-y-auto">
                    <Suspense fallback={html! { <LoadingComponent /> }}>
                        {content}
                    </Suspense>
                </main>
            </div>
        </>
    }
}