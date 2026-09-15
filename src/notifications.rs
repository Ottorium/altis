//! Drives the notification poller from the frontend.
//!
//! The poll itself lives in `altis_core::notifications`; this is only about who runs it and how
//! often. On desktop that is this loop, running in the webview for as long as the app's process
//! is alive (see `setup_tray` in the Tauri backend for how closing the window only hides it). On
//! Android the webview is gone the moment the app is closed, so the poll runs natively instead —
//! either as a WorkManager job or in a foreground service, see `BackgroundMode` — and this loop
//! stands down so the two don't notify about the same change twice.

use crate::native;
use crate::persistence_manager::{PersistenceManager, WebEnv};
use altis_core::notifications::run_once;
use altis_core::settings::{BackgroundMode, Settings};
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_futures::spawn_local;

fn settings() -> Settings {
    PersistenceManager::get_settings().ok().flatten().unwrap_or_default()
}

/// The mode Android should actually be in: turning notifications off altogether has to stop the
/// background polling too, whatever mode is selected under it
fn effective_mode(settings: &Settings) -> BackgroundMode {
    if settings.notification_settings.enabled {
        settings.notification_settings.background_mode
    } else {
        BackgroundMode::Off
    }
}

/// Whether Android is doing the polling right now, and this loop should therefore stay out of it
fn polls_natively(settings: &Settings) -> bool {
    native::is_android() && effective_mode(settings).is_background()
}

/// Hands the current settings to the native side and puts Android into the matching background
/// mode. Called whenever the notification settings change.
pub async fn apply_settings() {
    if !native::is_android() {
        return;
    }

    // synced before anything is told to start, so the first poll already has credentials
    let _ = native::sync_store(&PersistenceManager::background_store_entries()).await;
    let _ = native::set_background_polling(effective_mode(&settings()).as_str()).await;
}

/// Starts the poller. Re-reads the settings on every cycle, so changes made in the Settings
/// screen (including the poll interval itself) take effect on the next poll.
pub fn start() {
    spawn_local(async {
        // ask for the permission up front, so the first real notification isn't silently dropped
        let _ = native::ensure_notification_permission().await;
        apply_settings().await;

        loop {
            let settings = settings();

            if polls_natively(&settings) {
                // keep the native store's copy of the settings fresh while the app is open, so
                // the background poller is never working from a stale login
                PersistenceManager::sync_to_background();
            } else {
                run_once::<WebEnv>().await;
            }

            let minutes = settings.notification_settings.poll_interval_minutes.max(1);
            TimeoutFuture::new(minutes * 60 * 1000).await;
        }
    });
}
