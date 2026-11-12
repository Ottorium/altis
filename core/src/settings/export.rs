use super::{AuthSettings, Favorites, NotificationSettings, Settings, VisualSettings};
use serde::{Deserialize, Serialize};

/// Settings as they are shared with other devices, the logins are only included when asked for
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct SettingsExport {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub untis_auth: Option<AuthSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub b2e_auth: Option<AuthSettings>,
    pub visual_settings: VisualSettings,
    #[serde(default)]
    pub notification_settings: NotificationSettings,
    /// absent from exports made before favourites existed, which then keep the ones already there
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favorites: Option<Favorites>,
}

impl SettingsExport {
    pub fn new(settings: &Settings, include_credentials: bool) -> Self {
        Self {
            untis_auth: include_credentials.then(|| settings.untis_auth.clone()),
            b2e_auth: include_credentials.then(|| settings.b2e_auth.clone()),
            visual_settings: settings.visual_settings.clone(),
            notification_settings: settings.notification_settings.clone(),
            favorites: Some(settings.favorites.clone()),
        }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text.trim()).map_err(|_| "This isn't an Altis settings export".to_string())
    }

    pub fn has_credentials(&self) -> bool {
        self.untis_auth.is_some() || self.b2e_auth.is_some()
    }

    /// Overwrites the given settings with the imported ones, logins that weren't exported are kept
    pub fn apply_to(self, settings: &mut Settings) {
        if let Some(auth) = self.untis_auth {
            settings.untis_auth = auth;
        }
        if let Some(auth) = self.b2e_auth {
            settings.b2e_auth = auth;
        }
        settings.visual_settings = self.visual_settings;
        settings.notification_settings = self.notification_settings;
        if let Some(favorites) = self.favorites {
            settings.favorites = favorites;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::timetable::EntityKind;

    /// an export from before favourites existed must not wipe the ones on the importing device
    #[test]
    fn an_old_export_keeps_the_favourites() {
        let mut settings = Settings::default();
        settings.favorites.toggle(EntityKind::Class, "4AHIT");

        let old_export = r#"{"visual_settings": {}, "notification_settings": {}}"#;
        SettingsExport::parse(old_export).unwrap().apply_to(&mut settings);
        assert!(settings.favorites.contains(EntityKind::Class, "4AHIT"));

        let mut other = Settings::default();
        SettingsExport::new(&settings, false).apply_to(&mut other);
        assert!(other.favorites.contains(EntityKind::Class, "4AHIT"));
    }

    #[test]
    fn the_logins_only_travel_when_asked_for() {
        let mut settings = Settings::default();
        settings.untis_auth.secret = "secret".to_string();

        let without = SettingsExport::new(&settings, false);
        assert!(!without.has_credentials());
        assert!(!serde_json::to_string(&without).unwrap().contains("secret"));

        let mut other = Settings::default();
        other.untis_auth.secret = "kept".to_string();
        without.apply_to(&mut other);
        assert_eq!(other.untis_auth.secret, "kept");

        SettingsExport::new(&settings, true).apply_to(&mut other);
        assert_eq!(other.untis_auth.secret, "secret");
    }

    #[test]
    fn rejects_what_is_no_export() {
        assert!(SettingsExport::parse("https://example.com").is_err());
        assert!(SettingsExport::parse("  {\"visual_settings\": {}}\n").is_ok());
    }
}
