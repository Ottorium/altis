
mod components;
mod request_proxy;
mod authorization_untis_client;
mod persistence_manager;
mod untis_client;
mod untis_week;
mod data_models;
mod teacher_table_generator;
mod errors;

use components::app::App;

fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}
