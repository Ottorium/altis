use crate::storage;
use crate::untis::UntisClient;
use gloo_timers::callback::Timeout;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

/// When to try logging in after the wrapper appears. The app logs in on its own at start, so the
/// first tries mostly find that login finished rather than starting one of their own.
const LOGIN_RETRY_DELAYS_MS: [u32; 3] = [100, 300, 1000];

#[derive(Properties, PartialEq)]
pub struct AuthWrapperProps {
    #[prop_or_default]
    pub children: Children,
}

/// Logs in with the stored credentials, an error telling why it didn't work
async fn login() -> Result<(), String> {
    if UntisClient::is_authenticated() {
        return Ok(());
    }

    let settings = match storage::settings() {
        Ok(Some(settings)) => settings,
        Ok(None) => return Err("No credentials found".to_string()),
        Err(e) => return Err(format!("Failed to load settings: {e}")),
    };

    UntisClient::authenticate(&settings.untis_auth).await.map_err(|e| format!("Login failed: {e}"))
}

/// Shows its content once there is an Untis session, and a prompt to sign in until then
#[function_component(AuthWrapper)]
pub fn auth_wrapper(props: &AuthWrapperProps) -> Html {
    let authenticated = use_state(UntisClient::is_authenticated);
    let error = use_state(|| None::<String>);

    let perform_login = {
        let authenticated = authenticated.clone();
        let error = error.clone();
        Callback::from(move |_: ()| {
            let authenticated = authenticated.clone();
            let error = error.clone();
            spawn_local(async move {
                match login().await {
                    Ok(()) => authenticated.set(true),
                    Err(message) => {
                        error.set(Some(message));
                        authenticated.set(false);
                    }
                }
            });
        })
    };

    {
        let perform_login = perform_login.clone();
        let authenticated = authenticated.clone();
        use_effect_with((), move |_| {
            if !*authenticated {
                for delay in LOGIN_RETRY_DELAYS_MS {
                    let perform_login = perform_login.clone();
                    let authenticated = authenticated.clone();
                    Timeout::new(delay, move || {
                        if !*authenticated {
                            perform_login.emit(());
                        }
                    })
                    .forget();
                }
            }
            || ()
        });
    }

    if *authenticated {
        return html! {{ for props.children.iter() }};
    }

    let on_retry = {
        let error = error.clone();
        Callback::from(move |_| {
            error.set(None);
            perform_login.emit(());
        })
    };

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

                    { for error.as_ref().map(|message| html! {
                        <div class="alert alert-danger py-2 small mb-4" role="alert">
                            { message }
                        </div>
                    }) }

                    <button class="btn btn-primary w-100 py-2 shadow-sm fw-bold" onclick={on_retry}>
                        <i class="bi bi-arrow-clockwise me-2"></i>
                        {"Retry Connection"}
                    </button>
                </div>
            </div>
        </div>
    }
}
