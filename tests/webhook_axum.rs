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
    DEFAULT_TOLERANCE_SECS, ResolvedWebhook, SignatureHeader, VerifiedWebhook, WebhookRejection,
    WebhookVerificationResolver, WebhookVerifier, peek, verify_preparsed,
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
    UnknownInternalId,
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
            DynamicError::UnknownInternalId => StatusCode::NO_CONTENT.into_response(),
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
            signature
                .check_tolerance(now(), DEFAULT_TOLERANCE_SECS)
                .map_err(WebhookRejection::InvalidSignature)?;
            let peeked = peek(raw_body).map_err(WebhookRejection::Malformed)?;
            if peeked.internal_id.as_deref() != Some("+15550001111") {
                return Err(DynamicError::UnknownInternalId);
            }
            verify_preparsed(SECRET.as_bytes(), signature, raw_body)
                .map_err(WebhookRejection::InvalidSignature)?;
            Ok(DynamicContext { org_id: "org_1" })
        })())
    }
}
