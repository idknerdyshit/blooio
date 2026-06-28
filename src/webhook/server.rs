//! Framework-agnostic glue for verifying inbound webhooks inside an HTTP
//! server, plus the shared [`VerifiedWebhook`] extractor payload.
//!
//! The actual web-framework extractors live in [`super::axum`] and
//! [`super::actix`] (each behind its own feature) and are thin wrappers around
//! [`WebhookVerifier::verify_and_parse`].

use std::borrow::Cow;
use std::future::Future;

use crate::error::Error;
use crate::secret::Secret;
use crate::webhook::WebhookEvent;
use crate::webhook::signature::{self, DEFAULT_TOLERANCE_SECS, SignatureHeader, VerifyError};

/// Default header name carrying the `t=…,v1=…` signature.
pub const DEFAULT_SIGNATURE_HEADER: &str = "Blooio-Signature";

/// Alternate signature header name used by Blooio webhook deliveries.
pub const X_BLOOIO_SIGNATURE_HEADER: &str = "x-blooio-signature";

/// Holds the webhook signing secret (and verification options) so that the
/// framework extractors can authenticate and parse an inbound request in one
/// step. Cheap to clone; store it in your framework's application state.
#[derive(Clone)]
pub struct WebhookVerifier {
    secret: Secret<String>,
    tolerance: u64,
    header_name: Cow<'static, str>,
}

impl std::fmt::Debug for WebhookVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the secret.
        f.debug_struct("WebhookVerifier")
            .field("tolerance", &self.tolerance)
            .field("header_name", &self.header_name)
            .finish_non_exhaustive()
    }
}

impl WebhookVerifier {
    /// Build a verifier from a signing secret, using the default tolerance and
    /// [`DEFAULT_SIGNATURE_HEADER`].
    pub fn new(secret: impl Into<Secret<String>>) -> Self {
        WebhookVerifier {
            secret: secret.into(),
            tolerance: DEFAULT_TOLERANCE_SECS,
            header_name: Cow::Borrowed(DEFAULT_SIGNATURE_HEADER),
        }
    }

    /// Override the replay-protection tolerance window, in seconds.
    #[must_use]
    pub fn with_tolerance(mut self, tolerance_secs: u64) -> Self {
        self.tolerance = tolerance_secs;
        self
    }

    /// Override the request header the signature is read from.
    #[must_use]
    pub fn with_header_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.header_name = name.into();
        self
    }

    /// The header name this verifier reads the signature from.
    pub fn header_name(&self) -> &str {
        &self.header_name
    }

    /// The alternate header name accepted by default extractors, if any.
    ///
    /// When the verifier uses the default [`DEFAULT_SIGNATURE_HEADER`],
    /// extractors also accept [`X_BLOOIO_SIGNATURE_HEADER`]. Custom header
    /// names are treated as exact overrides.
    pub fn alternate_header_name(&self) -> Option<&'static str> {
        self.header_name
            .eq_ignore_ascii_case(DEFAULT_SIGNATURE_HEADER)
            .then_some(X_BLOOIO_SIGNATURE_HEADER)
    }

    /// Verify a signature header against `body` and, on success, parse the body
    /// into a [`WebhookEvent`].
    ///
    /// `signature_header` is the raw header value (`None` if the header was
    /// absent). The body is verified *before* it is parsed, so a returned
    /// event is always authentic.
    pub fn verify_and_parse(
        &self,
        signature_header: Option<&str>,
        body: &[u8],
    ) -> Result<WebhookEvent, WebhookRejection> {
        let sig = signature_header.ok_or(WebhookRejection::MissingSignature)?;
        signature::verify(self.secret.expose().as_bytes(), sig, body, self.tolerance)
            .map_err(WebhookRejection::InvalidSignature)?;
        WebhookEvent::parse(body).map_err(WebhookRejection::Malformed)
    }
}

/// Application-owned dynamic webhook verifier.
///
/// Implement this when the signing secret must be selected from information in
/// the request body. The resolver receives the parsed signature and raw body,
/// so it can check timestamp tolerance, peek at untrusted routing fields, look
/// up the correct secret, verify with
/// [`verify_preparsed`](crate::webhook::verify_preparsed), and return any
/// application context that should reach the handler.
pub trait WebhookVerificationResolver {
    /// Context returned to the handler after verification succeeds.
    type Context;
    /// Application error/rejection type.
    type Error;
    /// Future returned by [`verify`](Self::verify).
    type Future<'a>: Future<Output = std::result::Result<Self::Context, Self::Error>> + Send + 'a
    where
        Self: 'a;

    /// Verify this request and return handler context.
    fn verify<'a>(&'a self, signature: &'a SignatureHeader, raw_body: &'a [u8])
    -> Self::Future<'a>;
}

/// The successfully verified and parsed webhook plus resolver-provided
/// application context.
#[derive(Debug, Clone)]
pub struct ResolvedWebhook<R: WebhookVerificationResolver> {
    /// The parsed webhook event.
    pub event: WebhookEvent,
    /// Context returned by the resolver.
    pub context: R::Context,
}

/// The successfully verified and parsed webhook, produced by the framework
/// extractors. Destructure it to reach the inner [`WebhookEvent`].
#[derive(Debug, Clone)]
pub struct VerifiedWebhook(pub WebhookEvent);

/// Why an inbound webhook could not be accepted. Each variant maps to an HTTP
/// status via [`status_code`](WebhookRejection::status_code).
#[derive(Debug)]
#[non_exhaustive]
pub enum WebhookRejection {
    /// The framework extractor was used without registering a verifier or
    /// resolver in application state.
    MissingVerifier,
    /// The signature header was absent.
    MissingSignature,
    /// The signature was present but did not verify (bad HMAC, malformed
    /// header, or a timestamp outside the tolerance window).
    InvalidSignature(VerifyError),
    /// The signature verified but the body could not be parsed.
    Malformed(Error),
    /// The request body could not be read from the connection.
    BodyRead(String),
}

impl WebhookRejection {
    /// The HTTP status code an extractor should respond with: `401` for
    /// missing/invalid signatures, `400` for an unreadable or unparseable body.
    pub fn status_code(&self) -> u16 {
        match self {
            WebhookRejection::MissingVerifier => 500,
            WebhookRejection::MissingSignature | WebhookRejection::InvalidSignature(_) => 401,
            WebhookRejection::Malformed(_) | WebhookRejection::BodyRead(_) => 400,
        }
    }
}

impl std::fmt::Display for WebhookRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebhookRejection::MissingVerifier => f.write_str("webhook verifier not configured"),
            WebhookRejection::MissingSignature => f.write_str("missing webhook signature header"),
            WebhookRejection::InvalidSignature(e) => write!(f, "invalid webhook signature: {e}"),
            WebhookRejection::Malformed(e) => write!(f, "malformed webhook body: {e}"),
            WebhookRejection::BodyRead(e) => write!(f, "could not read webhook body: {e}"),
        }
    }
}

impl std::error::Error for WebhookRejection {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WebhookRejection::MissingVerifier
            | WebhookRejection::MissingSignature
            | WebhookRejection::BodyRead(_) => None,
            WebhookRejection::InvalidSignature(e) => Some(e),
            WebhookRejection::Malformed(e) => Some(e),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout
)]
mod tests {
    use super::*;
    use crate::webhook::MessageEventKind;

    const SECRET: &str = "whsec_test_secret";

    fn sign(timestamp: i64, body: &[u8]) -> String {
        use hmac::{Hmac, KeyInit, Mac};
        use sha2::Sha256;
        let mut mac = <Hmac<Sha256>>::new_from_slice(SECRET.as_bytes()).unwrap();
        mac.update(timestamp.to_string().as_bytes());
        mac.update(b".");
        mac.update(body);
        format!(
            "t={timestamp},v1={}",
            hex::encode(mac.finalize().into_bytes())
        )
    }

    #[test]
    fn missing_signature_is_rejected() {
        let v = WebhookVerifier::new(SECRET);
        let err = v.verify_and_parse(None, b"{}").unwrap_err();
        assert!(matches!(err, WebhookRejection::MissingSignature));
        assert_eq!(err.status_code(), 401);
    }

    #[test]
    fn bad_signature_is_rejected() {
        let v = WebhookVerifier::new(SECRET);
        let err = v
            .verify_and_parse(Some("t=1700000000,v1=deadbeef"), b"{}")
            .unwrap_err();
        assert!(matches!(err, WebhookRejection::InvalidSignature(_)));
        assert_eq!(err.status_code(), 401);
    }

    #[test]
    fn valid_signature_parses_event() {
        // Use a huge tolerance so the fixed timestamp verifies regardless of now.
        let v = WebhookVerifier::new(SECRET).with_tolerance(u64::MAX);
        let body = br#"{"event":"message.received","message_id":"m1"}"#;
        let header = sign(1_700_000_000, body);
        let ev = v.verify_and_parse(Some(&header), body).unwrap();
        assert_eq!(ev.kind(), Some(MessageEventKind::Received));
        assert_eq!(ev.payload.message_id.as_deref(), Some("m1"));
    }

    #[test]
    fn default_header_name_is_blooio_signature() {
        assert_eq!(
            WebhookVerifier::new(SECRET).header_name(),
            "Blooio-Signature"
        );
        assert_eq!(
            WebhookVerifier::new(SECRET).alternate_header_name(),
            Some("x-blooio-signature")
        );
        assert_eq!(
            WebhookVerifier::new(SECRET)
                .with_header_name("X-Sig")
                .header_name(),
            "X-Sig"
        );
        assert_eq!(
            WebhookVerifier::new(SECRET)
                .with_header_name("X-Sig")
                .alternate_header_name(),
            None
        );
    }

    #[test]
    fn debug_does_not_leak_secret() {
        let dbg = format!("{:?}", WebhookVerifier::new("super-secret"));
        assert!(!dbg.contains("super-secret"), "secret leaked in Debug");
    }
}
