use crate::components::settings::settings_card::SettingsCard;
use crate::persistence_manager::VisualSettings;
use yew::{function_component, html, use_effect_with, use_state, Callback, Html, Properties};

#[derive(Properties, PartialEq)]
pub struct VisualCardProps {
    pub initial: VisualSettings,
    pub on_save: Callback<VisualSettings>,
}

#[function_component(VisualSettingsCard)]
pub fn visual_settings_card(props: &VisualCardProps) -> Html {
    let force_ascii_timetable = use_state(|| props.initial.force_ascii_timetable);

    let on_save = props.on_save.clone();
    let force_ascii_timetable_val = *force_ascii_timetable;
    use_effect_with(force_ascii_timetable_val, move |&val| {
        on_save.emit(VisualSettings {
            force_ascii_timetable: val,
        });
        || ()
    });

    let on_toggle_ascii = {
        let force_ascii_timetable = force_ascii_timetable.clone();
        Callback::from(move |_| {
            force_ascii_timetable.set(!*force_ascii_timetable);
        })
    };

    html! {
        <SettingsCard title="Visual Settings">
            <form onsubmit={Callback::from(|e: yew::prelude::SubmitEvent| e.prevent_default())}>
                <div class="mb-3 form-check">
                    <input
                        type="checkbox"
                        class="form-check-input"
                        id="asciiCheck"
                        checked={*force_ascii_timetable}
                        onclick={on_toggle_ascii}
                    />
                    <label class="form-check-label small text-secondary" for="asciiCheck" style="cursor: pointer;">
                        {"Force ASCII Timetable"}
                    </label>
                </div>
                <div class="text-end">
                    <span class="badge rounded-pill bg-success opacity-75" style="font-size: 0.7rem;">
                        {"Settings autosaved"}
                    </span>
                </div>
            </form>
        </SettingsCard>
    }
}