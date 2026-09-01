#![allow(missing_docs)]

use super::ChannelType;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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
///
/// When two or more parts are image/video URLs, Blooio iMessage sends them as a
/// carousel by default. Use [`MessageContentFields::carousel`] to control that
/// behavior.
#[derive(Debug, Clone, Default, Serialize)]
pub struct MultipartPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Optional attribution badge shown under a Blooio iMessage bubble.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MessageBadge {
    /// Show "Sent with Siri".
    SentWithSiri,
    /// Show "Sent with `FaceTime`".
    SentWithFaceTime,
    /// Preserve a badge value added by a future API revision.
    Unknown(String),
}

impl MessageBadge {
    pub(crate) fn wire_value(&self) -> &str {
        match self {
            Self::SentWithSiri => "sent_with_siri",
            Self::SentWithFaceTime => "sent_with_facetime",
            Self::Unknown(value) => value,
        }
    }
}

impl Serialize for MessageBadge {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.wire_value())
    }
}

impl<'de> Deserialize<'de> for MessageBadge {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "sent_with_siri" => Self::SentWithSiri,
            "sent_with_facetime" => Self::SentWithFaceTime,
            _ => Self::Unknown(value),
        })
    }
}

/// Fields for an App Clip bubble.
#[derive(Debug, Clone, Default, Serialize)]
pub struct AppClipFields {
    /// App Clip launch URL; mutually exclusive with [`Self::bundle_id`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// App Clip bundle identifier used to build Apple's canonical launch URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    /// Optional preview title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

impl AppClipFields {
    /// Build App Clip content from a launch URL.
    #[must_use]
    pub fn url(value: impl Into<String>) -> Self {
        Self {
            url: Some(value.into()),
            ..Default::default()
        }
    }

    /// Build App Clip content from a bundle identifier.
    #[must_use]
    pub fn bundle_id(value: impl Into<String>) -> Self {
        Self {
            bundle_id: Some(value.into()),
            ..Default::default()
        }
    }

    /// Set the optional App Clip preview title.
    #[must_use]
    pub fn title(mut self, value: impl Into<String>) -> Self {
        self.title = Some(value.into());
        self
    }
}

/// Fields for a custom iMessage app-extension bubble.
#[derive(Debug, Clone, Serialize)]
pub struct IMessageAppFields {
    /// Your iMessage app extension's bundle identifier.
    pub bundle_id: String,
    /// Your Apple Developer Team ID.
    pub team_id: String,
    /// App-state URL consumed by the extension on tap.
    pub url: String,
    /// Display name shown for the app.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    /// Fallback template-card caption.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// Optional fallback template-card subcaption.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcaption: Option<String>,
    /// Optional fallback template-card thumbnail URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
    /// Optional App Store Adam ID for the extension fallback.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_store_id: Option<u64>,
}

impl IMessageAppFields {
    /// Build an iMessage app-extension bubble.
    #[must_use]
    pub fn new(
        bundle_id: impl Into<String>,
        team_id: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            bundle_id: bundle_id.into(),
            team_id: team_id.into(),
            url: url.into(),
            app_name: None,
            caption: None,
            subcaption: None,
            image_url: None,
            app_store_id: None,
        }
    }

    /// Set the display name shown for the app.
    #[must_use]
    pub fn app_name(mut self, value: impl Into<String>) -> Self {
        self.app_name = Some(value.into());
        self
    }

    /// Set the fallback template-card caption.
    #[must_use]
    pub fn caption(mut self, value: impl Into<String>) -> Self {
        self.caption = Some(value.into());
        self
    }

    /// Set the fallback template-card subcaption.
    #[must_use]
    pub fn subcaption(mut self, value: impl Into<String>) -> Self {
        self.subcaption = Some(value.into());
        self
    }

    /// Set the fallback template-card thumbnail URL.
    #[must_use]
    pub fn image_url(mut self, value: impl Into<String>) -> Self {
        self.image_url = Some(value.into());
        self
    }

    /// Set the App Store Adam ID used by the extension fallback.
    #[must_use]
    pub fn app_store_id(mut self, value: u64) -> Self {
        self.app_store_id = Some(value);
        self
    }
}

/// How a sent text field is interpreted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MessageFormat {
    /// Send text literally.
    Plain,
    /// Parse the supported iMessage Markdown styling.
    Markdown,
    /// Preserve a future provider-defined format.
    Unknown(String),
}

impl MessageFormat {
    pub(crate) fn wire_value(&self) -> &str {
        match self {
            Self::Plain => "plain",
            Self::Markdown => "markdown",
            Self::Unknown(value) => value,
        }
    }
}

impl Serialize for MessageFormat {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.wire_value())
    }
}

impl<'de> Deserialize<'de> for MessageFormat {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "plain" => Self::Plain,
            "markdown" => Self::Markdown,
            _ => Self::Unknown(value),
        })
    }
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
    pub formatted_text: Option<String>,
    pub status: Option<String>,
    pub provider_message_id: Option<String>,
    pub reply_to_message_id: Option<String>,
    pub error: Option<BTreeMap<String, Value>>,
    #[serde(default)]
    pub attachments: Vec<MessageAttachment>,
    pub interactive: Option<BTreeMap<String, Value>>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// One ordered attachment returned with a message.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MessageAttachment {
    pub url: Option<String>,
    pub media_type: Option<String>,
    pub size: Option<u64>,
    pub caption: Option<String>,
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
    pub format: Option<MessageFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<MultipartPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rich_link: Option<RichLinkFields>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_clip: Option<AppClipFields>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imessage_app: Option<IMessageAppFields>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge: Option<MessageBadge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carousel: Option<bool>,
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

    /// Set how the message text and multipart text parts are interpreted.
    #[must_use]
    pub fn format(mut self, value: MessageFormat) -> Self {
        self.format = Some(value);
        self
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

    /// Build an App Clip bubble.
    #[must_use]
    pub fn app_clip(fields: AppClipFields) -> Self {
        Self {
            app_clip: Some(fields),
            ..Default::default()
        }
    }

    /// Build a custom iMessage app-extension bubble.
    #[must_use]
    pub fn imessage_app(fields: IMessageAppFields) -> Self {
        Self {
            imessage_app: Some(fields),
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

    /// Set an attribution badge on Blooio iMessage channels.
    #[must_use]
    pub fn badge(mut self, value: MessageBadge) -> Self {
        self.badge = Some(value);
        self
    }

    /// Enable or disable Blooio iMessage media carousel grouping.
    #[must_use]
    pub fn carousel(mut self, value: bool) -> Self {
        self.carousel = Some(value);
        self
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
