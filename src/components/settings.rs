use crate::persistence_manager::*;
use crate::untis_client::UntisClient;
use log::error;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[function_component(SettingsComponent)]
pub fn settings() -> Html {
    let saved_settings = PersistenceManager::get_settings().ok();
    let initial_settings = saved_settings.unwrap_or(Settings {
        school_name: "".to_string(),
        username: "".to_string(),
        auth_secret: "".to_string(),
    });

    let school_state = use_state(|| initial_settings.school_name.clone());
    let user_state = use_state(|| initial_settings.username.clone());
    let secret_state = use_state(|| initial_settings.auth_secret.clone());

    let error_message = use_state(|| None::<String>);
    let secret_visible = use_state(|| false);

    let is_dirty = *school_state != initial_settings.school_name
        || *user_state != initial_settings.username
        || *secret_state != initial_settings.auth_secret;

    let on_school_input = {
        let school_state = school_state.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            school_state.set(input.value());
        })
    };

    let on_user_input = {
        let user_state = user_state.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            user_state.set(input.value());
        })
    };

    let on_secret_input = {
        let secret_state = secret_state.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            secret_state.set(input.value());
        })
    };

    let save = {
        let school_state = school_state.clone();
        let user_state = user_state.clone();
        let secret_state = secret_state.clone();
        let error_message = error_message.clone();

        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            error_message.set(None);

            let school = (*school_state).clone();
            let username = (*user_state).clone();
            let secret_str = (*secret_state).clone();

            let settings = Settings {
                school_name: school.clone(),
                username: username.clone(),
                auth_secret: secret_str.clone(),
            };

            if let Err(err) = PersistenceManager::save_settings(&settings) {
                error_message.set(Some(format!("Save failed: {}", err)));
                return;
            }

            // We clone again here because spawn_local requires 'static ownership of the data it uses.
            let school_for_async = school.clone();
            let user_for_async = username.clone();
            let secret_for_async = secret_str.clone();

            wasm_bindgen_futures::spawn_local(async move {
                let res = UntisClient::get_session_into_cookies(
                    school_for_async,
                    user_for_async,
                    secret_for_async
                ).await;

                if let Err(err) = res {
                    error!("Login failed: {:?}", err);
                }
            });
        })
    };

    let on_toggle_secret = {
        let secret_visible = secret_visible.clone();
        Callback::from(move |_| secret_visible.set(!*secret_visible))
    };

    let secret_input_type = if *secret_visible { "text" } else { "password" };
    let secret_icon_class = if *secret_visible { "bi bi-eye text-primary" } else { "bi bi-eye-slash text-secondary" };

    let button_class = if is_dirty { "btn btn-primary w-100 fw-bold" } else { "btn btn-success w-100 fw-bold" };
    let button_text = if is_dirty { "Save Changes" } else { "Saved" };

    html! {
        <div class="container-fluid bg-dark vh-100 d-flex align-items-center justify-content-center border-0 w-100 h-100" data-bs-theme="dark">
            <div class="col-12 col-md-6 col-lg-4">
                <div class="card bg-dark border-0">
                    <div class="card-body text-white">
                        <form>
                            <div class="mb-3">
                                <label class="form-label">{"School:"}</label>
                                <input type="text" value={(*school_state).clone()} oninput={on_school_input} placeholder="School" class="form-control" />
                            </div>
                            <div class="mb-3">
                                <label class="form-label">{"Username:"}</label>
                                <input type="text" value={(*user_state).clone()} oninput={on_user_input} placeholder="Username" class="form-control" />
                            </div>
                            <div class="mb-3">
                                <label class="form-label">{"Secret:"}</label>
                                <div class="input-group">
                                    <input type={secret_input_type} value={(*secret_state).clone()} oninput={on_secret_input} placeholder="Secret" class="form-control"/>
                                    <button class="btn btn-dark btn-custom-toggle" style="border-color: #495057;" type="button" onclick={on_toggle_secret}>
                                        <i class={secret_icon_class}></i>
                                    </button>
                                </div>
                            </div>
                            <button onclick={save} class={button_class}>{button_text}</button>
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