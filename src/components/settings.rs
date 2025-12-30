use crate::persistence_manager::*;
use crate::untis_client::UntisClient;
use log::error;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[function_component(SettingsComponent)]
pub fn settings() -> Html {
    let username_ref = use_node_ref();
    let secret_ref = use_node_ref();
    let school_ref = use_node_ref();

    let saved_settings = PersistenceManager::get_settings().ok();

    let (school_val, user_val, secret_val) = match saved_settings {
        Some(s) => (s.school_name, s.username, s.auth_secret),
        None => ("".to_string(), "".to_string(), "".to_string()),
    };

    let error_message = use_state(|| None::<String>);

    let save = {
        let username_ref = username_ref.clone();
        let secret_ref = secret_ref.clone();
        let school_ref = school_ref.clone();
        let error_message = error_message.clone();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();

            error_message.set(None);

            let username = username_ref.cast::<HtmlInputElement>().unwrap().value();
            let secret_str = secret_ref.cast::<HtmlInputElement>().unwrap().value();
            let school = school_ref.cast::<HtmlInputElement>().unwrap().value();

            let settings = Settings {
                school_name: school.clone(),
                username: username.clone(),
                auth_secret: secret_str.clone(),
            };

            let err = PersistenceManager::save_settings(&settings).err();
            if let Some(err) = err {
                error_message.set(Some(format!("Save failed: {}", err)));
                return;
            }

            wasm_bindgen_futures::spawn_local(async move {
                let res = UntisClient::get_session_into_cookies(school, username, secret_str).await;
                if let Err(err) = res {
                    error!("Login failed: {:?}", err);
                }
            });
        })
    };


    let secret_visible = use_state(|| false);
    let on_toggle_secret = {
        let secret_visible = secret_visible.clone();
        Callback::from(move |_| secret_visible.set(!*secret_visible))
    };

    let secret_input_type = if *secret_visible { "text" } else { "password" };
    let secret_icon_class = if *secret_visible { "bi bi-eye text-primary" } else { "bi bi-eye-slash text-secondary" };

    html! {
        <div class="container-fluid bg-dark vh-100 d-flex align-items-center justify-content-center border-0 w-100 h-100" data-bs-theme="dark">
            <div class="col-12 col-md-6 col-lg-4">
                <div class="card bg-dark border-0">
                    <div class="card-body text-white">
                        <form>
                            <div class="mb-3">
                                <label class="form-label">{"School:"}</label>
                                <input ref={school_ref} type="text" value={school_val} placeholder="School" class="form-control" />
                            </div>
                            <div class="mb-3">
                                <label class="form-label">{"Username:"}</label>
                                <input ref={username_ref} type="text" value={user_val} placeholder="Username" class="form-control" />
                            </div>
                            <div class="mb-3">
                                <label class="form-label">{"Secret:"}</label>
                                <div class="input-group">
                                    <input ref={secret_ref} type={secret_input_type} value={secret_val} placeholder="Secret" class="form-control"/>
                                    <button class="btn btn-dark btn-custom-toggle" style="border-color: #495057;" type="button" onclick={on_toggle_secret}>
                                        <i class={secret_icon_class}></i>
                                    </button>
                                </div>
                            </div>
                            <button onclick={save} class="btn btn-primary w-100 fw-bold">{"Save"}</button>
                            {
                                if let Some(msg) = &*error_message {
                                    html! { <div class="text-danger mt-2 small text-center">{ msg }</div> }
                                } else {
                                    html! {}
                                }
                            }
                        </form>
                    </div>
                </div>
            </div>
        </div>
    }
}