//! Webhook payload types and signature verification.
//!
//! This module is framework-agnostic: it does not run a web server. Parse the
//! POST body with [`WebhookEvent::parse`] and verify authenticity with
//! [`signature::verify`].

pub mod signature;

#[cfg(any(feature = "axum", feature = "actix"))]
pub mod server;

#[cfg(feature = "actix")]
pub mod actix;
#[cfg(feature = "axum")]
pub mod axum;

pub use signature::{
    DEFAULT_TOLERANCE_SECS, SignatureHeader, VerifyError, verify, verify_at, verify_default,
    verify_preparsed,
};

#[cfg(any(feature = "axum", feature = "actix"))]
pub use server::{
    DEFAULT_MAX_WEBHOOK_BODY_BYTES, DEFAULT_SIGNATURE_HEADER, ResolvedWebhook, VerifiedWebhook,
    WebhookRejection, WebhookVerificationResolver, WebhookVerifier, X_BLOOIO_SIGNATURE_HEADER,
};

use serde::Deserialize;

use crate::error::{Error, Result};
pub use crate::types::WebhookEventPayload;

/// Untrusted routing fields extracted from a raw webhook body.
///
/// Use this only to decide how to find the correct signing secret; verify the
/// body before trusting the values.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct WebhookPeek {
    /// The raw event name, such as `message.received`.
    pub event: Option<String>,
    /// The message protocol, such as `sms`.
    pub protocol: Option<String>,
    /// The provider message id.
    pub message_id: Option<String>,
    /// The receiving phone number or internal identifier.
    pub internal_id: Option<String>,
}

/// A verified inbound SMS event projected into the fields most handlers need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedSms {
    /// The provider message id.
    pub message_id: String,
    /// The sender phone number or handle.
    pub sender: String,
    /// The receiving phone number or internal identifier.
    pub internal_id: String,
    /// The inbound text. Missing/null text is represented as an empty string.
    pub text: String,
}

/// Why a parsed webhook could not be converted into a received SMS.
#[allow(missing_docs)]
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum WebhookConversionError {
    #[error("webhook event is not message.received")]
    WrongEvent { event: Option<String> },
    #[error("webhook protocol is not sms")]
    WrongProtocol { protocol: Option<String> },
    #[error("webhook payload is missing required field {0}")]
    MissingField(&'static str),
}

/// Extract untrusted routing fields from a raw webhook body.
///
/// This is intentionally a "peek": use it before verification only to choose
/// which signing secret to verify with.
pub fn peek(raw_body: &[u8]) -> Result<WebhookPeek> {
    serde_json::from_slice(raw_body).map_err(Error::decode)
}

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
    /// Classify a raw `event` string (matched case-insensitively).
    pub fn from_event(event: &str) -> Self {
        let eq = |name: &str| event.eq_ignore_ascii_case(name);
        if eq("message.received") {
            MessageEventKind::Received
        } else if eq("message.delivered") {
            MessageEventKind::Delivered
        } else if eq("message.failed") {
            MessageEventKind::Failed
        } else if eq("message.read") {
            MessageEventKind::Read
        } else if eq("message.sent") {
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

    /// Convert this event into a received SMS payload.
    ///
    /// The conversion requires a case-insensitive `message.received` event and
    /// `sms` protocol. `message_id`, `sender`, and `internal_id` must be
    /// present and non-empty; `text` defaults to an empty string.
    pub fn try_into_received_sms(self) -> std::result::Result<ReceivedSms, WebhookConversionError> {
        let payload = self.payload;
        let event = payload.event;
        if !event
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("message.received"))
        {
            return Err(WebhookConversionError::WrongEvent { event });
        }

        let protocol = payload.protocol;
        if !protocol
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("sms"))
        {
            return Err(WebhookConversionError::WrongProtocol { protocol });
        }

        Ok(ReceivedSms {
            message_id: required(payload.message_id, "message_id")?,
            sender: required(payload.sender, "sender")?,
            internal_id: required(payload.internal_id, "internal_id")?,
            text: payload.text.unwrap_or_default(),
        })
    }
}

fn required(
    value: Option<String>,
    field: &'static str,
) -> std::result::Result<String, WebhookConversionError> {
    value
        .filter(|value| !value.is_empty())
        .ok_or(WebhookConversionError::MissingField(field))
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

    #[test]
    fn peek_extracts_routing_fields_only() {
        let raw = br#"{"event":"message.received","protocol":"sms","message_id":"m1","internal_id":"+15550001111","ignored":{"x":1}}"#;
        let peeked = peek(raw).unwrap();
        assert_eq!(
            peeked,
            WebhookPeek {
                event: Some("message.received".into()),
                protocol: Some("sms".into()),
                message_id: Some("m1".into()),
                internal_id: Some("+15550001111".into()),
            }
        );
    }

    #[test]
    fn converts_received_sms_case_insensitively() {
        let raw = br#"{"event":"MESSAGE.RECEIVED","protocol":"SMS","message_id":"m1","sender":"+15550002222","internal_id":"+15550001111","text":"hi"}"#;
        let sms = WebhookEvent::parse(raw)
            .unwrap()
            .try_into_received_sms()
            .unwrap();
        assert_eq!(
            sms,
            ReceivedSms {
                message_id: "m1".into(),
                sender: "+15550002222".into(),
                internal_id: "+15550001111".into(),
                text: "hi".into(),
            }
        );
    }

    #[test]
    fn received_sms_text_defaults_to_empty() {
        let raw = br#"{"event":"message.received","protocol":"sms","message_id":"m1","sender":"+15550002222","internal_id":"+15550001111"}"#;
        let sms = WebhookEvent::parse(raw)
            .unwrap()
            .try_into_received_sms()
            .unwrap();
        assert_eq!(sms.text, "");
    }

    #[test]
    fn received_sms_rejects_wrong_event_and_protocol() {
        let wrong_event = br#"{"event":"message.sent","protocol":"sms","message_id":"m1","sender":"s","internal_id":"i"}"#;
        assert!(matches!(
            WebhookEvent::parse(wrong_event)
                .unwrap()
                .try_into_received_sms(),
            Err(WebhookConversionError::WrongEvent { .. })
        ));

        let wrong_protocol = br#"{"event":"message.received","protocol":"imessage","message_id":"m1","sender":"s","internal_id":"i"}"#;
        assert!(matches!(
            WebhookEvent::parse(wrong_protocol)
                .unwrap()
                .try_into_received_sms(),
            Err(WebhookConversionError::WrongProtocol { .. })
        ));
    }

    #[test]
    fn received_sms_requires_core_fields() {
        let raw =
            br#"{"event":"message.received","protocol":"sms","message_id":"m1","sender":"s"}"#;
        assert_eq!(
            WebhookEvent::parse(raw).unwrap().try_into_received_sms(),
            Err(WebhookConversionError::MissingField("internal_id"))
        );
    }
}
