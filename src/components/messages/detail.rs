use crate::native;
use crate::untis::UntisClient;
use altis_core::models::messages::{MessageSender, StorageAttachment};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew::suspense::use_future_with;

#[derive(Properties, PartialEq)]
pub struct MessageDetailProps {
    pub id: i32,
    /// leaves the message for the list, on a screen too small for both
    pub on_back: Callback<()>,
}

/// An opened message: its content and its attachments. Loading it is what marks it as read.
#[function_component(MessageDetail)]
pub fn message_detail(props: &MessageDetailProps) -> HtmlResult {
    let message = use_future_with(props.id, |id| {
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
            { match &*message {
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

/// The sender's picture, or their initial where they have none
fn render_avatar(sender: &MessageSender) -> Html {
    match &sender.image_url {
        Some(url) => html! { <img src={url.clone()} alt="" class="rounded-circle flex-shrink-0 message-avatar" /> },
        None => html! {
            <div class="rounded-circle flex-shrink-0 message-avatar d-flex align-items-center justify-content-center bg-secondary bg-opacity-25 fw-bold">
                { sender.display_name.chars().next().map(|initial| initial.to_uppercase().to_string()).unwrap_or_default() }
            </div>
        },
    }
}

#[derive(Properties, PartialEq)]
struct AttachmentButtonProps {
    attachment: StorageAttachment,
}

/// Downloads an attachment to wherever the user points it
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
                    // the user decided against saving it after all
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
                Some(Ok(message)) => html! { <small class="text-success ms-2">{ message }</small> },
                Some(Err(error)) => html! { <div class="small text-danger mt-1">{ error }</div> },
                None => html! {},
            }}
        </div>
    }
}

/// Downloads the attachment, `false` if the user cancelled
async fn download(attachment: &StorageAttachment) -> Result<bool, String> {
    let client = UntisClient::new().map_err(|e| e.to_string())?;
    let link = client.get_attachment_download(&attachment.id).await.map_err(|e| e.to_string())?;
    native::download_file(&link.download_url, &link.headers(), &attachment.name).await
}
