mod detail;

use crate::components::loading::Loading;
use crate::untis::UntisClient;
use altis_core::models::messages::MessagePreview;
use chrono::{Datelike, Local, NaiveDateTime};
use detail::MessageDetail;
use std::collections::HashSet;
use yew::prelude::*;
use yew::suspense::use_future_with;

/// The Untis inbox: the list of messages, and the one that is opened next to it
#[function_component(MessagesPage)]
pub fn messages_page() -> HtmlResult {
    let reload_trigger = use_state(|| 0u32);
    let selected = use_state(|| None::<i32>);
    // opened since the list was loaded, so the list still says they're unread
    let opened = use_state(HashSet::<i32>::new);

    let messages = use_future_with(*reload_trigger, |_| async {
        let client = UntisClient::new()?;
        client.get_messages().await
    })?;

    let on_select = {
        let selected = selected.clone();
        let opened = opened.clone();
        Callback::from(move |id: i32| {
            selected.set(Some(id));
            let mut ids = (*opened).clone();
            ids.insert(id);
            opened.set(ids);
        })
    };

    let on_back = {
        let selected = selected.clone();
        Callback::from(move |_| selected.set(None))
    };

    let on_reload = {
        let reload_trigger = reload_trigger.clone();
        Callback::from(move |_| reload_trigger.set(*reload_trigger + 1))
    };

    let is_unread = |message: &MessagePreview| !message.is_message_read && !opened.contains(&message.id);
    let unread_count = (*messages).as_ref().map_or(0, |messages| messages.iter().filter(|message| is_unread(message)).count());

    let list = match &*messages {
        Err(err) => html! { <div class="alert alert-danger m-3">{ err.to_string() }</div> },
        Ok(messages) if messages.is_empty() => html! {
            <p class="text-secondary text-center mt-5">{"No messages"}</p>
        },
        Ok(messages) => html! {
            { for messages.iter().map(|message| {
                render_preview(message, is_unread(message), *selected == Some(message.id), &on_select)
            }) }
        },
    };

    // on small screens only one of the list and the opened message fits
    let (list_display, detail_display) = match *selected {
        Some(_) => ("d-none d-md-flex", "d-flex"),
        None => ("d-flex", "d-none d-md-flex"),
    };

    Ok(html! {
        <div class="d-flex flex-grow-1 h-100 overflow-hidden">
            <div class={classes!(list_display, "flex-column", "flex-shrink-0", "messages-list-pane")}>
                <div class="d-flex align-items-center justify-content-between px-3 py-3 border-bottom border-secondary border-opacity-25">
                    <div class="d-flex align-items-center gap-2">
                        <h5 class="mb-0 fw-bold">{"Messages"}</h5>
                        if unread_count > 0 {
                            <span class="badge rounded-pill bg-primary text-dark">{ unread_count }</span>
                        }
                    </div>
                    <button class="btn btn-outline-primary btn-sm" title="Reload" onclick={on_reload}>
                        <i class="bi bi-arrow-clockwise"></i>
                    </button>
                </div>
                <div class="flex-grow-1 overflow-y-auto">
                    { list }
                </div>
            </div>
            <div class={classes!(detail_display, "flex-column", "flex-grow-1", "overflow-hidden")}>
                { match *selected {
                    Some(id) => html! {
                        <Suspense fallback={html! { <Loading /> }}>
                            <MessageDetail key={id} {id} {on_back} />
                        </Suspense>
                    },
                    None => html! {
                        <div class="d-flex flex-column flex-grow-1 align-items-center justify-content-center text-secondary">
                            <i class="bi bi-envelope-open fs-1 mb-2"></i>
                            <span>{"Select a message"}</span>
                        </div>
                    },
                }}
            </div>
        </div>
    })
}

/// A message in the list: its subject, sender and the start of its content
fn render_preview(message: &MessagePreview, unread: bool, active: bool, on_select: &Callback<i32>) -> Html {
    let id = message.id;
    let onclick = on_select.reform(move |_: MouseEvent| id);

    html! {
        <button
            type="button"
            class={classes!("message-item", "d-block", "px-3", "py-3", active.then_some("selected-gradient"))}
            {onclick}
        >
            <div class="d-flex align-items-center gap-2">
                if unread {
                    <span class="message-unread-dot flex-shrink-0"></span>
                }
                <span class={classes!("text-truncate", "flex-grow-1", if unread { "fw-bold text-white" } else { "text-light" })}>
                    { &message.subject }
                </span>
                <small class="text-secondary flex-shrink-0">{ format_sent_short(message.sent_date_time, Local::now().date_naive()) }</small>
            </div>
            <div class="d-flex align-items-center gap-2 small mt-1">
                <span class="text-primary text-truncate">{ &message.sender.display_name }</span>
                if message.has_attachments {
                    <i class="bi bi-paperclip text-secondary"></i>
                }
            </div>
            <div class="small text-secondary message-preview mt-1">{ &message.content_preview }</div>
        </button>
    }
}

/// Only the time for messages from today, the year only for ones from other years
fn format_sent_short(sent: NaiveDateTime, today: chrono::NaiveDate) -> String {
    let format = if sent.date() == today {
        "%H:%M"
    } else if sent.year() == today.year() {
        "%d %b"
    } else {
        "%d %b %Y"
    };
    sent.format(format).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn a_message_says_no_more_about_when_it_was_sent_than_needed() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 11).unwrap();
        let sent = |year, month, day| NaiveDate::from_ymd_opt(year, month, day).unwrap().and_hms_opt(8, 15, 0).unwrap();

        assert_eq!(format_sent_short(sent(2026, 11, 11), today), "08:15");
        assert_eq!(format_sent_short(sent(2026, 11, 9), today), "09 Nov");
        assert_eq!(format_sent_short(sent(2025, 9, 1), today), "01 Sep 2025");
    }
}
