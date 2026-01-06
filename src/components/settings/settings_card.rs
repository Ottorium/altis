use yew::{function_component, html, AttrValue, Children, Html, Properties};

#[derive(Properties, PartialEq)]
pub struct SettingsCardProps {
    pub title: AttrValue,
    pub children: Children,
}

#[function_component(SettingsCard)]
pub fn settings_card(props: &SettingsCardProps) -> Html {
    html! {
        <div class="card border-primary mb-4 shadow-sm">
            <div class="card-header">
                <h5 class="mb-0">{&props.title}</h5>
            </div>
            <div class="card-body">
                {props.children.clone()}
            </div>
        </div>
    }
}
