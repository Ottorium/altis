use crate::persistence_manager::PersistenceManager;
use crate::untis_client::UntisClient;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct AuthWrapperProps {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(AuthWrapper)]
pub fn auth_wrapper(props: &AuthWrapperProps) -> Html {
    let session = use_state(|| UntisClient::is_authenticated());
    let error = use_state(|| Option::<String>::None);

    let onclick = {
        let (session, error) = (session.clone(), error.clone());
        Callback::from(move |_| {
            let (session, error) = (session.clone(), error.clone());
            error.set(None);
            spawn_local(async move {
                if UntisClient::is_authenticated() {
                    return session.set(true);
                }

                match PersistenceManager::get_settings() {
                    Ok(s) => match UntisClient::get_session_into_cookies(
                        s.school_name,
                        s.username,
                        s.auth_secret,
                    )
                    .await
                    {
                        Ok(_) => session.set(true),
                        Err(e) => {
                            error.set(Some(format!("Login failed: {e}")));
                            session.set(false);
                        }
                    },
                    Err(e) => {
                        error.set(Some(format!("No credentials found: {e}")));
                        session.set(false);
                    }
                }
            });
        })
    };

    if *session {
        html! { for props.children.iter() }
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

                        <button class="btn btn-primary w-100 py-2 shadow-sm" {onclick}>
                            <i class="bi bi-arrow-clockwise me-2"></i>
                            {"Retry Connection"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
