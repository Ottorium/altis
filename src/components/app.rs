use crate::components::absences::AbsencesPage;
use crate::components::auth_wrapper::AuthWrapper;
use crate::components::book2eat::Book2EatPage;
use crate::components::loading::Loading;
use crate::components::messages::MessagesPage;
use crate::components::navbar::NavBar;
use crate::components::settings::SettingsPage;
use crate::components::timetable::TimetablePage;
use crate::components::update_banner::UpdateBanner;
use crate::untis::UntisClient;
use crate::{notification_poller, storage};
use altis_core::settings::{Settings, Tab};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[function_component(App)]
pub fn app() -> Html {
    // the part of the settings the frame around the screens goes by: its tabs, its banner
    let general = use_state_eq(|| storage::settings_or_default().general);
    let active_tab = use_state(|| general.initial_tab());

    let on_nav_change = {
        let active_tab = active_tab.clone();
        Callback::from(move |tab: Tab| active_tab.set(tab))
    };

    let on_settings_change = {
        let general = general.clone();
        Callback::from(move |settings: Settings| general.set(settings.general))
    };

    let content = match *active_tab {
        Tab::Timetable => html! { <AuthWrapper><TimetablePage /></AuthWrapper> },
        Tab::Messages => html! { <AuthWrapper><MessagesPage /></AuthWrapper> },
        Tab::Absences => html! { <AuthWrapper><AbsencesPage /></AuthWrapper> },
        Tab::Settings => html! { <SettingsPage on_change={on_settings_change} /> },
        Tab::Book2Eat => html! { <Book2EatPage /> },
    };

    // once, as the app starts: logs in and starts the poller, which keeps going for as long as
    // the app is open. A session that runs out later is renewed by the request that finds out.
    use_effect_with((), |_| {
        if let Ok(Some(settings)) = storage::settings() {
            spawn_local(async move {
                let _ = UntisClient::authenticate(&settings.untis_auth).await;
            });
        }
        notification_poller::start();
        || ()
    });

    html! {
        <>
            <div class="d-flex flex-column flex-md-row vh-100 overflow-hidden bg-dark text-white">
                <NavBar tabs={general.shown_tabs()} active_tab={*active_tab} on_change={on_nav_change} />
                <main class="d-flex flex-column flex-grow-1 overflow-y-auto">
                    <UpdateBanner enabled={general.check_for_updates} />
                    <Suspense fallback={html! { <Loading /> }}>
                        {content}
                    </Suspense>
                </main>
            </div>
        </>
    }
}
