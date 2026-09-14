use crate::components::settings::settings_card::SettingsCard;
use crate::native;
use crate::persistence_manager::{describe_lead_time_minutes, ExamReminderSettings, MessageNotificationSettings, NotificationSettings, TimetableChangeSettings};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::{
    Callback, Html, InputEvent, KeyboardEvent, MouseEvent, Properties, TargetCast, classes,
    function_component, html, use_effect_with, use_state,
};

#[derive(Properties, PartialEq)]
pub struct NotificationCardProps {
    pub initial: NotificationSettings,
    pub on_save: Callback<NotificationSettings>,
}

const POLL_INTERVALS: [u32; 6] = [1, 5, 10, 15, 30, 60];
const LEAD_TIME_PRESETS: [i64; 6] = [15, 60, 180, 1440, 2880, 10080];

#[function_component(NotificationSettingsCard)]
pub fn notification_settings_card(props: &NotificationCardProps) -> Html {
    let enabled = use_state(|| props.initial.enabled);
    let poll_interval_minutes = use_state(|| props.initial.poll_interval_minutes);
    let timetable_changes = use_state(|| props.initial.timetable_changes.clone());
    let exam_reminders = use_state(|| props.initial.exam_reminders.clone());
    let message_notifications = use_state(|| props.initial.message_notifications.clone());
    let custom_lead = use_state(String::new);
    let test_status = use_state(|| None::<Result<(), String>>);

    let on_save = props.on_save.clone();
    let settings = NotificationSettings {
        enabled: *enabled,
        poll_interval_minutes: *poll_interval_minutes,
        timetable_changes: (*timetable_changes).clone(),
        exam_reminders: (*exam_reminders).clone(),
        message_notifications: (*message_notifications).clone(),
    };
    use_effect_with(settings, move |settings| {
        on_save.emit(settings.clone());
        || ()
    });

    let on_toggle_enabled = {
        let enabled = enabled.clone();
        Callback::from(move |_| enabled.set(!*enabled))
    };

    let on_interval_change = {
        let poll_interval_minutes = poll_interval_minutes.clone();
        Callback::from(move |e: yew::Event| {
            let select: HtmlSelectElement = e.target_unchecked_into();
            if let Ok(minutes) = select.value().parse::<u32>() {
                poll_interval_minutes.set(minutes);
            }
        })
    };

    let toggle_tt = |get: fn(&mut TimetableChangeSettings) -> &mut bool| {
        let timetable_changes = timetable_changes.clone();
        Callback::from(move |_| {
            let mut updated = (*timetable_changes).clone();
            *get(&mut updated) = !*get(&mut updated);
            timetable_changes.set(updated);
        })
    };
    let on_tt_enabled = toggle_tt(|s| &mut s.enabled);
    let on_tt_cancelled = toggle_tt(|s| &mut s.notify_cancelled);
    let on_tt_room = toggle_tt(|s| &mut s.notify_room_change);
    let on_tt_time = toggle_tt(|s| &mut s.notify_time_change);
    let on_tt_sub = toggle_tt(|s| &mut s.notify_substitution);

    let toggle_exam = |get: fn(&mut ExamReminderSettings) -> &mut bool| {
        let exam_reminders = exam_reminders.clone();
        Callback::from(move |_| {
            let mut updated = (*exam_reminders).clone();
            *get(&mut updated) = !*get(&mut updated);
            exam_reminders.set(updated);
        })
    };
    let on_exam_enabled = toggle_exam(|s| &mut s.enabled);

    let toggle_lead_time = {
        let exam_reminders = exam_reminders.clone();
        move |minutes: i64| {
            let exam_reminders = exam_reminders.clone();
            Callback::from(move |_| {
                let mut updated = (*exam_reminders).clone();
                if let Some(pos) = updated.lead_times_minutes.iter().position(|&m| m == minutes) {
                    updated.lead_times_minutes.remove(pos);
                } else {
                    updated.lead_times_minutes.push(minutes);
                    updated.lead_times_minutes.sort();
                }
                exam_reminders.set(updated);
            })
        }
    };

    let on_remove_lead = {
        let exam_reminders = exam_reminders.clone();
        Callback::from(move |minutes: i64| {
            let mut updated = (*exam_reminders).clone();
            updated.lead_times_minutes.retain(|&m| m != minutes);
            exam_reminders.set(updated);
        })
    };

    let on_custom_lead_input = {
        let custom_lead = custom_lead.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            custom_lead.set(input.value());
        })
    };

    let on_add_custom_lead = {
        let custom_lead = custom_lead.clone();
        let exam_reminders = exam_reminders.clone();
        Callback::from(move |_: ()| {
            if let Ok(minutes) = custom_lead.trim().parse::<i64>()
                && minutes > 0 {
                let mut updated = (*exam_reminders).clone();
                if !updated.lead_times_minutes.contains(&minutes) {
                    updated.lead_times_minutes.push(minutes);
                    updated.lead_times_minutes.sort();
                    exam_reminders.set(updated);
                }
                custom_lead.set(String::new());
            }
        })
    };

    let on_add_custom_lead_click = {
        let on_add_custom_lead = on_add_custom_lead.clone();
        Callback::from(move |e: MouseEvent| {
            e.prevent_default();
            on_add_custom_lead.emit(());
        })
    };

    let on_custom_lead_keydown = {
        let on_add_custom_lead = on_add_custom_lead.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" {
                e.prevent_default();
                on_add_custom_lead.emit(());
            }
        })
    };

    let on_msg_enabled = {
        let message_notifications = message_notifications.clone();
        Callback::from(move |_| {
            let updated = MessageNotificationSettings { enabled: !message_notifications.enabled };
            message_notifications.set(updated);
        })
    };

    let on_test_notification = {
        let test_status = test_status.clone();
        Callback::from(move |_| {
            let test_status = test_status.clone();
            spawn_local(async move {
                if !native::ensure_notification_permission().await {
                    test_status.set(Some(Err("Notification permission was not granted".to_string())));
                    return;
                }
                test_status.set(match native::send_notification("Altis", "This is a test notification").await {
                    Ok(()) => Some(Ok(())),
                    Err(e) => Some(Err(e)),
                });
            });
        })
    };

    let custom_lead_times: Vec<i64> = exam_reminders.lead_times_minutes.iter()
        .filter(|m| !LEAD_TIME_PRESETS.contains(*m))
        .copied()
        .collect();

    let dim = |on: bool| if on { "" } else { "opacity: 0.4; pointer-events: none;" };

    html! {
        <SettingsCard title="Notifications">
            <form onsubmit={Callback::from(|e: yew::prelude::SubmitEvent| e.prevent_default())}>
                <div class="mb-3 form-check">
                    <input type="checkbox" class="form-check-input" id="notifEnabled" checked={*enabled} onclick={on_toggle_enabled} />
                    <label class="form-check-label small text-light fw-bold" for="notifEnabled" style="cursor: pointer;">
                        {"Enable notifications"}
                    </label>
                    <i
                        class="bi bi-info-circle text-secondary ms-1"
                        title="On desktop this relies on your OS notification system (e.g. a running notification daemon on Linux) and may not show up on every setup. Use \"Send test notification\" below to check."
                    ></i>
                </div>

                <div class="mb-3" style={dim(*enabled)}>
                    <label class="form-label small text-secondary mb-1">{"Check for changes every"}</label>
                    <select class="form-select form-select-sm bg-dark text-light border-secondary" style="width: auto;" onchange={on_interval_change}>
                        { for POLL_INTERVALS.iter().map(|m| html! {
                            <option value={m.to_string()} selected={*poll_interval_minutes == *m}>{ format!("{m} minutes") }</option>
                        })}
                    </select>
                </div>

                <hr class="border-secondary opacity-25 my-3" />

                <div style={dim(*enabled)}>
                    <div class="mb-2 form-check">
                        <input type="checkbox" class="form-check-input" id="ttEnabled" checked={timetable_changes.enabled} onclick={on_tt_enabled} />
                        <label class="form-check-label small text-light fw-bold" for="ttEnabled" style="cursor: pointer;">{"Timetable changes"}</label>
                    </div>
                    <div class="ps-4 d-flex flex-column gap-1" style={dim(timetable_changes.enabled)}>
                        <div class="form-check">
                            <input type="checkbox" class="form-check-input" id="ttCancelled" checked={timetable_changes.notify_cancelled} onclick={on_tt_cancelled} />
                            <label class="form-check-label small text-secondary" for="ttCancelled" style="cursor: pointer;">{"Cancelled lessons"}</label>
                        </div>
                        <div class="form-check">
                            <input type="checkbox" class="form-check-input" id="ttRoom" checked={timetable_changes.notify_room_change} onclick={on_tt_room} />
                            <label class="form-check-label small text-secondary" for="ttRoom" style="cursor: pointer;">{"Room changes"}</label>
                        </div>
                        <div class="form-check">
                            <input type="checkbox" class="form-check-input" id="ttTime" checked={timetable_changes.notify_time_change} onclick={on_tt_time} />
                            <label class="form-check-label small text-secondary" for="ttTime" style="cursor: pointer;">{"Time changes"}</label>
                        </div>
                        <div class="form-check">
                            <input type="checkbox" class="form-check-input" id="ttSub" checked={timetable_changes.notify_substitution} onclick={on_tt_sub} />
                            <label class="form-check-label small text-secondary" for="ttSub" style="cursor: pointer;">{"Teacher / substitution changes"}</label>
                        </div>
                    </div>
                </div>

                <hr class="border-secondary opacity-25 my-3" />

                <div style={dim(*enabled)}>
                    <div class="mb-2 form-check">
                        <input type="checkbox" class="form-check-input" id="examEnabled" checked={exam_reminders.enabled} onclick={on_exam_enabled} />
                        <label class="form-check-label small text-light fw-bold" for="examEnabled" style="cursor: pointer;">{"Exam reminders"}</label>
                    </div>
                    <div class="ps-4" style={dim(exam_reminders.enabled)}>
                        <div class="form-text text-secondary small mb-2">{"Remind me before an exam:"}</div>
                        <div class="d-flex flex-wrap gap-2 mb-3">
                            { for LEAD_TIME_PRESETS.iter().map(|&minutes| {
                                let checked = exam_reminders.lead_times_minutes.contains(&minutes);
                                let onclick = toggle_lead_time(minutes);
                                html! {
                                    <button type="button" class={classes!("btn", "btn-sm", if checked { "btn-primary" } else { "btn-outline-secondary" })} {onclick}>
                                        { format!("{} before", describe_lead_time_minutes(minutes)) }
                                    </button>
                                }
                            })}
                        </div>

                        if !custom_lead_times.is_empty() {
                            <div class="d-flex flex-column gap-2 mb-3">
                                { for custom_lead_times.iter().map(|&minutes| {
                                    let on_remove = on_remove_lead.reform(move |_| minutes);
                                    html! {
                                        <div class="d-flex align-items-center justify-content-between p-2 rounded bg-dark border border-secondary border-opacity-25" key={minutes.to_string()}>
                                            <span class="small text-light">{ format!("{} before", describe_lead_time_minutes(minutes)) }</span>
                                            <button type="button" class="btn btn-outline-danger btn-sm px-2 py-1" onclick={on_remove}>
                                                <i class="bi bi-trash"></i>
                                            </button>
                                        </div>
                                    }
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
                                oninput={on_custom_lead_input}
                                onkeydown={on_custom_lead_keydown}
                            />
                            <button type="button" class="btn btn-sm btn-primary" onclick={on_add_custom_lead_click} disabled={custom_lead.trim().is_empty()}>
                                <i class="bi bi-plus-lg me-1"></i>{"Add"}
                            </button>
                        </div>
                    </div>
                </div>

                <hr class="border-secondary opacity-25 my-3" />

                <div class="mb-3 form-check" style={dim(*enabled)}>
                    <input type="checkbox" class="form-check-input" id="msgEnabled" checked={message_notifications.enabled} onclick={on_msg_enabled} />
                    <label class="form-check-label small text-light fw-bold" for="msgEnabled" style="cursor: pointer;">{"New message alerts"}</label>
                </div>

                <hr class="border-secondary opacity-25 my-3" />

                <div class="d-flex align-items-center justify-content-between">
                    <button type="button" class="btn btn-outline-primary btn-sm" onclick={on_test_notification}>
                        {"Send test notification"}
                    </button>
                    <span class="badge rounded-pill bg-success opacity-75" style="font-size: 0.7rem;">
                        {"Settings autosaved"}
                    </span>
                </div>
                { match &*test_status {
                    Some(Ok(())) => html! { <div class="small text-success mt-2">{"Test notification sent"}</div> },
                    Some(Err(e)) => html! { <div class="small text-danger mt-2">{ e }</div> },
                    None => html! {},
                }}
            </form>
        </SettingsCard>
    }
}
