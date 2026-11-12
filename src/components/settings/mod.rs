mod auth_card;
mod card;
mod clear_button;
mod exam_reminders;
mod notification_card;
mod share_button;
mod subject_colors;
mod visual_card;

use crate::untis::UntisClient;
use crate::{notification_poller, storage};
use altis_core::settings::{AuthSettings, NotificationSettings, Settings, VisualSettings};
use auth_card::{AuthKind, AuthSettingsCard};
use clear_button::ClearSettingsButton;
use notification_card::NotificationSettingsCard;
use share_button::ShareSettingsButton;
use visual_card::VisualSettingsCard;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

/// A callback that stores one part of the settings: the stored settings are read, the part is
/// put into them and they are written back, so a card never overwrites what another one stored
fn save_part<T: 'static>(
    shown: &UseStateHandle<Option<Settings>>,
    error_message: &UseStateHandle<Option<String>>,
    put: fn(&mut Settings, T),
) -> Callback<T> {
    let (shown, error_message) = (shown.clone(), error_message.clone());

    Callback::from(move |part: T| {
        let mut settings = match storage::settings() {
            Ok(stored) => stored.unwrap_or_default(),
            Err(err) => return error_message.set(Some(format!("Load failed: {err}"))),
        };
        put(&mut settings, part);

        match storage::save_settings(&settings) {
            Ok(()) => {
                shown.set(Some(settings));
                error_message.set(None);
            }
            Err(err) => error_message.set(Some(format!("Save failed: {err}"))),
        }
    })
}

/// The settings screen: the logins, and a card per group of settings
#[function_component(SettingsPage)]
pub fn settings_page() -> Html {
    // none if the stored settings can't be read, which is told rather than papered over with defaults
    let shown = use_state(|| storage::settings().ok().map(Option::unwrap_or_default));
    let error_message = use_state(|| None::<String>);

    let on_untis_auth_save = {
        let save = save_part(&shown, &error_message, |settings, auth| settings.untis_auth = auth);
        Callback::from(move |auth: AuthSettings| {
            save.emit(auth.clone());
            // logs in right away, so a typo shows up here rather than on the next screen
            spawn_local(async move {
                let _ = UntisClient::authenticate(&auth).await;
            });
        })
    };
    let on_b2e_auth_save = save_part(&shown, &error_message, |settings, auth| settings.b2e_auth = auth);
    let on_visual_save = save_part(&shown, &error_message, |settings, visual: VisualSettings| settings.visual_settings = visual);
    let on_notification_save = {
        let save = save_part(&shown, &error_message, |settings, notifications| settings.notification_settings = notifications);
        Callback::from(move |notifications: NotificationSettings| {
            save.emit(notifications);
            // starts or stops Android's background service to match the new settings
            spawn_local(notification_poller::apply_settings());
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

                    if let Some(message) = &*error_message {
                        <div class="alert alert-danger">{ message }</div>
                    }

                    if let Some(settings) = &*shown {
                        <AuthSettingsCard kind={AuthKind::Untis} initial={settings.untis_auth.clone()} on_save={on_untis_auth_save} />
                        <AuthSettingsCard kind={AuthKind::Book2Eat} initial={settings.b2e_auth.clone()} on_save={on_b2e_auth_save} />
                        <VisualSettingsCard initial={settings.visual_settings.clone()} on_save={on_visual_save} />
                        <NotificationSettingsCard initial={settings.notification_settings.clone()} on_save={on_notification_save} />
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
