use crate::components::loading::LoadingComponent;
use crate::data_models::response_models::untis_messages::{MessagePreview, MessageSender, StorageAttachment};
use crate::native;
use crate::untis::untis_client::UntisClient;
use chrono::{Datelike, Local, NaiveDateTime};
use std::collections::HashSet;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::suspense::use_future_with;

#[function_component(MessagesComponent)]
pub fn messages() -> HtmlResult {
    let reload_trigger = use_state(|| 0);
    let selected = use_state(|| None::<i32>);
    // opened since the list was loaded, so the list still says they're unread
    let opened = use_state(HashSet::<i32>::new);

    let res = use_future_with(*reload_trigger, |_| async {
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
        let trigger = reload_trigger.clone();
        Callback::from(move |_| trigger.set(*trigger + 1))
    };

    let is_unread = |message: &MessagePreview| !message.is_message_read && !opened.contains(&message.id);
    let unread_count = (*res).as_ref().map_or(0, |messages| messages.iter().filter(|m| is_unread(m)).count());

    let list = match &*res {
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
                        <Suspense fallback={html! { <LoadingComponent /> }}>
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
                <small class="text-secondary flex-shrink-0">{ format_sent_short(message.sent_date_time) }</small>
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
fn format_sent_short(sent: NaiveDateTime) -> String {
    let today = Local::now().date_naive();
    let format = if sent.date() == today {
        "%H:%M"
    } else if sent.year() == today.year() {
        "%d %b"
    } else {
        "%d %b %Y"
    };
    sent.format(format).to_string()
}

fn render_avatar(sender: &MessageSender) -> Html {
    match &sender.image_url {
        Some(url) => html! { <img src={url.clone()} alt="" class="rounded-circle flex-shrink-0 message-avatar" /> },
        None => html! {
            <div class="rounded-circle flex-shrink-0 message-avatar d-flex align-items-center justify-content-center bg-secondary bg-opacity-25 fw-bold">
                { sender.display_name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default() }
            </div>
        },
    }
}

#[derive(Properties, PartialEq)]
struct MessageDetailProps {
    id: i32,
    on_back: Callback<()>,
}

#[function_component(MessageDetail)]
fn message_detail(props: &MessageDetailProps) -> HtmlResult {
    let res = use_future_with(props.id, |id| {
        let id = *id;
        async move {
            let client = UntisClient::new()?;
            client.get_message(id).await
        }
    })?;

    let back_button = html! {
        <button class="btn btn-link text-primary text-decoration-none d-md-none p-0 mb-3" onclick={props.on_back.reform(|_| ())}>
            <i class="bi bi-chevron-left me-1"></i>{"Messages"}
        </button>
    };

    Ok(html! {
        <div class="flex-grow-1 overflow-y-auto p-3 p-md-4">
            { back_button }
            { match &*res {
                Err(err) => html! { <div class="alert alert-danger">{ err.to_string() }</div> },
                Ok(message) => html! {
                    <>
                        <h4 class="fw-bold mb-3">{ &message.subject }</h4>
                        <div class="d-flex align-items-center gap-3 mb-4">
                            { render_avatar(&message.sender) }
                            <div class="d-flex flex-column">
                                <span class="fw-semibold text-primary">{ &message.sender.display_name }</span>
                                <small class="text-secondary">{ message.sent_date_time.format("%A, %d %B %Y, %H:%M").to_string() }</small>
                            </div>
                        </div>
                        <div class="message-content">{ &message.content }</div>
                        if !message.storage_attachments.is_empty() {
                            <div class="d-flex flex-column align-items-start gap-2 mt-4 pt-3 border-top border-secondary border-opacity-25">
                                { for message.storage_attachments.iter().map(|attachment| html! {
                                    <AttachmentButton attachment={attachment.clone()} />
                                }) }
                            </div>
                        }
                    </>
                },
            }}
        </div>
    })
}

#[derive(Properties, PartialEq)]
struct AttachmentButtonProps {
    attachment: StorageAttachment,
}

#[function_component(AttachmentButton)]
fn attachment_button(props: &AttachmentButtonProps) -> Html {
    let downloading = use_state(|| false);
    // Ok is a success message, Err an error
    let status = use_state(|| None::<Result<String, String>>);

    let onclick = {
        let attachment = props.attachment.clone();
        let downloading = downloading.clone();
        let status = status.clone();
        Callback::from(move |_| {
            downloading.set(true);
            status.set(None);
            let attachment = attachment.clone();
            let downloading = downloading.clone();
            let status = status.clone();
            spawn_local(async move {
                match download(&attachment).await {
                    Ok(true) => status.set(Some(Ok("Saved".to_string()))),
                    Ok(false) => {}
                    Err(e) => status.set(Some(Err(format!("Download failed: {e}")))),
                }
                downloading.set(false);
            });
        })
    };

    html! {
        <div class="mw-100">
            <button type="button" class="btn btn-outline-primary btn-sm d-inline-flex align-items-center gap-2 mw-100" disabled={*downloading} {onclick}>
                if *downloading {
                    <span class="spinner-border spinner-border-sm flex-shrink-0" role="status"></span>
                } else {
                    <i class="bi bi-paperclip flex-shrink-0"></i>
                }
                <span class="text-truncate">{ &props.attachment.name }</span>
            </button>
            { match &*status {
                Some(Ok(msg)) => html! { <small class="text-success ms-2">{ msg }</small> },
                Some(Err(err)) => html! { <div class="small text-danger mt-1">{ err }</div> },
                None => html! {},
            }}
        </div>
    }
}

async fn download(attachment: &StorageAttachment) -> Result<bool, String> {
    let client = UntisClient::new().map_err(|e| e.to_string())?;
    let link = client.get_attachment_download(&attachment.id).await.map_err(|e| e.to_string())?;
    native::download_file(&link.download_url, &link.headers(), &attachment.name).await
}
