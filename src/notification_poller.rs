//! Drives the notification poller from the frontend.
//!
//! The poll itself lives in `altis_core::notifications`; this is only about who runs it and how
//! often. On desktop that is this loop, running in the webview for as long as the app's process
//! is alive (see `setup_tray` in the Tauri backend for how closing the window only hides it). On
//! Android the webview is gone the moment the app is closed, so the poll runs natively instead —
//! either as a WorkManager job or in a foreground service, see `BackgroundMode` — and this loop
//! stands down so the two don't notify about the same change twice.

use crate::web_env::WebEnv;
use crate::{native, storage};
use altis_core::notifications::run_once;
use altis_core::settings::NotificationSettings;
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_futures::spawn_local;

const MS_PER_MINUTE: u32 = 60 * 1000;

/// Whether Android is doing the polling right now, and this loop should therefore stay out of it
fn polls_natively(settings: &NotificationSettings) -> bool {
    native::is_android() && settings.effective_background_mode().is_background()
}

/// Hands the current settings to the native side and puts Android into the matching background
/// mode. Called whenever the notification settings change.
pub async fn apply_settings() {
    if !native::is_android() {
        return;
    }

    // synced before anything is told to start, so the first poll already has credentials
    let _ = native::sync_store(&storage::background_store_entries()).await;
    let mode = storage::settings_or_default().notification_settings.effective_background_mode();
    let _ = native::set_background_polling(mode.as_str()).await;
}

/// Starts the poller. Re-reads the settings on every cycle, so changes made in the Settings
/// screen (including the poll interval itself) take effect on the next poll.
pub fn start() {
    spawn_local(async {
        // ask for the permission up front, so the first real notification isn't silently dropped
        let _ = native::ensure_notification_permission().await;
        apply_settings().await;

        loop {
            let settings = storage::settings_or_default().notification_settings;

            if polls_natively(&settings) {
                // keep the native store's copy of the settings fresh while the app is open, so
                // the background poller is never working from a stale login
                storage::sync_to_background();
            } else {
                run_once::<WebEnv>().await;
            }

            TimeoutFuture::new(settings.poll_minutes() * MS_PER_MINUTE).await;
        }
    });
}
