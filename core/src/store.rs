use crate::env::Env;
use crate::settings::{Cookies, NotificationState, Settings};
use std::marker::PhantomData;

pub const SETTINGS_KEY: &str = "user_settings";
pub const JSESSIONID_KEY: &str = "JSESSIONID";
pub const TENANT_ID_KEY: &str = "Tenant-Id";
pub const SCHOOLNAME_KEY: &str = "schoolname";
pub const NOTIFICATION_STATE_KEY: &str = "notification_state";

/// The keys the Android background poller needs a copy of to do its job on its own. The frontend
/// mirrors exactly these into the native store (see `native::sync_store`); everything else stays
/// in whichever store wrote it.
///
/// Only the settings, deliberately: the session cookies are left out so the poller keeps its own
/// Untis session rather than racing the webview to overwrite a shared one. It can always log in
/// by itself, since the credentials it would need to do that are the very thing being synced.
pub const SHARED_KEYS: [&str; 1] = [SETTINGS_KEY];

/// The stored state the Untis client and the poller need, on top of whatever key-value store the
/// platform provides. The frontend wraps this in `PersistenceManager`, which adds its own
/// (browser-only, compressed) timetable caches on top.
pub struct Store<E: Env>(PhantomData<E>);

impl<E: Env> Store<E> {
    pub fn get_cookies() -> Option<Cookies> {
        let env = E::default();
        Some(Cookies {
            jsessionid: env.get(JSESSIONID_KEY)?,
            tenant_id: env.get(TENANT_ID_KEY)?,
            school_name_base32: env.get(SCHOOLNAME_KEY)?,
        })
    }

    pub fn save_cookies(cookies: &Cookies) -> Result<(), String> {
        let env = E::default();
        env.set(JSESSIONID_KEY, &cookies.jsessionid)?;
        env.set(TENANT_ID_KEY, &cookies.tenant_id)?;
        env.set(SCHOOLNAME_KEY, &cookies.school_name_base32)
    }

    pub fn clear_cookies() {
        let env = E::default();
        for key in [JSESSIONID_KEY, TENANT_ID_KEY, SCHOOLNAME_KEY] {
            env.remove(key);
        }
    }

    pub fn get_settings() -> Result<Option<Settings>, String> {
        match E::default().get(SETTINGS_KEY) {
            Some(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(|e| format!("Failed to parse settings: {e}")),
            None => Ok(None),
        }
    }

    pub fn save_settings(settings: &Settings) -> Result<(), String> {
        let serialized = serde_json::to_string(settings).map_err(|e| format!("Serialization failed: {e}"))?;
        E::default().set(SETTINGS_KEY, &serialized)
    }

    /// The settings as the poller wants them: missing or unparseable settings are not worth an
    /// error to anyone who only wants to know whether to send a notification
    pub fn settings_or_default() -> Settings {
        Self::get_settings().ok().flatten().unwrap_or_default()
    }

    pub fn get_notification_state() -> Result<Option<NotificationState>, String> {
        match E::default().get(NOTIFICATION_STATE_KEY) {
            Some(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(|e| format!("Failed to parse notification state: {e}")),
            None => Ok(None),
        }
    }

    pub fn save_notification_state(state: &NotificationState) -> Result<(), String> {
        let serialized = serde_json::to_string(state).map_err(|e| format!("Serialization failed: {e}"))?;
        E::default().set(NOTIFICATION_STATE_KEY, &serialized)
    }
}
