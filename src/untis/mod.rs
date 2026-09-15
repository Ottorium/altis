pub mod cached_untis_client;
mod teacher_table_generator;

use crate::persistence_manager::WebEnv;

/// The shared Untis client, bound to the browser: localStorage for storage, the Tauri `proxy`
/// command for requests, the notification plugin for notifications. The Android background poller
/// binds the same client to its own platform instead, see `altis_core::env::Env`.
pub type UntisClient = altis_core::untis::untis_client::UntisClient<WebEnv>;
