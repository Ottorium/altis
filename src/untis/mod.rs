pub mod cached_client;
mod entity_timetables;

use crate::web_env::WebEnv;

/// The shared Untis client, bound to the browser: localStorage for storage, the Tauri `proxy`
/// command for requests, the notification plugin for notifications. The Android background poller
/// binds the same client to its own platform instead, see `altis_core::env::Env`.
pub type UntisClient = altis_core::untis::client::UntisClient<WebEnv>;
