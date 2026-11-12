//! Everything the user can set. Stored as one JSON document, so every field added later needs a
//! default: the settings of an install that predates it have to keep loading.

mod export;
mod favorites;
mod notifications;
mod visual;

pub use export::SettingsExport;
pub use favorites::Favorites;
pub use notifications::*;
pub use visual::*;

use serde::{Deserialize, Serialize};

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub untis_auth: AuthSettings,
    pub b2e_auth: AuthSettings,
    pub visual_settings: VisualSettings,
    #[serde(default)]
    pub notification_settings: NotificationSettings,
    #[serde(default)]
    pub favorites: Favorites,
}

/// A login: for Untis the school, the user and the secret Untis hands out for other apps, for
/// the canteen its id, the mail address and the password
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AuthSettings {
    pub school_identifier: String,
    pub user_identifier: String,
    pub secret: String,
}

impl AuthSettings {
    pub fn is_complete(&self) -> bool {
        !self.school_identifier.is_empty() && !self.user_identifier.is_empty() && !self.secret.is_empty()
    }
}

fn default_true() -> bool {
    true
}
