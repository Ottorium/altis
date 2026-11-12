use super::card::{autosaved_badge, divider, SettingsCard};
use super::exam_reminders::ExamReminders;
use crate::components::forms::{dimmed_unless, select_value, update, Checkbox};
use crate::native;
use altis_core::settings::{BackgroundMode, ExamReminderSettings, NotificationSettings, MAX_WEEKS_AHEAD, PERIODIC_INTERVAL_MINUTES};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

/// The poll intervals on offer, in minutes
const POLL_INTERVALS: [u32; 6] = [1, 5, 10, 15, 30, 60];

const SECTION_LABEL_CLASS: &str = "form-check-label small text-light fw-bold";

#[derive(Properties, PartialEq)]
pub struct NotificationCardProps {
    pub initial: NotificationSettings,
    pub on_save: Callback<NotificationSettings>,
}

fn background_mode_label(mode: BackgroundMode) -> String {
    match mode {
        BackgroundMode::Off => "Don't check".to_string(),
        BackgroundMode::Periodic => format!("Check about every {PERIODIC_INTERVAL_MINUTES} minutes"),
        BackgroundMode::Continuous => "Check on the interval above".to_string(),
    }
}

/// The weeks that are watched, in words
fn weeks_ahead_label(weeks_ahead: u32) -> String {
    match weeks_ahead {
        0 => "This week".to_string(),
        1 => "This week and the next".to_string(),
        more => format!("This week and the next {more}"),
    }
}

/// What a background mode costs and what it is good for
fn background_mode_hint(mode: BackgroundMode) -> String {
    match mode {
        BackgroundMode::Off => "Notifications only arrive while Altis is open.".to_string(),
        BackgroundMode::Periodic => format!(
            "No permanent notification. Android decides when the check actually runs, so {PERIODIC_INTERVAL_MINUTES} minutes is a floor rather than a promise - expect longer gaps while the phone is idle."
        ),
        BackgroundMode::Continuous => {
            "Keeps to the interval above, but Android charges a permanent \"watching your timetable\" notification for staying awake.".to_string()
        }
    }
}

/// What to be notified about, and how often to check for it. Stored as it is changed.
#[function_component(NotificationSettingsCard)]
pub fn notification_settings_card(props: &NotificationCardProps) -> Html {
    let settings = use_state(|| props.initial.clone());
    let test_status = use_state(|| None::<Result<(), String>>);

    {
        let on_save = props.on_save.clone();
        use_effect_with((*settings).clone(), move |settings| {
            on_save.emit(settings.clone());
            || ()
        });
    }

    // a checkbox that flips one of the settings
    let toggle = |flip: fn(&mut NotificationSettings) -> &mut bool| {
        update(&settings, move |settings: &mut NotificationSettings, _: MouseEvent| {
            let flag = flip(settings);
            *flag = !*flag;
        })
    };

    let on_interval_change = update(&settings, |settings: &mut NotificationSettings, e: Event| {
        if let Ok(minutes) = select_value(&e).parse() {
            settings.poll_interval_minutes = minutes;
        }
    });
    let on_weeks_ahead_change = update(&settings, |settings: &mut NotificationSettings, e: Event| {
        if let Ok(weeks) = select_value(&e).parse() {
            settings.weeks_ahead = weeks;
        }
    });
    let on_background_mode_change = update(&settings, |settings: &mut NotificationSettings, e: Event| {
        let picked = select_value(&e);
        settings.background_mode = BackgroundMode::ALL.into_iter().find(|mode| mode.as_str() == picked).unwrap_or_default();
    });
    let on_exam_reminders_change = update(&settings, |settings: &mut NotificationSettings, exams: ExamReminderSettings| settings.exam_reminders = exams);

    let on_test_notification = {
        let test_status = test_status.clone();
        Callback::from(move |_| {
            let test_status = test_status.clone();
            spawn_local(async move {
                if !native::ensure_notification_permission().await {
                    return test_status.set(Some(Err("Notification permission was not granted".to_string())));
                }
                test_status.set(Some(native::send_notification("Altis", "This is a test notification").await));
            });
        })
    };

    let enabled = settings.enabled;
    let changes = &settings.timetable_changes;

    html! {
        <SettingsCard title="Notifications">
            <form onsubmit={Callback::from(|e: SubmitEvent| e.prevent_default())}>
                <Checkbox id="notifEnabled" label="Enable notifications" class="mb-3 form-check" label_class={SECTION_LABEL_CLASS}
                          checked={enabled} on_toggle={toggle(|settings| &mut settings.enabled)}>
                    <i
                        class="bi bi-info-circle text-secondary ms-1"
                        title="On desktop this relies on your OS notification system (e.g. a running notification daemon on Linux) and may not show up on every setup. Use \"Send test notification\" below to check."
                    ></i>
                </Checkbox>

                <div class="mb-3" style={dimmed_unless(enabled)}>
                    <label class="form-label small text-secondary mb-1">{"Check for changes every"}</label>
                    <select class="form-select form-select-sm bg-dark text-light border-secondary" style="width: auto;" onchange={on_interval_change}>
                        { for POLL_INTERVALS.iter().map(|minutes| html! {
                            <option value={minutes.to_string()} selected={settings.poll_interval_minutes == *minutes}>{ format!("{minutes} minutes") }</option>
                        })}
                    </select>
                </div>

                <div class="mb-3" style={dimmed_unless(enabled)}>
                    <label class="form-label small text-secondary mb-1" for="weeksAhead">{"Watch the timetable of"}</label>
                    <select id="weeksAhead" class="form-select form-select-sm bg-dark text-light border-secondary" style="width: auto;" onchange={on_weeks_ahead_change}>
                        { for (0..=MAX_WEEKS_AHEAD).map(|weeks| html! {
                            <option value={weeks.to_string()} selected={settings.weeks_ahead() == weeks}>{ weeks_ahead_label(weeks) }</option>
                        })}
                    </select>
                    <div class="form-text text-secondary small">
                        {"Changes and exams are only noticed in these weeks. Every week more is two more requests a check."}
                    </div>
                </div>

                if native::is_android() {
                    <div class="mb-3" style={dimmed_unless(enabled)}>
                        <label class="form-label small text-secondary mb-1">{"When the app is closed"}</label>
                        <select class="form-select form-select-sm bg-dark text-light border-secondary" style="width: auto;" onchange={on_background_mode_change}>
                            { for BackgroundMode::ALL.into_iter().map(|mode| html! {
                                <option value={mode.as_str()} selected={settings.background_mode == mode}>{ background_mode_label(mode) }</option>
                            })}
                        </select>
                        <div class="form-text text-secondary small">
                            { background_mode_hint(settings.background_mode) }
                        </div>
                    </div>
                }

                { divider() }

                <div style={dimmed_unless(enabled)}>
                    <Checkbox id="ttEnabled" label="Timetable changes" class="mb-2 form-check" label_class={SECTION_LABEL_CLASS}
                              checked={changes.enabled} on_toggle={toggle(|settings| &mut settings.timetable_changes.enabled)} />
                    <div class="ps-4 d-flex flex-column gap-1" style={dimmed_unless(changes.enabled)}>
                        <Checkbox id="ttCancelled" label="Cancelled lessons"
                                  checked={changes.notify_cancelled} on_toggle={toggle(|settings| &mut settings.timetable_changes.notify_cancelled)} />
                        <Checkbox id="ttRoom" label="Room changes"
                                  checked={changes.notify_room_change} on_toggle={toggle(|settings| &mut settings.timetable_changes.notify_room_change)} />
                        <Checkbox id="ttTime" label="Time changes"
                                  checked={changes.notify_time_change} on_toggle={toggle(|settings| &mut settings.timetable_changes.notify_time_change)} />
                        <Checkbox id="ttSub" label="Teacher / substitution changes"
                                  checked={changes.notify_substitution} on_toggle={toggle(|settings| &mut settings.timetable_changes.notify_substitution)} />
                        <Checkbox id="ttAdded" label="New lessons"
                                  checked={changes.notify_added} on_toggle={toggle(|settings| &mut settings.timetable_changes.notify_added)} />
                        <Checkbox id="ttRemoved" label="Removed lessons"
                                  checked={changes.notify_removed} on_toggle={toggle(|settings| &mut settings.timetable_changes.notify_removed)} />
                    </div>
                </div>

                { divider() }

                <div style={dimmed_unless(enabled)}>
                    <ExamReminders settings={settings.exam_reminders.clone()} on_change={on_exam_reminders_change} />
                </div>

                { divider() }

                <Checkbox id="msgEnabled" label="New message alerts" class="mb-3 form-check" label_class={SECTION_LABEL_CLASS} style={dimmed_unless(enabled)}
                          checked={settings.message_notifications.enabled} on_toggle={toggle(|settings| &mut settings.message_notifications.enabled)} />

                <Checkbox id="absenceEnabled" label="New absence alerts" class="mb-3 form-check" label_class={SECTION_LABEL_CLASS} style={dimmed_unless(enabled)}
                          checked={settings.absence_notifications.enabled} on_toggle={toggle(|settings| &mut settings.absence_notifications.enabled)}>
                    <div class="form-text small text-secondary">{"When a teacher marks you absent."}</div>
                    <Checkbox id="absenceOwn" label="Also for absences I report myself" class="form-check mt-1" style={dimmed_unless(settings.absence_notifications.enabled)}
                              checked={settings.absence_notifications.include_own} on_toggle={toggle(|settings| &mut settings.absence_notifications.include_own)} />
                </Checkbox>

                { divider() }

                <div class="d-flex align-items-center justify-content-between">
                    <button type="button" class="btn btn-outline-primary btn-sm" onclick={on_test_notification}>
                        {"Send test notification"}
                    </button>
                    { autosaved_badge() }
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
