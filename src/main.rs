mod book2eat_client;
mod components;
mod hooks;
mod native;
mod notification_poller;
mod storage;
mod untis;
mod web_env;

use components::app::App;

/// What this build was compiled as, which is what the release tags are compared against. The
/// release script keeps it in step with `tauri.conf.json` and the published tag.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
