use super::card::SettingsCard;
use crate::components::forms::on_text_input;
use altis_core::settings::AuthSettings;
use web_sys::MouseEvent;
use yew::prelude::*;

/// What a login is for
#[derive(Clone, Copy, PartialEq)]
pub enum AuthKind {
    Untis,
    Book2Eat,
}

impl AuthKind {
    fn title(self) -> &'static str {
        match self {
            AuthKind::Untis => "Untis-Authentication",
            AuthKind::Book2Eat => "Book2Eat-Authentication",
        }
    }

    fn school_label(self) -> &'static str {
        match self {
            AuthKind::Untis => "School",
            AuthKind::Book2Eat => "Canteen-ID (2 for STH Hollabrunn)",
        }
    }

    fn user_label(self) -> &'static str {
        match self {
            AuthKind::Untis => "Username",
            AuthKind::Book2Eat => "Mail",
        }
    }

    fn secret_label(self) -> &'static str {
        match self {
            AuthKind::Untis => "Secret",
            AuthKind::Book2Eat => "Password",
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct AuthCardProps {
    pub kind: AuthKind,
    pub initial: AuthSettings,
    pub on_save: Callback<AuthSettings>,
}

/// The fields of a login, stored when its button is pressed rather than as they are typed
#[function_component(AuthSettingsCard)]
pub fn auth_settings_card(props: &AuthCardProps) -> Html {
    let school = use_state(|| props.initial.school_identifier.clone());
    let user = use_state(|| props.initial.user_identifier.clone());
    let secret = use_state(|| props.initial.secret.clone());
    let secret_visible = use_state(|| false);

    let entered = AuthSettings { school_identifier: (*school).clone(), user_identifier: (*user).clone(), secret: (*secret).clone() };
    let is_dirty = entered != props.initial;

    let on_save = {
        let on_save = props.on_save.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            on_save.emit(entered.clone());
        })
    };

    let secret_icon_class = if *secret_visible { "bi bi-eye text-primary" } else { "bi bi-eye-slash text-secondary" };

    html! {
        <SettingsCard title={props.kind.title()}>
            <form>
                if props.kind == AuthKind::Untis {
                    <p class="small text-secondary mb-3">
                        {"You can find this info on https://htl-hl.webuntis.com/profile → Data access (Freigaben) → Display (Anzeigen)"}
                    </p>
                }
                <div class="mb-3">
                    <label class="form-label small text-secondary">{ props.kind.school_label() }</label>
                    <input type="text" value={(*school).clone()} oninput={on_text_input(&school)} class="form-control" />
                </div>
                <div class="mb-3">
                    <label class="form-label small text-secondary">{ props.kind.user_label() }</label>
                    <input type="text" value={(*user).clone()} oninput={on_text_input(&user)} class="form-control" />
                </div>
                <div class="mb-3">
                    <label class="form-label small text-secondary">{ props.kind.secret_label() }</label>
                    <div class="input-group">
                        <input type={if *secret_visible {"text"} else {"password"}}
                               value={(*secret).clone()}
                               oninput={on_text_input(&secret)}
                               class="form-control"/>
                        <button class="btn btn-dark btn-custom-toggle bg-transparent" style="border-color: #495057;" type="button" onclick={move |_| secret_visible.set(!*secret_visible)}>
                            <i class={secret_icon_class}></i>
                        </button>
                    </div>
                </div>
                <button onclick={on_save} disabled={!is_dirty} class={if is_dirty {"btn btn-primary w-100"} else {"btn btn-outline-success w-100"}}>
                    {if is_dirty {"Save Changes"} else {"Saved"}}
                </button>
            </form>
        </SettingsCard>
    }
}
