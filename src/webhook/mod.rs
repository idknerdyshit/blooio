//! Webhook payload types and signature verification.
//!
//! This module is framework-agnostic: it does not run a web server. Parse the
//! POST body with [`WebhookEvent::parse`] and verify authenticity with
//! [`signature::verify`].

pub mod signature;

pub use signature::{DEFAULT_TOLERANCE_SECS, VerifyError, verify, verify_at, verify_default};

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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unreadable_literal
)]
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

    #[test]
    fn classifies_each_known_event() {
        assert_eq!(
            MessageEventKind::from_event("message.sent"),
            MessageEventKind::Sent
        );
        assert_eq!(
            MessageEventKind::from_event("message.delivered"),
            MessageEventKind::Delivered
        );
        assert_eq!(
            MessageEventKind::from_event("message.failed"),
            MessageEventKind::Failed
        );
        assert_eq!(
            MessageEventKind::from_event("message.read"),
            MessageEventKind::Read
        );
    }

    #[test]
    fn classification_is_case_insensitive() {
        assert_eq!(
            MessageEventKind::from_event("MESSAGE.DELIVERED"),
            MessageEventKind::Delivered
        );
    }

    #[test]
    fn parses_delivered_payload_fields() {
        let raw = br#"{"event":"message.delivered","message_id":"m2","status":"delivered","delivered_at":1700000000}"#;
        let ev = WebhookEvent::parse(raw).unwrap();
        assert_eq!(ev.kind(), Some(MessageEventKind::Delivered));
        assert_eq!(ev.payload.delivered_at, Some(1700000000));
        assert_eq!(ev.payload.status.as_deref(), Some("delivered"));
    }

    #[test]
    fn parses_failed_payload_with_error_fields() {
        let raw = br#"{"event":"message.failed","message_id":"m3","error_code":"unreachable","error_message":"no service"}"#;
        let ev = WebhookEvent::parse(raw).unwrap();
        assert_eq!(ev.kind(), Some(MessageEventKind::Failed));
        assert_eq!(ev.payload.error_code.as_deref(), Some("unreachable"));
        assert_eq!(ev.payload.error_message.as_deref(), Some("no service"));
    }

    #[test]
    fn parses_group_received_payload() {
        let raw = br#"{"event":"message.received","is_group":true,"group_id":"g1","group_name":"Team","sender":"+15550001111","text":"hi all"}"#;
        let ev = WebhookEvent::parse(raw).unwrap();
        assert_eq!(ev.kind(), Some(MessageEventKind::Received));
        assert_eq!(ev.payload.is_group, Some(true));
        assert_eq!(ev.payload.group_id.as_deref(), Some("g1"));
    }

    #[test]
    fn kind_is_none_when_event_absent() {
        let raw = br#"{"message_id":"m4","text":"no event field"}"#;
        let ev = WebhookEvent::parse(raw).unwrap();
        assert_eq!(ev.kind(), None);
        assert_eq!(ev.payload.message_id.as_deref(), Some("m4"));
    }

    #[test]
    fn parse_rejects_malformed_json() {
        let err = WebhookEvent::parse(b"not json").unwrap_err();
        assert!(matches!(err, Error::Decode(_)));
        // A decode error is not an API error.
        assert_eq!(err.code(), None);
        assert_eq!(err.status(), None);
    }

    #[test]
    fn parse_accepts_unknown_extra_fields() {
        // Forward-compatibility: unknown keys must not break parsing.
        let raw = br#"{"event":"message.received","brand_new_field":{"x":1},"message_id":"m5"}"#;
        let ev = WebhookEvent::parse(raw).unwrap();
        assert_eq!(ev.payload.message_id.as_deref(), Some("m5"));
    }
}
