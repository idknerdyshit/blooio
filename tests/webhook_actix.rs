//! Integration tests for the actix-web webhook extractor.

#![cfg(feature = "actix")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{test, web, App, HttpResponse};
use blooio::webhook::{VerifiedWebhook, WebhookVerifier};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

const SECRET: &str = "whsec_actix_test";

fn now() -> i64 {
    i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()).unwrap()
}

fn sign(timestamp: i64, body: &[u8]) -> String {
    let mut mac = <Hmac<Sha256>>::new_from_slice(SECRET.as_bytes()).unwrap();
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    format!("t={timestamp},v1={}", hex::encode(mac.finalize().into_bytes()))
}

async fn handler(VerifiedWebhook(event): VerifiedWebhook) -> HttpResponse {
    HttpResponse::Ok().body(event.payload.message_id.unwrap_or_default())
}

macro_rules! app {
    () => {
        App::new()
            .app_data(WebhookVerifier::new(SECRET))
            .route("/webhooks", web::post().to(handler))
    };
}

#[actix_web::test]
async fn valid_signature_is_accepted_and_parsed() {
    let app = test::init_service(app!()).await;
    let body = br#"{"event":"message.received","message_id":"m_actix"}"#;
    let req = test::TestRequest::post()
        .uri("/webhooks")
        .insert_header(("Blooio-Signature", sign(now(), body)))
        .set_payload(body.to_vec())
        .to_request();
    let resp = test::call_and_read_body(&app, req).await;
    assert_eq!(&resp[..], b"m_actix");
}

#[actix_web::test]
async fn missing_signature_is_unauthorized() {
    let app = test::init_service(app!()).await;
    let req = test::TestRequest::post()
        .uri("/webhooks")
        .set_payload(&br#"{"event":"message.received"}"#[..])
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
}

#[actix_web::test]
async fn bad_signature_is_unauthorized() {
    let app = test::init_service(app!()).await;
    let req = test::TestRequest::post()
        .uri("/webhooks")
        .insert_header(("Blooio-Signature", "t=1700000000,v1=deadbeef"))
        .set_payload(&br#"{"event":"message.received"}"#[..])
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
}
