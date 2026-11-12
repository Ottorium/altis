mod book2eat_client;
mod components;
mod hooks;
mod native;
mod notification_poller;
mod storage;
mod untis;
mod web_env;

use components::app::App;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
