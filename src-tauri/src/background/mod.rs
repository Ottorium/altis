//! Polling for timetable changes while the app is closed.
//!
//! On desktop the poller runs in the webview and the app simply stays alive in the tray. Android
//! gives no such option: the webview is gone as soon as the app is closed, so the same poll
//! (`altis_core::notifications`) runs natively here instead, in a thread owned by a foreground
//! service that keeps the process around. Everything below is a no-op on other platforms.

use std::collections::HashMap;

#[cfg(target_os = "android")]
mod bridge;
#[cfg(target_os = "android")]
mod env;
#[cfg(target_os = "android")]
mod store;

/// Takes the frontend's copy of the settings and the Untis session, so the poller can keep
/// working without a webview to read them from
#[cfg(target_os = "android")]
pub fn sync_store(entries: HashMap<String, String>) -> Result<(), String> {
    store::sync(&altis_core::store::SHARED_KEYS, entries)
}

#[cfg(not(target_os = "android"))]
pub fn sync_store(_entries: HashMap<String, String>) -> Result<(), String> {
    Ok(())
}

/// Switches Android background polling between "off", "periodic" (a WorkManager job) and
/// "continuous" (the foreground service), returning whether polling now happens outside the
/// webview. Always `false` off Android, where the frontend keeps doing it itself.
#[cfg(target_os = "android")]
pub fn set_mode(mode: &str) -> Result<bool, String> {
    bridge::set_background_mode(mode)?;
    Ok(mode != altis_core::settings::BackgroundMode::Off.as_str())
}

#[cfg(not(target_os = "android"))]
pub fn set_mode(_mode: &str) -> Result<bool, String> {
    Ok(false)
}

/// How often the stop flag is checked while waiting for the next poll. Long enough not to matter
/// for battery, short enough that turning the service off doesn't appear to hang.
#[cfg(target_os = "android")]
const STOP_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

/// Runs one poll on the calling thread, for the WorkManager job
#[cfg(target_os = "android")]
pub(crate) fn run_once_blocking() {
    block_on(altis_core::notifications::run_once::<env::NativeEnv>());
}

#[cfg(target_os = "android")]
fn block_on(future: impl std::future::Future<Output = ()>) {
    // a current-thread runtime, because the poll's futures are deliberately not `Send`: the same
    // code runs single-threaded in the frontend's WASM
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread().enable_all().build() else { return };
    runtime.block_on(future);
}

/// The foreground service's loop, for when polling has to be more frequent than WorkManager's
/// 15-minute floor allows
#[cfg(target_os = "android")]
fn poll_loop() {
    use altis_core::store::Store;
    use env::NativeEnv;

    block_on(async {
        while bridge::should_run() {
            altis_core::notifications::run_once::<NativeEnv>().await;

            let minutes = Store::<NativeEnv>::settings_or_default()
                .notification_settings
                .poll_interval_minutes
                .max(1);

            let until = std::time::Instant::now() + std::time::Duration::from_secs(u64::from(minutes) * 60);
            while bridge::should_run() && std::time::Instant::now() < until {
                tokio::time::sleep(STOP_CHECK_INTERVAL).await;
            }
        }
    });
}
