use crate::VERSION;
use crate::web_env::WebEnv;
use altis_core::update_check::{self, RELEASES_PAGE};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct UpdateBannerProps {
    /// whether to look for a newer release at all
    pub enabled: bool,
}

/// Shown at the top of the app when a newer release has been published. Hiding it puts it away
/// for ten days, unless something newer than the hidden release comes out in the meantime.
#[function_component(UpdateBanner)]
pub fn update_banner(props: &UpdateBannerProps) -> Html {
    let available = use_state(|| None::<String>);

    {
        let available = available.clone();
        // once per app start, and again when the check is switched on; the check itself only
        // reaches GitHub once a day
        use_effect_with(props.enabled, move |enabled| {
            if *enabled {
                spawn_local(async move {
                    available.set(update_check::check::<WebEnv>(VERSION).await);
                });
            }
            || ()
        });
    }

    let Some(version) = (*available).clone().filter(|_| props.enabled) else {
        return html! {};
    };

    let on_dismiss = {
        let available = available.clone();
        let version = version.clone();
        Callback::from(move |_| {
            update_check::dismiss::<WebEnv>(&version);
            available.set(None);
        })
    };

    html! {
        <div class="alert alert-primary d-flex align-items-center gap-2 rounded-0 border-0 mb-0 py-2 px-3" role="alert">
            <i class="bi bi-arrow-up-circle"></i>
            <span class="flex-grow-1">
                { format!("Altis {version} is out, you have {VERSION}") }
            </span>
            <a
                href={RELEASES_PAGE}
                target="_blank"
                rel="noopener noreferrer"
                class="btn btn-sm btn-primary text-nowrap"
            >
                {"Get it"}
            </a>
            <button
                type="button"
                class="btn-close btn-close-white"
                title="Hide for 10 days"
                aria-label="Hide for 10 days"
                onclick={on_dismiss}
            ></button>
        </div>
    }
}
