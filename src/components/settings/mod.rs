mod auth_card;
mod card;
mod clear_button;
mod exam_reminders;
mod general_card;
mod lesson_sections;
mod notification_card;
mod share_button;
mod subject_colors;
mod visual_card;

use crate::untis::UntisClient;
use crate::{notification_poller, storage, VERSION};
use altis_core::settings::{AuthSettings, GeneralSettings, NotificationSettings, Settings, VisualSettings};
use auth_card::{AuthKind, AuthSettingsCard};
use clear_button::ClearSettingsButton;
use general_card::GeneralSettingsCard;
use notification_card::NotificationSettingsCard;
use share_button::ShareSettingsButton;
use visual_card::VisualSettingsCard;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SettingsPageProps {
    /// the settings were stored, for whoever goes by them outside this screen
    pub on_change: Callback<Settings>,
}

/// A callback that stores one part of the settings: the stored settings are read, the part is
/// put into them and they are written back, so a card never overwrites what another one stored
fn save_part<T: 'static>(
    shown: &UseStateHandle<Option<Settings>>,
    error_message: &UseStateHandle<Option<String>>,
    on_change: &Callback<Settings>,
    put: fn(&mut Settings, T),
) -> Callback<T> {
    let (shown, error_message, on_change) = (shown.clone(), error_message.clone(), on_change.clone());

    Callback::from(move |part: T| {
        let mut settings = match storage::settings() {
            Ok(stored) => stored.unwrap_or_default(),
            Err(err) => return error_message.set(Some(format!("Load failed: {err}"))),
        };
        put(&mut settings, part);

        match storage::save_settings(&settings) {
            Ok(()) => {
                on_change.emit(settings.clone());
                shown.set(Some(settings));
                error_message.set(None);
            }
            Err(err) => error_message.set(Some(format!("Save failed: {err}"))),
        }
    })
}

/// The settings screen: the logins, and a card per group of settings
#[function_component(SettingsPage)]
pub fn settings_page(props: &SettingsPageProps) -> Html {
    // none if the stored settings can't be read, which is told rather than papered over with defaults
    let shown = use_state(|| storage::settings().ok().map(Option::unwrap_or_default));
    let error_message = use_state(|| None::<String>);

    let on_untis_auth_save = {
        let save = save_part(&shown, &error_message, &props.on_change, |settings, auth| settings.untis_auth = auth);
        Callback::from(move |auth: AuthSettings| {
            save.emit(auth.clone());
            // logs in right away, so a typo shows up here rather than on the next screen
            spawn_local(async move {
                let _ = UntisClient::authenticate(&auth).await;
            });
        })
    };
    let on_b2e_auth_save = save_part(&shown, &error_message, &props.on_change, |settings, auth| settings.b2e_auth = auth);
    let on_general_save = save_part(&shown, &error_message, &props.on_change, |settings, general: GeneralSettings| settings.general = general);
    let on_visual_save = save_part(&shown, &error_message, &props.on_change, |settings, visual: VisualSettings| settings.visual_settings = visual);
    let on_notification_save = {
        let save = save_part(&shown, &error_message, &props.on_change, |settings, notifications| settings.notification_settings = notifications);
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
                        <GeneralSettingsCard initial={settings.general.clone()} on_save={on_general_save} />
                        <VisualSettingsCard initial={settings.visual_settings.clone()} on_save={on_visual_save} />
                        <NotificationSettingsCard initial={settings.notification_settings.clone()} on_save={on_notification_save} />
                    } else {
                        <div class="alert alert-warning">
                            {"Failed to parse settings. Your settings might be corrupted. "}
                        </div>
                    }

                    <p class="text-center text-secondary small mb-0">{ format!("Altis {VERSION}") }</p>
                </div>
            </div>
        </div>
    }
}
