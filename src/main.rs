mod components;
mod request_proxy;
mod persistence_manager;
mod untis;
mod book2eat;
mod native;
mod notifications;

use components::app::App;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
