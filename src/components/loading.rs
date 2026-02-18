use yew::prelude::*;

#[function_component(LoadingComponent)]
pub fn loading_component() -> Html {
    html! {
        <div class="d-flex flex-grow-1 flex-column justify-content-center align-items-center w-100 h-100">
            <div class="spinner-border text-primary" role="status" style="width: 3rem; height: 3rem;">
                <span class="visually-hidden">{"Loading..."}</span>
            </div>
            <p class="mt-3 text-secondary">{"fetching data..."}</p>
        </div>
    }
}
