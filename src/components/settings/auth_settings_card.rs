use web_sys::{HtmlInputElement, InputEvent, MouseEvent};
use yew::{function_component, html, use_state, Callback, Html, Properties, TargetCast, UseStateHandle};
use crate::components::settings::settings_card::SettingsCard;
use crate::persistence_manager::AuthSettings;

#[derive(PartialEq)]
pub enum AuthType {
    Untis,
    Book2Eat,
}

#[derive(Properties, PartialEq)]
pub struct AuthCardProps {
    pub r#type: AuthType,
    pub initial: AuthSettings,
    pub on_save: Callback<AuthSettings>,
}

#[function_component(AuthSettingsCard)]
pub fn auth_settings_card(props: &AuthCardProps) -> Html {
    let school = use_state(|| props.initial.school_identifier.clone());
    let user = use_state(|| props.initial.user_identifier.clone());
    let secret = use_state(|| props.initial.secret.clone());
    let secret_visible = use_state(|| false);

    let is_dirty = *school != props.initial.school_identifier
        || *user != props.initial.user_identifier
        || *secret != props.initial.secret;

    let on_input = |state: UseStateHandle<String>| {
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            state.set(input.value());
        })
    };

    let handle_save = {
        let on_save = props.on_save.clone();
        let school = school.clone();
        let user = user.clone();
        let secret = secret.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            on_save.emit(AuthSettings {
                school_identifier: (*school).clone(),
                user_identifier: (*user).clone(),
                secret: (*secret).clone(),
            });
        })
    };

    let secret_icon_class = if *secret_visible { "bi bi-eye text-primary" } else { "bi bi-eye-slash text-secondary" };

    html! {
        <SettingsCard title={ match props.r#type {AuthType::Untis => "Untis-Authentication", AuthType::Book2Eat => "Book2Eat-Authentication"}}>
            <form>
                <div class="mb-3">
                    <label class="form-label small text-secondary">{ match props.r#type {AuthType::Untis => "School", AuthType::Book2Eat => "Canteen-ID"}}</label>
                    <input type="text" value={(*school).clone()} oninput={on_input(school.clone())} class="form-control" />
                </div>
                <div class="mb-3">
                    <label class="form-label small text-secondary">{ match props.r#type {AuthType::Untis => "Username", AuthType::Book2Eat => "Mail"}}</label>
                    <input type="text" value={(*user).clone()} oninput={on_input(user.clone())} class="form-control" />
                </div>
                <div class="mb-3">
                    <label class="form-label small text-secondary">{ match props.r#type {AuthType::Untis => "Secret", AuthType::Book2Eat => "Password"}}</label>
                    <div class="input-group">
                        <input type={if *secret_visible {"text"} else {"password"}}
                               value={(*secret).clone()}
                               oninput={on_input(secret.clone())}
                               class="form-control"/>
                        <button class="btn btn-dark btn-custom-toggle bg-transparent" style="border-color: #495057;" type="button" onclick={move |_| secret_visible.set(!*secret_visible)}>
                            <i class={secret_icon_class}></i>
                        </button>
                    </div>
                </div>
                <button onclick={handle_save} disabled={!is_dirty} class={if is_dirty {"btn btn-primary w-100"} else {"btn btn-outline-success w-100"}}>
                    {if is_dirty {"Save Changes"} else {"Saved"}}
                </button>
            </form>
        </SettingsCard>
    }
}