use crate::components::settings::auth_settings_card::{AuthSettingsCard, AuthType};
use crate::components::settings::clear_settings_button::ClearSettingsButton;
use crate::components::settings::notification_settings::NotificationSettingsCard;
use crate::components::settings::share_settings_button::ShareSettingsButton;
use crate::components::settings::visual_settings::VisualSettingsCard;
use crate::persistence_manager::PersistenceManager;
use crate::untis::UntisClient;
use altis_core::settings::{AuthSettings, NotificationSettings, Settings, VisualSettings};
use yew::prelude::*;

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
            update_settings.emit(Box::new(move |s| s.untis_auth = new_auth));

            wasm_bindgen_futures::spawn_local(async move {
                let _ = UntisClient::authenticate(
                    auth_clone.school_identifier,
                    auth_clone.user_identifier,
                    auth_clone.secret,
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

    let b2e_save = {
        let update_settings = update_settings.clone();
        Callback::from(move |new_auth: AuthSettings| {
            update_settings.emit(Box::new(move |s| s.b2e_auth = new_auth));
        })
    };

    let on_notification_save = {
        let update_settings = update_settings.clone();
        Callback::from(move |new_notification: NotificationSettings| {
            update_settings.emit(Box::new(move |s| s.notification_settings = new_notification));
            // starts or stops Android's background service to match the new settings
            wasm_bindgen_futures::spawn_local(crate::notifications::apply_settings());
        })
    };

    html! {
        <div class="container py-5" data-bs-theme="dark">
            <div class="row justify-content-center">
                <div class="col-12 col-md-8 col-lg-6">
                    <div class="d-flex justify-content-between align-items-center mb-4">
                        <h2 class="text-white mb-0">{"Settings"}</h2>
                        <div class="d-flex gap-2">
                            <ShareSettingsButton />
                            <ClearSettingsButton />
                        </div>
                    </div>

                    {if let Some(msg) = &*error_message {
                        html! { <div class="alert alert-danger">{msg}</div> }
                    } else { html!{} }}

                    if let Some(settings) = &*settings_state {
                        <AuthSettingsCard
                            r#type={AuthType::Untis}
                            initial={settings.clone().untis_auth}
                            on_save={on_auth_save}
                        />

                        <AuthSettingsCard
                            r#type={AuthType::Book2Eat}
                            initial={settings.clone().b2e_auth}
                            on_save={b2e_save}
                        />

                        <VisualSettingsCard
                            initial={settings.clone().visual_settings}
                            on_save={on_visual_save}
                        />

                        <NotificationSettingsCard
                            initial={settings.clone().notification_settings}
                            on_save={on_notification_save}
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