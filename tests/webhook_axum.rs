//! Integration tests for the axum webhook extractor.

#![cfg(feature = "axum")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::post;
use blooio::webhook::{VerifiedWebhook, WebhookVerifier};
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

fn app() -> Router {
    Router::new()
        .route("/webhooks", post(handler))
        .with_state(WebhookVerifier::new(SECRET))
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
