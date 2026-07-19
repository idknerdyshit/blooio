#![allow(missing_docs)]

use super::ChannelType;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Optional preview metadata for text messages.
#[derive(Debug, Clone, Default, Serialize)]
pub struct LinkPreview {
    #[serde(rename = "imageUrl", skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// One ordered part in multipart message content.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MultipartPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// A v4 message.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub chat_id: Option<String>,
    pub channel_id: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub protocol: Option<String>,
    pub direction: Option<String>,
    #[serde(rename = "type")]
    pub content_type: Option<String>,
    pub text: Option<String>,
    pub status: Option<String>,
    pub provider_message_id: Option<String>,
    pub reply_to_message_id: Option<String>,
    pub error: Option<BTreeMap<String, Value>>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Per-recipient results returned by a multi-recipient send.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FanOutResult {
    pub data: Vec<MessageSendResult>,
    pub fan_out: Option<bool>,
    pub sent: Option<u32>,
    pub failed: Option<u32>,
    pub dry_run: Option<bool>,
    pub routing: Option<RoutingMetadata>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Sender-selection details returned by send endpoints.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RoutingMetadata {
    pub mode: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub number: Option<String>,
    pub sender_key: Option<String>,
    pub priority_id: Option<String>,
    pub priority: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Recommended fallback for a failed or constrained send.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MessageFallback {
    pub recommended: Option<bool>,
    pub reason: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Unwrapped result returned by v4 send operations.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MessageSendResult {
    /// A multi-recipient send that fanned out to individual messages.
    FanOut(Box<FanOutResult>),
    /// A single accepted message or dry-run preview.
    Message(Box<MessageSendDetails>),
}

/// Detailed result for a single recipient send.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MessageSendDetails {
    pub id: Option<String>,
    pub chat_id: Option<String>,
    pub channel_id: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub protocol: Option<String>,
    pub direction: Option<String>,
    #[serde(rename = "type")]
    pub content_type: Option<String>,
    pub status: Option<String>,
    pub group_id: Option<String>,
    #[serde(default)]
    pub hybrid: BTreeMap<String, Value>,
    pub error: Option<BTreeMap<String, Value>>,
    pub fallback: Option<MessageFallback>,
    pub to: Option<String>,
    pub dry_run: Option<bool>,
    pub would_send: Option<bool>,
    #[serde(default)]
    pub preview: BTreeMap<String, Value>,
    pub poll: Option<super::PollContent>,
    pub routing: Option<RoutingMetadata>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A message lifecycle event.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MessageEvent {
    pub id: Option<String>,
    pub message_id: Option<String>,
    pub kind: Option<String>,
    pub occurred_at: Option<i64>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

/// Current lifecycle state returned by the message-status endpoint.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MessageStatus {
    pub id: Option<String>,
    pub status: Option<String>,
    pub protocol: Option<String>,
    pub error: Option<BTreeMap<String, Value>>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Result after adding a reaction to a message.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReactionResult {
    pub message_id: Option<String>,
    pub reaction: Option<String>,
    pub at: Option<i64>,
}

/// Typed recipient selector for a global send.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Recipient {
    /// One phone number or email address.
    Identifier { identifier: String },
    /// Existing v4 contact.
    Contact { contact_id: String },
    /// Existing v4 group.
    Group { group_id: String },
    /// Multiple phone numbers or email addresses.
    Identifiers { identifiers: Vec<String> },
    /// A raw list of identifiers accepted by the v4 wire format.
    List(Vec<String>),
    /// A raw identifier accepted by the v4 wire format.
    Raw(String),
}

/// Typed selector for the sending channel or priority.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SenderSelector {
    /// Select an exact technical channel.
    Channel { id: String },
    /// Select a hybrid sender by representative number.
    Hybrid {
        #[serde(rename = "type")]
        kind: String,
        number: String,
    },
    /// Select a numbered channel of a given type.
    Number {
        #[serde(rename = "type")]
        channel_type: ChannelType,
        number: String,
    },
    /// Select a non-numbered sender key.
    SenderKey {
        #[serde(rename = "type")]
        channel_type: ChannelType,
        sender_key: String,
    },
    /// Filter an optional priority by channel type.
    ChannelType {
        #[serde(rename = "type")]
        channel_type: ChannelType,
        #[serde(skip_serializing_if = "Option::is_none")]
        priority_id: Option<String>,
    },
    /// Select an explicit priority.
    Priority { priority_id: String },
}

impl SenderSelector {
    /// Select an exact technical channel id.
    #[must_use]
    pub fn channel(id: impl Into<String>) -> Self {
        Self::Channel { id: id.into() }
    }

    /// Select the hybrid sender associated with a number.
    #[must_use]
    pub fn hybrid(number: impl Into<String>) -> Self {
        Self::Hybrid {
            kind: "hybrid".to_owned(),
            number: number.into(),
        }
    }

    /// Select an explicit priority id.
    #[must_use]
    pub fn priority(priority_id: impl Into<String>) -> Self {
        Self::Priority {
            priority_id: priority_id.into(),
        }
    }
}

impl Recipient {
    /// Address one phone number or email.
    #[must_use]
    pub fn identifier(value: impl Into<String>) -> Self {
        Self::Identifier {
            identifier: value.into(),
        }
    }
    /// Address a v4 contact.
    #[must_use]
    pub fn contact(value: impl Into<String>) -> Self {
        Self::Contact {
            contact_id: value.into(),
        }
    }
    /// Address a v4 group.
    #[must_use]
    pub fn group(value: impl Into<String>) -> Self {
        Self::Group {
            group_id: value.into(),
        }
    }
}

/// Typed v4 message content.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContent {
    /// Plain text content.
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        reply_to: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        effect: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        link_preview: Option<LinkPreview>,
    },
    /// One or more public media URLs.
    Media {
        attachments: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reply_to: Option<String>,
    },
    /// Ordered text and media parts.
    Multipart {
        parts: Vec<MultipartPart>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reply_to: Option<String>,
    },
    /// A rich link card.
    RichLink {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        reply_to: Option<String>,
    },
    /// `WhatsApp` Business or RCS interactive content.
    Interactive { kind: String },
    /// Poll content.
    Poll { title: String, options: Vec<String> },
}

impl MessageContent {
    /// Build plain text content.
    #[must_use]
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text {
            text: value.into(),
            reply_to: None,
            effect: None,
            link_preview: None,
        }
    }
    /// Build media content from public URLs.
    #[must_use]
    pub fn media(attachments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Media {
            attachments: attachments.into_iter().map(Into::into).collect(),
            caption: None,
            reply_to: None,
        }
    }
    /// Build a rich link.
    #[must_use]
    pub fn rich_link(url: impl Into<String>) -> Self {
        Self::RichLink {
            url: url.into(),
            title: None,
            reply_to: None,
        }
    }
    /// Build poll content.
    #[must_use]
    pub fn poll(
        title: impl Into<String>,
        options: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::Poll {
            title: title.into(),
            options: options.into_iter().map(Into::into).collect(),
        }
    }
}
