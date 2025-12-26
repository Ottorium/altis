use yew::prelude::*;
use web_sys::HtmlInputElement;
#[function_component(App)]
pub fn app() -> Html {
    let name = use_state(|| "".to_string());
    let input_ref = use_node_ref();

    let onsubmit = {
        let name = name.clone();
        let input_ref = input_ref.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            if let Some(input) = input_ref.cast::<HtmlInputElement>() {
                name.set(input.value());
            }
        })
    };

    html! {
        <main class="container">
            <h1>{"Welcome to Tauri + Yew"}</h1>
            <form class="row" {onsubmit}>
                <input ref={input_ref} placeholder="Enter a name..." />
                <button type="submit">{"Greet"}</button>
            </form>
            <p>{ format!("Hello, {}!", *name) }</p>
        </main>
    }
}