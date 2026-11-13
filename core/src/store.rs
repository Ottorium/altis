use crate::env::Env;
use crate::notifications::NotificationState;
use crate::settings::Settings;
use crate::untis::Cookies;
use crate::update_check::UpdateState;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

pub const SETTINGS_KEY: &str = "user_settings";
pub const JSESSIONID_KEY: &str = "JSESSIONID";
pub const TENANT_ID_KEY: &str = "Tenant-Id";
pub const SCHOOLNAME_KEY: &str = "schoolname";
pub const TOKEN_KEY: &str = "untis_token";
pub const NOTIFICATION_STATE_KEY: &str = "notification_state";
pub const UPDATE_STATE_KEY: &str = "update_state";

/// The keys the Android background poller needs a copy of to do its job on its own. The frontend
/// mirrors exactly these into the native store (see `native::sync_store`); everything else stays
/// in whichever store wrote it.
///
/// Only the settings, deliberately: the session cookies are left out so the poller keeps its own
/// Untis session rather than racing the webview to overwrite a shared one. It can always log in
/// by itself, since the credentials it would need to do that are the very thing being synced.
pub const SHARED_KEYS: [&str; 1] = [SETTINGS_KEY];

/// The stored state the Untis client and the poller need, on top of whatever key-value store the
/// platform provides. The frontend's `storage` module adds its own (browser-only, compressed)
/// timetable caches on top.
pub struct Store<E: Env>(PhantomData<E>);

impl<E: Env> Store<E> {
    /// Reads a value stored as JSON. `what` names it in the error for one that doesn't parse.
    fn read_json<T: DeserializeOwned>(key: &str, what: &str) -> Result<Option<T>, String> {
        match E::default().get(key) {
            Some(raw) => serde_json::from_str(&raw).map(Some).map_err(|e| format!("Failed to parse {what}: {e}")),
            None => Ok(None),
        }
    }

    fn write_json<T: Serialize>(key: &str, value: &T) -> Result<(), String> {
        let serialized = serde_json::to_string(value).map_err(|e| format!("Serialization failed: {e}"))?;
        E::default().set(key, &serialized)
    }

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
        env.set(SCHOOLNAME_KEY, &cookies.school_name_base32)?;
        // a token handed out for the previous session isn't one we want to keep using
        env.remove(TOKEN_KEY);
        Ok(())
    }

    pub fn clear_cookies() {
        let env = E::default();
        for key in [JSESSIONID_KEY, TENANT_ID_KEY, SCHOOLNAME_KEY, TOKEN_KEY] {
            env.remove(key);
        }
    }

    pub fn get_token() -> Option<String> {
        E::default().get(TOKEN_KEY)
    }

    pub fn save_token(token: &str) -> Result<(), String> {
        E::default().set(TOKEN_KEY, token)
    }

    pub fn clear_token() {
        E::default().remove(TOKEN_KEY);
    }

    pub fn get_settings() -> Result<Option<Settings>, String> {
        Self::read_json(SETTINGS_KEY, "settings")
    }

    pub fn save_settings(settings: &Settings) -> Result<(), String> {
        Self::write_json(SETTINGS_KEY, settings)
    }

    /// The settings for whoever has no use for an error: missing or unparseable ones are not
    /// worth one to a poller that only wants to know whether to send a notification
    pub fn settings_or_default() -> Settings {
        Self::get_settings().ok().flatten().unwrap_or_default()
    }

    /// What the notification poller remembers, starting over if there is nothing readable
    pub fn notification_state_or_default() -> NotificationState {
        Self::read_json(NOTIFICATION_STATE_KEY, "notification state").ok().flatten().unwrap_or_default()
    }

    pub fn save_notification_state(state: &NotificationState) -> Result<(), String> {
        Self::write_json(NOTIFICATION_STATE_KEY, state)
    }

    /// What the update check remembers, starting over if there is nothing readable
    pub fn update_state_or_default() -> UpdateState {
        Self::read_json(UPDATE_STATE_KEY, "update state").ok().flatten().unwrap_or_default()
    }

    pub fn save_update_state(state: &UpdateState) -> Result<(), String> {
        Self::write_json(UPDATE_STATE_KEY, state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_env::TestEnv;

    type Store = super::Store<TestEnv>;

    #[test]
    fn settings_come_back_the_way_they_were_saved() {
        TestEnv::reset();
        assert_eq!(Store::get_settings(), Ok(None));

        let mut settings = Settings::default();
        settings.untis_auth.school_identifier = "htl".to_string();
        Store::save_settings(&settings).unwrap();
        assert_eq!(Store::get_settings(), Ok(Some(settings)));
    }

    #[test]
    fn broken_settings_are_an_error_to_the_screen_and_defaults_to_the_poller() {
        TestEnv::reset();
        TestEnv.set(SETTINGS_KEY, "{not json").unwrap();

        assert!(Store::get_settings().unwrap_err().starts_with("Failed to parse settings: "));
        assert_eq!(Store::settings_or_default(), Settings::default());
    }

    #[test]
    fn a_new_session_drops_the_token_of_the_old_one() {
        TestEnv::reset();
        Store::save_token("old").unwrap();
        assert_eq!(Store::get_cookies(), None);

        let cookies = Cookies { jsessionid: "A".to_string(), tenant_id: "1".to_string(), school_name_base32: "_x".to_string() };
        Store::save_cookies(&cookies).unwrap();
        assert_eq!(Store::get_cookies(), Some(cookies));
        assert_eq!(Store::get_token(), None);

        Store::clear_cookies();
        assert_eq!(Store::get_cookies(), None);
    }
}
