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
    pub alias: Option<String>,
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
    pub from: Option<String>,
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
    Identifier(String),
    /// Existing v4 contact.
    Contact { contact_id: String },
    /// Existing v4 group.
    Group { group_id: String },
    /// Multiple phone numbers or email addresses.
    Identifiers(Vec<String>),
    /// A raw list of identifiers accepted by the v4 wire format.
    List(Vec<String>),
    /// A raw identifier accepted by the v4 wire format.
    Raw(String),
}

/// Hybrid sender: either `true`/`false` or a phone number string.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Hybrid {
    /// Boolean on/off flag.
    On(bool),
    /// A representative phone number string.
    Number(String),
}

/// Rich link card fields.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RichLinkFields {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Interactive content wrapper for `WhatsApp` Business / RCS.
#[derive(Debug, Clone, Default, Serialize)]
pub struct InteractiveFields {
    pub kind: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Template content placeholder for `WhatsApp` Business.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TemplateFields {
    /// The template identifier returned by the template-creation endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Flat message content body matching the server's untagged request format.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MessageContentFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<MultipartPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rich_link: Option<RichLinkFields>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<super::PollContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactive: Option<InteractiveFields>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateFields>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_preview: Option<LinkPreview>,
}

impl MessageContentFields {
    /// Build plain text content.
    #[must_use]
    pub fn text(value: impl Into<String>) -> Self {
        Self {
            text: Some(value.into()),
            ..Default::default()
        }
    }

    /// Build media content from public URLs.
    ///
    /// Requires at least one URL; the server rejects empty attachment arrays with `422`.
    ///
    /// # Panics
    ///
    /// Panics if `attachments` is empty.
    #[must_use]
    pub fn media(attachments: &[impl AsRef<str>]) -> Self {
        assert!(!attachments.is_empty(), "media requires at least one URL");
        Self {
            attachments: Some(attachments.iter().map(|s| s.as_ref().to_owned()).collect()),
            ..Default::default()
        }
    }

    /// Build a rich link card with an optional title.
    #[must_use]
    pub fn rich_link(url: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            rich_link: Some(RichLinkFields {
                url: url.into(),
                title: Some(title.into()),
            }),
            ..Default::default()
        }
    }

    /// Build poll content.
    ///
    /// Requires at least two options; the server rejects fewer with `422`.
    ///
    /// # Panics
    ///
    /// Panics if `options` has fewer than two items.
    #[must_use]
    pub fn poll(title: impl Into<String>, options: &[impl AsRef<str>]) -> Self {
        assert!(options.len() >= 2, "poll requires at least two options");
        Self {
            poll: Some(super::PollContent {
                title: Some(title.into()),
                options: options.iter().map(|s| s.as_ref().to_owned()).collect(),
            }),
            ..Default::default()
        }
    }

    /// Build template content for `WhatsApp` Business.
    #[must_use]
    pub fn template(template_id: impl Into<String>) -> Self {
        Self {
            template: Some(TemplateFields {
                template_id: Some(template_id.into()),
                extra: BTreeMap::new(),
            }),
            ..Default::default()
        }
    }

    /// Well-known interactive content kinds for `WhatsApp` Business / RCS.
    ///
    /// `kind` values accepted by the server:
    /// - `"carousel"` — a scrollable card carousel
    /// - `"product"` — a single product card
    /// - `"product_list"` — a product catalog list
    pub const INTERACTIVE_KIND_CAROUSEL: &'static str = "carousel";
    pub const INTERACTIVE_KIND_PRODUCT: &'static str = "product";
    pub const INTERACTIVE_KIND_PRODUCT_LIST: &'static str = "product_list";

    /// Build interactive content for `WhatsApp` Business / RCS.
    ///
    /// Use one of the `INTERACTIVE_KIND_*` constants for well-known kinds,
    /// or pass a custom string for provider-specific values.
    #[must_use]
    pub fn interactive(kind: impl Into<String>) -> Self {
        Self {
            interactive: Some(InteractiveFields {
                kind: kind.into(),
                extra: BTreeMap::new(),
            }),
            ..Default::default()
        }
    }
}

impl Recipient {
    /// Address one phone number or email.
    #[must_use]
    pub fn identifier(value: impl Into<String>) -> Self {
        Self::Identifier(value.into())
    }
    /// Address multiple phone numbers or emails.
    #[must_use]
    pub fn identifiers(values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::Identifiers(values.into_iter().map(Into::into).collect())
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
