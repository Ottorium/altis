use crate::components::qr_code::QrCode;
use crate::components::qr_scanner::QrScanner;
use crate::native;
use crate::persistence_manager::{PersistenceManager, SettingsExport};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

const FILE_NAME: &str = "altis-settings.json";

#[function_component(ShareSettingsButton)]
pub fn share_settings_button() -> Html {
    let show_modal = use_state(|| false);
    let include_credentials = use_state(|| false);
    let scanning = use_state(|| false);
    let pending_import = use_state(|| None::<SettingsExport>);
    // Ok is a success message, Err an error
    let status = use_state(|| None::<Result<String, String>>);

    let settings = PersistenceManager::get_settings().ok().flatten().unwrap_or_default();
    let export = SettingsExport::new(&settings, *include_credentials);

    let toggle_modal = {
        let show_modal = show_modal.clone();
        let pending_import = pending_import.clone();
        let status = status.clone();
        Callback::from(move |_| {
            show_modal.set(!*show_modal);
            pending_import.set(None);
            status.set(None);
        })
    };

    let on_toggle_credentials = {
        let include_credentials = include_credentials.clone();
        Callback::from(move |_| include_credentials.set(!*include_credentials))
    };

    let on_save_file = {
        let status = status.clone();
        let contents = serde_json::to_string_pretty(&export).unwrap_or_default();
        Callback::from(move |_| {
            let status = status.clone();
            let contents = contents.clone();
            spawn_local(async move {
                match native::save_file(FILE_NAME, &contents).await {
                    Ok(true) => status.set(Some(Ok("Settings saved".to_string()))),
                    Ok(false) => {}
                    Err(e) => status.set(Some(Err(format!("Saving failed: {e}")))),
                }
            });
        })
    };

    let start_import = {
        let status = status.clone();
        let pending_import = pending_import.clone();
        Callback::from(move |text: String| match SettingsExport::parse(&text) {
            Ok(import) => {
                status.set(None);
                pending_import.set(Some(import));
            }
            Err(e) => status.set(Some(Err(e))),
        })
    };

    let on_open_file = {
        let status = status.clone();
        let start_import = start_import.clone();
        Callback::from(move |_| {
            let status = status.clone();
            let start_import = start_import.clone();
            spawn_local(async move {
                match native::open_file().await {
                    Ok(Some(text)) => start_import.emit(text),
                    Ok(None) => {}
                    Err(e) => status.set(Some(Err(format!("Opening failed: {e}")))),
                }
            });
        })
    };

    let on_start_scan = {
        let status = status.clone();
        let scanning = scanning.clone();
        Callback::from(move |_| {
            status.set(None);
            scanning.set(true);
        })
    };

    let on_scanned = {
        let scanning = scanning.clone();
        Callback::from(move |text: String| {
            scanning.set(false);
            start_import.emit(text);
        })
    };

    let on_scan_error = {
        let scanning = scanning.clone();
        let status = status.clone();
        Callback::from(move |e: String| {
            scanning.set(false);
            status.set(Some(Err(format!("Scanning failed: {e}"))));
        })
    };

    let on_cancel_scan = {
        let scanning = scanning.clone();
        Callback::from(move |_| scanning.set(false))
    };

    let on_cancel_import = {
        let pending_import = pending_import.clone();
        Callback::from(move |_| pending_import.set(None))
    };

    let on_confirm_import = {
        let pending_import = pending_import.clone();
        let status = status.clone();
        Callback::from(move |_| {
            let Some(import) = (*pending_import).clone() else { return };
            // broken settings are replaced as well
            let mut settings = PersistenceManager::get_settings().ok().flatten().unwrap_or_default();
            import.apply_to(&mut settings);
            match PersistenceManager::save_settings(&settings) {
                // the settings cards keep their own copy of the settings, so the page is reloaded to show the new ones
                Ok(()) => { let _ = web_sys::window().unwrap().location().reload(); }
                Err(e) => {
                    pending_import.set(None);
                    status.set(Some(Err(format!("Import failed: {e}"))));
                }
            }
        })
    };

    let share_body = html! {
        <div class="modal-body">
            <div class="form-check form-switch mb-3">
                <input
                    class="form-check-input"
                    type="checkbox"
                    role="switch"
                    id="includeCredentials"
                    checked={*include_credentials}
                    onclick={on_toggle_credentials}
                />
                <label class="form-check-label small text-secondary" for="includeCredentials" style="cursor: pointer;">
                    {"Include credentials (Untis & Book2Eat logins)"}
                </label>
            </div>
            if *include_credentials {
                <div class="alert alert-warning py-2 small">
                    <i class="bi bi-exclamation-triangle me-1"></i>
                    {"Anyone with the file or QR code can log in as you."}
                </div>
            }

            <div class="bg-white rounded p-3 mx-auto" style="width: min(100%, 280px); aspect-ratio: 1;">
                <QrCode data={serde_json::to_string(&export).unwrap_or_default()} />
            </div>
            <p class="small text-secondary text-center mt-2 mb-3">{"Scan it with Altis on another device"}</p>
            <button type="button" class="btn btn-outline-primary w-100" onclick={on_save_file}>
                <i class="bi bi-download me-1"></i>{"Save as File"}
            </button>

            <hr class="border-secondary opacity-25 my-4" />

            <label class="form-label fw-bold small text-light mb-2">{"Import"}</label>
            <div class="d-flex gap-2">
                <button type="button" class="btn btn-outline-secondary flex-fill" onclick={on_open_file}>
                    <i class="bi bi-upload me-1"></i>{"Open File"}
                </button>
                <button type="button" class="btn btn-outline-secondary flex-fill" onclick={on_start_scan}>
                    <i class="bi bi-camera me-1"></i>{"Scan QR Code"}
                </button>
            </div>

            { match &*status {
                Some(Ok(msg)) => html! { <div class="small text-success mt-3">{ msg }</div> },
                Some(Err(err)) => html! { <div class="alert alert-danger py-2 small mt-3 mb-0">{ err }</div> },
                None => html! {},
            }}
        </div>
    };

    let confirm_body = |import: &SettingsExport| html! {
        <>
            <div class="modal-body">
                <p class="mb-0">
                    { if import.has_credentials() {
                        "This replaces your visual settings and your Untis and Book2Eat credentials."
                    } else {
                        "This replaces your visual settings. Your credentials are kept."
                    }}
                </p>
            </div>
            <div class="modal-footer">
                <button type="button" class="btn btn-outline-secondary" onclick={on_cancel_import.clone()}>{"Cancel"}</button>
                <button type="button" class="btn btn-primary" onclick={on_confirm_import.clone()}>{"Import"}</button>
            </div>
        </>
    };

    let modal = html! {
        <div class="modal d-block" tabindex="-1" style="background: rgba(0,0,0,0.5)">
            <div class="modal-dialog modal-dialog-centered modal-dialog-scrollable">
                <div class="modal-content bg-dark text-white border-primary">
                    <div class="modal-header">
                        <h5 class="modal-title">
                            { if pending_import.is_some() { "Import Settings?" } else { "Share Settings" } }
                        </h5>
                        <button type="button" class="btn-close btn-close-white" onclick={toggle_modal.clone()}></button>
                    </div>
                    { match &*pending_import {
                        Some(import) => confirm_body(import),
                        None => share_body,
                    }}
                </div>
            </div>
        </div>
    };

    html! {
        <>
            <button class="btn btn-outline-primary" onclick={toggle_modal}>
                <i class="bi bi-share me-1"></i>{"Share"}
            </button>

            { if *scanning {
                html! { <QrScanner on_scan={on_scanned} on_error={on_scan_error} on_cancel={on_cancel_scan} /> }
            } else if *show_modal {
                modal
            } else {
                html! {}
            }}
        </>
    }
}
