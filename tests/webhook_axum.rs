//! Integration tests for the axum webhook extractor.

#![cfg(feature = "axum")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::Body;
use axum::extract::FromRef;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use blooio::webhook::{
    DEFAULT_MAX_WEBHOOK_BODY_BYTES, ResolvedWebhook, SignatureHeader, VerifiedWebhook, VerifyError,
    WebhookRejection, WebhookVerificationResolver, WebhookVerifier, peek, verify_preparsed,
};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use tower::ServiceExt;

const SECRET: &str = "whsec_axum_test";

fn now() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )
    .unwrap()
}

fn sign(timestamp: i64, body: &[u8]) -> String {
    let mut mac = <Hmac<Sha256>>::new_from_slice(SECRET.as_bytes()).unwrap();
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    format!(
        "t={timestamp},v1={}",
        hex::encode(mac.finalize().into_bytes())
    )
}

async fn handler(VerifiedWebhook(event): VerifiedWebhook) -> String {
    event.payload.message_id.unwrap_or_default()
}

async fn dynamic_handler(
    ResolvedWebhook { event, context }: ResolvedWebhook<DynamicResolver>,
) -> String {
    format!(
        "{}:{}",
        context.org_id,
        event.payload.message_id.unwrap_or_default()
    )
}

fn app() -> Router {
    Router::new()
        .route("/webhooks", post(handler))
        .with_state(WebhookVerifier::new(SECRET))
}

fn dynamic_app() -> Router {
    Router::new()
        .route("/webhooks", post(dynamic_handler))
        .with_state(DynamicState {
            resolver: DynamicResolver,
        })
}

async fn send(headers: &[(&str, &str)], body: &[u8]) -> StatusCode {
    let mut req = Request::builder().method("POST").uri("/webhooks");
    for (k, v) in headers {
        req = req.header(*k, *v);
    }
    let resp = app()
        .oneshot(req.body(Body::from(body.to_vec())).unwrap())
        .await
        .unwrap();
    resp.status()
}

#[tokio::test]
async fn valid_signature_is_accepted_and_parsed() {
    let body = br#"{"event":"message.received","message_id":"m_axum"}"#;
    let header = sign(now(), body);
    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks")
                .header("Blooio-Signature", header)
                .body(Body::from(body.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&bytes[..], b"m_axum");
}

#[tokio::test]
async fn x_blooio_signature_alias_is_accepted() {
    let body = br#"{"event":"message.received","message_id":"m_axum_alias"}"#;
    let header = sign(now(), body);
    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks")
                .header("x-blooio-signature", header)
                .body(Body::from(body.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn dynamic_resolver_can_verify_and_return_context() {
    let body = br#"{"event":"message.received","protocol":"sms","message_id":"m_dynamic","sender":"+15550002222","internal_id":"+15550001111","text":"hi"}"#;
    let header = sign(now(), body);
    let resp = dynamic_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks")
                .header("x-blooio-signature", header)
                .body(Body::from(body.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(&bytes[..], b"org_1:m_dynamic");
}

#[tokio::test]
async fn dynamic_resolver_rejects_expired_signature() {
    let body = br#"{"event":"message.received","protocol":"sms","message_id":"m_dynamic","sender":"+15550002222","internal_id":"+15550001111","text":"hi"}"#;
    let resp = dynamic_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks")
                .header("x-blooio-signature", sign(1, body))
                .body(Body::from(body.to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn missing_signature_is_unauthorized() {
    let status = send(&[], br#"{"event":"message.received"}"#).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn bad_signature_is_unauthorized() {
    let status = send(
        &[("Blooio-Signature", "t=1700000000,v1=deadbeef")],
        br#"{"event":"message.received"}"#,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn oversized_body_is_rejected() {
    let body = vec![b'x'; DEFAULT_MAX_WEBHOOK_BODY_BYTES + 1];
    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/webhooks")
                .header("Blooio-Signature", "t=1700000000,v1=deadbeef")
                .header("content-length", body.len().to_string())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn streaming_body_limit_is_enforced_without_content_length() {
    for (size, expected) in [
        (DEFAULT_MAX_WEBHOOK_BODY_BYTES, StatusCode::UNAUTHORIZED),
        (
            DEFAULT_MAX_WEBHOOK_BODY_BYTES + 1,
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
    ] {
        let body = vec![b'x'; size];
        let resp = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhooks")
                    .header("x-blooio-signature", "t=1700000000,v1=deadbeef")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), expected);
    }
}

#[derive(Clone)]
struct DynamicState {
    resolver: DynamicResolver,
}

#[derive(Clone)]
struct DynamicResolver;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DynamicContext {
    org_id: &'static str,
}

#[derive(Debug)]
enum DynamicError {
    Rejection(WebhookRejection),
}

impl From<WebhookRejection> for DynamicError {
    fn from(value: WebhookRejection) -> Self {
        DynamicError::Rejection(value)
    }
}

impl IntoResponse for DynamicError {
    fn into_response(self) -> Response {
        match self {
            DynamicError::Rejection(rejection) => rejection.into_response(),
        }
    }
}

impl FromRef<DynamicState> for DynamicResolver {
    fn from_ref(state: &DynamicState) -> Self {
        state.resolver.clone()
    }
}

impl WebhookVerificationResolver for DynamicResolver {
    type Context = DynamicContext;
    type Error = DynamicError;
    type Future<'a> = std::future::Ready<Result<DynamicContext, DynamicError>>;

    fn verify<'a>(
        &'a self,
        signature: &'a SignatureHeader,
        raw_body: &'a [u8],
    ) -> Self::Future<'a> {
        std::future::ready((|| {
            let peeked = peek(raw_body).map_err(WebhookRejection::Malformed)?;
            let known_identifier = peeked.internal_id.as_deref() == Some("+15550001111");
            let secret = if known_identifier {
                SECRET.as_bytes()
            } else {
                b"non-production-dummy-secret"
            };
            let verification = verify_preparsed(secret, signature, raw_body);
            if !known_identifier {
                return Err(WebhookRejection::InvalidSignature(VerifyError::Mismatch).into());
            }
            verification.map_err(WebhookRejection::InvalidSignature)?;
            Ok(DynamicContext { org_id: "org_1" })
        })())
    }
}

#[tokio::test]
async fn dynamic_resolver_hides_identifier_existence() {
    let known = br#"{"event":"message.received","internal_id":"+15550001111"}"#;
    let unknown = br#"{"event":"message.received","internal_id":"+15550009999"}"#;
    let timestamp = now();
    let mut responses = Vec::new();
    for body in [known.as_slice(), unknown.as_slice()] {
        let response = dynamic_app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/webhooks")
                    .header("x-blooio-signature", format!("t={timestamp},v1=deadbeef"))
                    .body(Body::from(body.to_vec()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        responses.push((status, body));
    }
    assert_eq!(responses[0], responses[1]);
    assert_eq!(responses[0].0, StatusCode::UNAUTHORIZED);
}
