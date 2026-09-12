use crate::data_models::response_models::untis_response_models::deserialize_null_as_default;
use chrono::NaiveDateTime;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageList {
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub incoming_messages: Vec<MessagePreview>,
}

#[derive(Default, Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSender {
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub display_name: String,
    pub image_url: Option<String>,
}

/// A message in the inbox, with only the start of its content
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePreview {
    pub id: i32,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub subject: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub content_preview: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub sender: MessageSender,
    pub sent_date_time: NaiveDateTime,
    #[serde(default)]
    pub has_attachments: bool,
    #[serde(default)]
    pub is_message_read: bool,
}

#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub subject: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub content: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub sender: MessageSender,
    pub sent_date_time: NaiveDateTime,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub storage_attachments: Vec<StorageAttachment>,
}

#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct StorageAttachment {
    pub id: String,
    pub name: String,
}

/// Where to download an attachment from, the link is only valid for a few minutes
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentDownload {
    pub download_url: String,
    #[serde(default, deserialize_with = "deserialize_null_as_default")]
    pub additional_headers: Vec<AttachmentHeader>,
}

#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct AttachmentHeader {
    pub key: String,
    pub value: String,
}

impl AttachmentDownload {
    /// The files are encrypted on the storage server, these headers carry the key to decrypt them
    pub fn headers(&self) -> HashMap<String, Vec<String>> {
        self.additional_headers.iter()
            // the host is already set by the request itself
            .filter(|h| !h.key.eq_ignore_ascii_case("host"))
            .map(|h| (h.key.clone(), vec![h.value.clone()]))
            .collect()
    }
}
