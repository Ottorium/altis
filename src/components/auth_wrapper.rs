use crate::untis::untis_client::UntisClient;
use crate::persistence_manager::PersistenceManager;
use gloo_timers::callback::Timeout;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct AuthWrapperProps {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(AuthWrapper)]
pub fn auth_wrapper(props: &AuthWrapperProps) -> Html {
    let session = use_state(UntisClient::is_authenticated);
    let error = use_state(|| Option::<String>::None);

    let perform_login = {
        let session = session.clone();
        let error = error.clone();
        Callback::from(move |_: ()| {
            let session = session.clone();
            let error = error.clone();
            spawn_local(async move {
                if UntisClient::is_authenticated() {
                    session.set(true);
                    return;
                }

                let settings = match PersistenceManager::get_settings() {
                    Ok(Some(s)) => s,
                    Ok(None) => {
                        error.set(Some("No credentials found".to_string()));
                        session.set(false);
                        return;
                    }
                    Err(e) => {
                        error.set(Some(format!("Failed to load settings: {e}")));
                        session.set(false);
                        return;
                    }
                };

                match UntisClient::authenticate(
                    settings.untis_auth.school_identifier,
                    settings.untis_auth.user_identifier,
                    settings.untis_auth.secret,
                ).await {
                    Ok(_) => session.set(true),
                    Err(e) => {
                        error.set(Some(format!("Login failed: {e}")));
                        session.set(false);
                    }
                }
            });
        })
    };

    {
        let perform_login = perform_login.clone();
        let session = session.clone();
        use_effect_with((), move |_| {
            if !*session {
                for delay in [100, 300, 1000] {
                    let perform_login = perform_login.clone();
                    let session = session.clone();
                    Timeout::new(delay, move || {
                        if !*session {
                            perform_login.emit(());
                        }
                    }).forget();
                }
            }
            || ()
        });
    }

    let onclick = {
        let perform_login = perform_login.clone();
        let error = error.clone();
        Callback::from(move |_| {
            error.set(None);
            perform_login.emit(());
        })
    };

    if *session {
        html! {{ for props.children.iter() }}
    } else {
        html! {
            <div class="d-flex align-items-center justify-content-center vh-100 bg-dark">
                <div class="card bg-dark border-primary shadow-lg" style="max-width: 400px;">
                    <div class="card-body text-center p-5">
                        <div class="mb-4">
                            <i class="bi bi-shield-lock text-warning" style="font-size: 3rem;"></i>
                        </div>

                        <h4 class="card-title text-white mb-3">{"Authentication Required"}</h4>
                        <p class="card-text text-secondary mb-4">
                            { "Please sign in to view this content. You can set your credentials in the settings." }
                        </p>

                        { for error.as_ref().map(|msg| html! {
                            <div class="alert alert-danger py-2 small mb-4" role="alert">
                                { msg }
                            </div>
                        }) }

                        <button class="btn btn-primary w-100 py-2 shadow-sm fw-bold" {onclick}>
                            <i class="bi bi-arrow-clockwise me-2"></i>
                            {"Retry Connection"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
