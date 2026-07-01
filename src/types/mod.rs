//! Shared serde models mirroring `components.schemas` from the `OpenAPI` spec.
//!
//! Fields are modelled as `Option` wherever the API may omit them. Server-owned
//! nested objects are intentionally kept as [`serde_json::Value`] so
//! forward-compatible fields are preserved rather than guessed and dropped.
//! String status, direction, reaction, webhook-type, and protocol fields mirror
//! Blooio's raw values so new API vocabulary does not require an SDK release.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::secret::Secret;

/// Free-form JSON value alias used for intentionally opaque API fields.
///
/// These fields represent server-owned nested objects whose schema may evolve
/// independently of the SDK. Keeping them as JSON is a stability choice, not
/// unfinished typing.
pub type Json = Value;

mod string_list_sealed {
    pub(crate) trait Sealed {}

    impl Sealed for Vec<String> {}
    impl<const N: usize> Sealed for [String; N] {}
    impl<const N: usize> Sealed for [&str; N] {}
    impl Sealed for &[String] {}
    impl Sealed for &[&str] {}
    impl Sealed for &Vec<String> {}
    impl<const N: usize> Sealed for &[String; N] {}
    impl<const N: usize> Sealed for &[&str; N] {}
}

/// A collection that can be converted into the `Vec<String>` body fields used
/// by batch-style requests.
///
/// This accepts owned `Vec<String>` values, string arrays, and string slices
/// while preserving inference for existing `vec!["value".into()]` call sites.
#[allow(private_bounds)]
pub trait IntoStringList: string_list_sealed::Sealed {
    /// Convert this collection into owned strings.
    fn into_string_vec(self) -> Vec<String>;
}

impl IntoStringList for Vec<String> {
    fn into_string_vec(self) -> Vec<String> {
        self
    }
}

impl<const N: usize> IntoStringList for [String; N] {
    fn into_string_vec(self) -> Vec<String> {
        self.into_iter().collect()
    }
}

impl<const N: usize> IntoStringList for [&str; N] {
    fn into_string_vec(self) -> Vec<String> {
        self.into_iter().map(str::to_owned).collect()
    }
}

impl IntoStringList for &[String] {
    fn into_string_vec(self) -> Vec<String> {
        self.to_vec()
    }
}

impl IntoStringList for &[&str] {
    fn into_string_vec(self) -> Vec<String> {
        self.iter().map(|value| (*value).to_owned()).collect()
    }
}

impl IntoStringList for &Vec<String> {
    fn into_string_vec(self) -> Vec<String> {
        self.clone()
    }
}

impl<const N: usize> IntoStringList for &[String; N] {
    fn into_string_vec(self) -> Vec<String> {
        self.to_vec()
    }
}

impl<const N: usize> IntoStringList for &[&str; N] {
    fn into_string_vec(self) -> Vec<String> {
        self.iter().map(|value| (*value).to_owned()).collect()
    }
}

/// Result of a phone-number lookup.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct PhoneNumberLookupResult {
    pub input: Option<String>,
    pub valid: Option<bool>,
    pub possible: Option<bool>,
    pub e164: Option<String>,
    pub national: Option<String>,
    pub international: Option<String>,
    pub country_calling_code: Option<String>,
    pub country: Option<String>,
    pub national_number: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub location: Option<Json>,
    pub area_code: Option<String>,
    pub exchange: Option<String>,
    pub area_code_region: Option<String>,
}

/// A contact's shared location (Find My-style).
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ContactLocation {
    pub handle: Option<String>,
    pub coordinates: Option<Vec<f64>>,
    pub status: Option<String>,
    pub last_updated: Option<i64>,
}

/// Generic deletion result.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct DeleteResponse {
    pub success: Option<bool>,
    pub deleted_at: Option<i64>,
}

/// Response of `GET /me`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct MeResponse {
    pub auth_type: Option<String>,
    pub valid: Option<bool>,
    pub user_id: Option<String>,
    // Wrapped in `Secret` so the key never appears in `Debug`/tracing output,
    // matching the crate-wide redaction invariant.
    pub api_key: Option<Secret<String>>,
    pub organization_id: Option<String>,
    pub organization: Option<Json>,
    pub metadata: Option<Json>,
    pub integration_details: Option<Json>,
    pub devices: Option<Vec<Json>>,
    pub usage: Option<Json>,
}

/// A contact.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct Contact {
    pub id: Option<String>,
    pub contact_id: Option<String>,
    pub identifier: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub created_at: Option<i64>,
    pub last_message_time: Option<i64>,
    pub tags: Option<Vec<String>>,
}

/// A group chat.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct Group {
    pub group_id: Option<String>,
    pub name: Option<String>,
    pub chat_guid: Option<String>,
    pub icon_url: Option<String>,
    pub member_count: Option<i64>,
    pub message_count: Option<i64>,
    pub last_message_text: Option<String>,
    pub last_message_time: Option<i64>,
    pub last_message_direction: Option<String>,
    pub created_at: Option<i64>,
}

/// A member of a group.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct GroupMember {
    pub id: Option<String>,
    pub contact_id: Option<String>,
    pub identifier: Option<String>,
    pub name: Option<String>,
    pub added_at: Option<i64>,
}

/// Result of setting/removing a group icon.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct GroupIconResponse {
    pub success: Option<bool>,
    pub group_id: Option<String>,
    pub chat_guid: Option<String>,
    pub icon_url: Option<String>,
    pub device_sync: Option<Json>,
    pub message: Option<String>,
}

/// State of a chat's background/wallpaper.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ChatBackgroundResponse {
    pub chat_id: Option<String>,
    pub has_background: Option<bool>,
    pub background_id: Option<String>,
    pub background_version: Option<i64>,
    pub changed: Option<bool>,
}

/// Per-device sync outcome.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct DeviceSyncResult {
    pub chat_guid: Option<String>,
    pub action: Option<String>,
    pub synced: Option<bool>,
    pub error: Option<String>,
}

/// A registered webhook.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct Webhook {
    pub webhook_id: Option<String>,
    pub webhook_url: Option<String>,
    pub webhook_type: Option<String>,
    pub scope: Option<String>,
    pub api_key_name: Option<String>,
    pub integration_name: Option<String>,
    pub created_at: Option<i64>,
    pub deprecated_at: Option<i64>,
    pub valid_until: Option<i64>,
    pub last_triggered: Option<i64>,
    pub failure_count: Option<i64>,
    pub is_active: Option<bool>,
}

/// A single webhook delivery log entry.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct WebhookLog {
    pub event_id: Option<String>,
    pub scope: Option<String>,
    pub attempted_time: Option<i64>,
    pub response_received_at: Option<i64>,
    pub webhook_url: Option<String>,
    pub event_body: Option<WebhookEventPayload>,
    pub response_status: Option<i64>,
    pub response_json: Option<Json>,
    pub metadata: Option<Json>,
}

/// The JSON payload Blooio POSTs to a webhook URL.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct WebhookEventPayload {
    pub event: Option<String>,
    pub message_id: Option<String>,
    pub external_id: Option<String>,
    pub status: Option<String>,
    pub protocol: Option<String>,
    pub timestamp: Option<i64>,
    pub internal_id: Option<String>,
    pub text: Option<String>,
    pub attachments: Option<Vec<Json>>,
    pub is_group: Option<bool>,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub participants: Option<Vec<Json>>,
    pub sender: Option<String>,
    pub sent_at: Option<i64>,
    pub delivered_at: Option<i64>,
    pub read_at: Option<i64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

/// Summary of the most recent message in a chat.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct LastMessage {
    pub message_id: Option<String>,
    pub text: Option<String>,
    pub direction: Option<String>,
    pub time_sent: Option<i64>,
}

/// A chat. The list and detail endpoints return the same shape; detail
/// responses additionally populate `first_message_time`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct Chat {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub is_group: Option<bool>,
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub member_count: Option<i64>,
    pub contact: Option<Json>,
    pub message_count: Option<i64>,
    pub inbound_count: Option<i64>,
    pub outbound_count: Option<i64>,
    pub first_message_time: Option<i64>,
    pub last_message_time: Option<i64>,
    pub last_inbound_time: Option<i64>,
    pub last_outbound_time: Option<i64>,
    pub last_message: Option<LastMessage>,
}

/// A reaction on a message.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct Reaction {
    pub reaction: Option<String>,
    pub is_added: Option<bool>,
    pub time_sent: Option<i64>,
    pub sender: Option<String>,
}

/// A message (list view).
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct Message {
    pub message_id: Option<String>,
    pub direction: Option<String>,
    pub external_id: Option<String>,
    pub internal_id: Option<String>,
    pub text: Option<String>,
    pub attachments: Option<Vec<Json>>,
    pub sender: Option<String>,
    pub reactions: Option<Vec<Reaction>>,
    pub time_sent: Option<i64>,
    pub time_delivered: Option<i64>,
    pub status: Option<String>,
    pub protocol: Option<String>,
    pub error: Option<String>,
}

/// A message (detail view).
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct MessageDetail {
    pub message_id: Option<String>,
    pub chat_id: Option<String>,
    pub direction: Option<String>,
    pub internal_id: Option<String>,
    pub contact: Option<Json>,
    pub sender: Option<String>,
    pub text: Option<String>,
    pub attachments: Option<Vec<Json>>,
    pub reactions: Option<Vec<Reaction>>,
    pub time_sent: Option<i64>,
    pub time_delivered: Option<i64>,
    pub status: Option<String>,
    pub protocol: Option<String>,
    pub error: Option<String>,
}

/// Delivery status of a message.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct MessageStatus {
    pub message_id: Option<String>,
    pub chat_id: Option<String>,
    pub direction: Option<String>,
    pub status: Option<String>,
    pub protocol: Option<String>,
    pub time_sent: Option<i64>,
    pub time_delivered: Option<i64>,
    pub error: Option<String>,
}

/// Result of starting/stopping a typing indicator.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct TypingResponse {
    pub chat_id: Option<String>,
    pub typing: Option<bool>,
    pub started_at: Option<i64>,
    pub stopped_at: Option<i64>,
    pub warning: Option<String>,
}

/// Result of marking a chat read.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ReadResponse {
    pub chat_id: Option<String>,
    pub status: Option<String>,
    pub marked_at: Option<i64>,
}

/// Result of adding a reaction.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ReactionResponse {
    pub success: Option<bool>,
    pub message_id: Option<String>,
    pub reaction: Option<String>,
    pub action: Option<String>,
}

/// Override for a rich link preview on URL messages.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LinkPreview {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Response of `POST /chats/{chatId}/messages`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct SendMessageResponse {
    /// Set when a single message was sent.
    pub message_id: Option<String>,
    /// Set in batch/URL-balloon mode.
    pub message_ids: Option<Vec<String>>,
    pub count: Option<i64>,
    pub status: Option<String>,
    pub group_id: Option<String>,
    pub group_created: Option<bool>,
    pub participants: Option<Vec<String>>,
    pub parent_unresolved: Option<bool>,
}

impl SendMessageResponse {
    /// All message IDs from this response, whether single or batch.
    pub fn ids(&self) -> Vec<&str> {
        if let Some(ids) = &self.message_ids {
            ids.iter().map(String::as_str).collect()
        } else if let Some(id) = &self.message_id {
            vec![id.as_str()]
        } else {
            Vec::new()
        }
    }
}
