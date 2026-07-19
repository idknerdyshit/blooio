//! V4 webhook event-envelope parsing with shared signature verification.

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::v4::types::ChannelType;
use crate::{Error, Result};

pub use crate::webhook::{
    DEFAULT_TOLERANCE_SECS, SignatureHeader, VerifyError, verify, verify_at, verify_default,
    verify_preparsed,
};

/// Parsed v4 webhook event envelope.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub message_id: Option<String>,
    pub chat_id: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub occurred_at: i64,
    #[serde(default)]
    pub data: Map<String, Value>,
}

impl WebhookEvent {
    /// Parse an untrusted raw v4 webhook body.
    pub fn parse(raw_body: &[u8]) -> Result<Self> {
        serde_json::from_slice(raw_body).map_err(|error| Error::decode_json::<Self>(&error))
    }
}

/// Untrusted routing fields from a v4 event envelope.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct WebhookPeek {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub message_id: Option<String>,
    pub chat_id: Option<String>,
}

/// Peek at untrusted routing fields before signature verification.
pub fn peek(raw_body: &[u8]) -> Result<WebhookPeek> {
    serde_json::from_slice(raw_body).map_err(|error| Error::decode_json::<WebhookPeek>(&error))
}

/// Verify and parse a v4 event using the default tolerance.
pub fn verify_and_parse(
    secret: &[u8],
    signature_header: &str,
    raw_body: &[u8],
) -> Result<WebhookEvent> {
    verify_default(secret, signature_header, raw_body).map_err(Error::from)?;
    WebhookEvent::parse(raw_body)
}
