use crate::components::forms::{dimmed_unless, on_enter, on_text_input, Checkbox};
use altis_core::settings::{describe_minutes, ExamReminderSettings};
use yew::prelude::*;

/// The lead times offered as buttons, in minutes: from a quarter of an hour to a week
const LEAD_TIME_PRESETS: [i64; 6] = [15, 60, 180, 1440, 2880, 10080];

#[derive(Properties, PartialEq)]
pub struct ExamRemindersProps {
    pub settings: ExamReminderSettings,
    pub on_change: Callback<ExamReminderSettings>,
}

/// Whether to be reminded of exams, and how long before them
#[function_component(ExamReminders)]
pub fn exam_reminders(props: &ExamRemindersProps) -> Html {
    let custom_lead = use_state(String::new);
    let settings = &props.settings;

    // a callback that changes the settings by what it is called with
    fn change<A: 'static>(props: &ExamRemindersProps, apply: impl Fn(&mut ExamReminderSettings, A) + 'static) -> Callback<A> {
        let (settings, on_change) = (props.settings.clone(), props.on_change.clone());
        Callback::from(move |argument| {
            let mut updated = settings.clone();
            apply(&mut updated, argument);
            on_change.emit(updated);
        })
    }

    let on_toggle_enabled = change(props, |settings, _: MouseEvent| settings.enabled = !settings.enabled);
    let on_toggle_lead_time = change(props, |settings, minutes: i64| settings.toggle_lead_time(minutes));
    let on_remove_lead_time = change(props, |settings, minutes: i64| settings.remove_lead_time(minutes));

    let on_add_custom = {
        let custom_lead = custom_lead.clone();
        let add = change(props, |settings, minutes: i64| settings.add_lead_time(minutes));
        Callback::from(move |_: ()| {
            if let Ok(minutes) = custom_lead.trim().parse::<i64>()
                && minutes > 0 {
                add.emit(minutes);
                custom_lead.set(String::new());
            }
        })
    };
    let on_add_custom_click = {
        let on_add_custom = on_add_custom.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            on_add_custom.emit(());
        })
    };

    let custom_lead_times = settings.lead_times_minutes.iter().copied().filter(|minutes| !LEAD_TIME_PRESETS.contains(minutes)).collect::<Vec<_>>();

    html! {
        <>
            <Checkbox id="examEnabled" label="Exam reminders" class="mb-2 form-check" label_class="form-check-label small text-light fw-bold"
                      checked={settings.enabled} on_toggle={on_toggle_enabled} />
            <div class="ps-4" style={dimmed_unless(settings.enabled)}>
                <div class="form-text text-secondary small mb-2">{"Remind me before an exam:"}</div>
                <div class="d-flex flex-wrap gap-2 mb-3">
                    { for LEAD_TIME_PRESETS.into_iter().map(|minutes| {
                        let button_class = if settings.has_lead_time(minutes) { "btn-primary" } else { "btn-outline-secondary" };
                        html! {
                            <button type="button" class={classes!("btn", "btn-sm", button_class)} onclick={on_toggle_lead_time.reform(move |_| minutes)}>
                                { format!("{} before", describe_minutes(minutes)) }
                            </button>
                        }
                    })}
                </div>

                if !custom_lead_times.is_empty() {
                    <div class="d-flex flex-column gap-2 mb-3">
                        { for custom_lead_times.into_iter().map(|minutes| html! {
                            <div class="d-flex align-items-center justify-content-between p-2 rounded bg-dark border border-secondary border-opacity-25" key={minutes.to_string()}>
                                <span class="small text-light">{ format!("{} before", describe_minutes(minutes)) }</span>
                                <button type="button" class="btn btn-outline-danger btn-sm px-2 py-1" onclick={on_remove_lead_time.reform(move |_| minutes)}>
                                    <i class="bi bi-trash"></i>
                                </button>
                            </div>
                        })}
                    </div>
                }

                <div class="d-flex gap-2 align-items-center">
                    <input
                        type="number"
                        min="1"
                        class="form-control form-control-sm bg-dark text-light border-secondary"
                        style="max-width: 160px;"
                        placeholder="Custom minutes before"
                        value={(*custom_lead).clone()}
                        oninput={on_text_input(&custom_lead)}
                        onkeydown={on_enter(on_add_custom)}
                    />
                    <button type="button" class="btn btn-sm btn-primary" onclick={on_add_custom_click} disabled={custom_lead.trim().is_empty()}>
                        <i class="bi bi-plus-lg me-1"></i>{"Add"}
                    </button>
                </div>
            </div>
        </>
    }
}
