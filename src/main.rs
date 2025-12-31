
mod components;
mod request_proxy;
mod authorization_untis_client;
mod persistence_manager;
mod untis_client;
mod untis_week;
mod data_models;

use components::app::App;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
