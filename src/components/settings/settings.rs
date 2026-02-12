use crate::authorization_untis_client;
use crate::components::settings::auth_settings_card::AuthSettingsCard;
use crate::components::settings::clear_settings_button::ClearSettingsButton;
use crate::persistence_manager::*;
use yew::prelude::*;
use crate::components::settings::visual_settings::VisualSettingsCard;

#[function_component(SettingsComponent)]
pub fn settings() -> Html {
    let settings_state = use_state(|| PersistenceManager::get_settings().ok().map(|inner| inner.unwrap_or(Settings::default())));
    let error_message = use_state(|| None::<String>);

    let update_settings = {
        let error_message = error_message.clone();
        let settings_state = settings_state.clone();

        Callback::from(move |updater: Box<dyn FnOnce(&mut Settings)>| {
            let current_settings = PersistenceManager::get_settings();

            match current_settings {
                Ok(maybe_s) => {
                    let mut settings = maybe_s.unwrap_or_default();
                    updater(&mut settings);

                    if let Err(err) = PersistenceManager::save_settings(&settings) {
                        error_message.set(Some(format!("Save failed: {}", err)));
                    } else {
                        settings_state.set(Some(settings.clone()));
                        error_message.set(None);
                    }
                }
                Err(err) => error_message.set(Some(format!("Load failed: {}", err))),
            }
        })
    };

    let on_auth_save = {
        let update_settings = update_settings.clone();
        Callback::from(move |new_auth: AuthSettings| {
            let auth_clone = new_auth.clone();
            update_settings.emit(Box::new(move |s| s.auth_settings = new_auth));

            wasm_bindgen_futures::spawn_local(async move {
                let _ = authorization_untis_client::get_session_into_cookies(
                    auth_clone.school_name,
                    auth_clone.username,
                    auth_clone.auth_secret,
                )
                    .await;
            });
        })
    };

    let on_visual_save = {
        let update_settings = update_settings.clone();
        Callback::from(move |new_visual: VisualSettings| {
            update_settings.emit(Box::new(move |s| s.visual_settings = new_visual));
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

                        <VisualSettingsCard
                            initial={settings.clone().visual_settings}
                            on_save={on_visual_save}
                        />

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