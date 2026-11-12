use crate::components::absences::AbsencesPage;
use crate::components::auth_wrapper::AuthWrapper;
use crate::components::book2eat::Book2EatPage;
use crate::components::loading::Loading;
use crate::components::messages::MessagesPage;
use crate::components::navbar::{NavBar, Tab};
use crate::components::settings::SettingsPage;
use crate::components::timetable::TimetablePage;
use crate::components::update_banner::UpdateBanner;
use crate::untis::UntisClient;
use crate::{notification_poller, storage};
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
        Tab::Timetable => html! { <AuthWrapper><TimetablePage /></AuthWrapper> },
        Tab::Messages => html! { <AuthWrapper><MessagesPage /></AuthWrapper> },
        Tab::Absences => html! { <AuthWrapper><AbsencesPage /></AuthWrapper> },
        Tab::Settings => html! { <SettingsPage /> },
        Tab::Book2Eat => html! { <Book2EatPage /> },
    };

    if let Ok(Some(settings)) = storage::settings() {
        spawn_local(async move {
            let _ = UntisClient::authenticate(&settings.untis_auth).await;
        });
    }

    // starts once and keeps polling for as long as the app is open
    use_effect_with((), |_| {
        notification_poller::start();
        || ()
    });

    html! {
        <>
            <div class="d-flex flex-column flex-md-row vh-100 overflow-hidden bg-dark text-white">
                <NavBar active_tab={*active_tab} on_change={on_nav_change} />
                <main class="d-flex flex-column flex-grow-1 overflow-y-auto">
                    <UpdateBanner />
                    <Suspense fallback={html! { <Loading /> }}>
                        {content}
                    </Suspense>
                </main>
            </div>
        </>
    }
}
