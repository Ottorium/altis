use crate::authorization_untis_client;
use crate::components::settings::auth_settings_card::AuthSettingsCard;
use crate::components::settings::clear_settings_button::ClearSettingsButton;
use crate::persistence_manager::*;
use yew::prelude::*;

#[function_component(SettingsComponent)]
pub fn settings() -> Html {
    let settings_state = use_state(|| PersistenceManager::get_settings().ok().map(|inner| inner.unwrap_or(Settings::default())));
    let error_message = use_state(|| None::<String>);

    let on_auth_save = {
        let error_message = error_message.clone();
        let settings_state = settings_state.clone();
        Callback::from(move |new_auth_settings: AuthSettings| {
            let new_settings = Settings { auth_settings: new_auth_settings };
            if let Err(err) = PersistenceManager::save_settings(&new_settings) {
                error_message.set(Some(format!("Save failed: {}", err)));
            } else {
                settings_state.set(Some(new_settings.clone()));
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = authorization_untis_client::get_session_into_cookies(
                        new_settings.auth_settings.school_name,
                        new_settings.auth_settings.username,
                        new_settings.auth_settings.auth_secret,
                    ).await;
                });
            }
        })
    };

    html! {
        <div class="container py-5" data-bs-theme="dark">
            <div class="row justify-content-center">
                <div class="col-12 col-md-8 col-lg-6">
                    <div class="d-flex justify-content-between align-items-center mb-4">
                        <h2 class="text-white mb-0">{"Settings"}</h2>
                        <ClearSettingsButton />
                    </div>

                    {if let Some(msg) = &*error_message {
                        html! { <div class="alert alert-danger">{msg}</div> }
                    } else { html!{} }}

                    if let Some(settings) = &*settings_state {
                        <AuthSettingsCard
                            initial={settings.clone().auth_settings}
                            on_save={on_auth_save}
                        />

                        // future settings

                    } else {
                        <div class="alert alert-warning">
                            {"Failed to parse settings. Your settings might be corrupted. "}
                        </div>
                    }
                </div>
            </div>
        </div>
    }
}