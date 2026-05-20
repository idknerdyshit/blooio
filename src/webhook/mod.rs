//! Webhook payload types and signature verification.
//!
//! This module is framework-agnostic: it does not run a web server. Parse the
//! POST body with [`WebhookEvent::parse`] and verify authenticity with
//! [`signature::verify`].

pub mod signature;

pub use signature::{verify, verify_default, verify_at, VerifyError, DEFAULT_TOLERANCE_SECS};

use crate::error::{Error, Result};
pub use crate::types::WebhookEventPayload;

/// The kind of message event, with a raw fallback for forward-compatibility.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageEventKind {
    Received,
    Sent,
    Delivered,
    Failed,
    Read,
    /// An event string this version doesn't recognize.
    Other(String),
}

impl MessageEventKind {
    /// Classify a raw `event` string.
    pub fn from_event(event: &str) -> Self {
        let e = event.to_ascii_lowercase();
        if e.contains("received") {
            MessageEventKind::Received
        } else if e.contains("delivered") {
            MessageEventKind::Delivered
        } else if e.contains("failed") {
            MessageEventKind::Failed
        } else if e.contains("read") {
            MessageEventKind::Read
        } else if e.contains("sent") {
            MessageEventKind::Sent
        } else {
            MessageEventKind::Other(event.to_owned())
        }
    }
}

/// A parsed, typed webhook event.
#[derive(Debug, Clone)]
pub struct WebhookEvent {
    /// The decoded payload.
    pub payload: WebhookEventPayload,
}

impl WebhookEvent {
    /// Parse a raw webhook body. Verify the signature separately, before
    /// trusting the contents.
    pub fn parse(raw_body: &[u8]) -> Result<Self> {
        let payload: WebhookEventPayload =
            serde_json::from_slice(raw_body).map_err(Error::decode)?;
        Ok(WebhookEvent { payload })
    }

    /// The classified event kind, if the payload carried an `event` field.
    pub fn kind(&self) -> Option<MessageEventKind> {
        self.payload
            .event
            .as_deref()
            .map(MessageEventKind::from_event)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::print_stdout, clippy::unreadable_literal)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_classifies() {
        let raw = br#"{"event":"message.received","message_id":"m1","text":"hi"}"#;
        let ev = WebhookEvent::parse(raw).unwrap();
        assert_eq!(ev.kind(), Some(MessageEventKind::Received));
        assert_eq!(ev.payload.message_id.as_deref(), Some("m1"));
    }

    #[test]
    fn unknown_event_falls_back() {
        assert_eq!(
            MessageEventKind::from_event("message.reacted"),
            MessageEventKind::Other("message.reacted".into())
        );
    }
}
