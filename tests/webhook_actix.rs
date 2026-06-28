//! Integration tests for the actix-web webhook extractor.

#![cfg(feature = "actix")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{App, HttpResponse, test, web};
use blooio::webhook::{
    DEFAULT_TOLERANCE_SECS, ResolvedWebhook, SignatureHeader, VerifiedWebhook, WebhookRejection,
    WebhookVerificationResolver, WebhookVerifier, peek, verify_preparsed,
};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

const SECRET: &str = "whsec_actix_test";

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

async fn handler(VerifiedWebhook(event): VerifiedWebhook) -> HttpResponse {
    HttpResponse::Ok().body(event.payload.message_id.unwrap_or_default())
}

async fn dynamic_handler(
    ResolvedWebhook { event, context }: ResolvedWebhook<DynamicResolver>,
) -> HttpResponse {
    HttpResponse::Ok().body(format!(
        "{}:{}",
        context.org_id,
        event.payload.message_id.unwrap_or_default()
    ))
}

macro_rules! app {
    () => {
        App::new()
            .app_data(WebhookVerifier::new(SECRET))
            .route("/webhooks", web::post().to(handler))
    };
}

macro_rules! dynamic_app {
    () => {
        App::new()
            .app_data(DynamicResolver)
            .route("/webhooks", web::post().to(dynamic_handler))
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
async fn x_blooio_signature_alias_is_accepted() {
    let app = test::init_service(app!()).await;
    let body = br#"{"event":"message.received","message_id":"m_actix_alias"}"#;
    let req = test::TestRequest::post()
        .uri("/webhooks")
        .insert_header(("x-blooio-signature", sign(now(), body)))
        .set_payload(body.to_vec())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 200);
}

#[actix_web::test]
async fn dynamic_resolver_can_verify_and_return_context() {
    let app = test::init_service(dynamic_app!()).await;
    let body = br#"{"event":"message.received","protocol":"sms","message_id":"m_dynamic","sender":"+15550002222","internal_id":"+15550001111","text":"hi"}"#;
    let req = test::TestRequest::post()
        .uri("/webhooks")
        .insert_header(("x-blooio-signature", sign(now(), body)))
        .set_payload(body.to_vec())
        .to_request();
    let resp = test::call_and_read_body(&app, req).await;
    assert_eq!(&resp[..], b"org_1:m_dynamic");
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

impl std::fmt::Display for DynamicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynamicError::Rejection(rejection) => write!(f, "{rejection}"),
            DynamicError::UnknownInternalId => f.write_str("unknown internal_id"),
        }
    }
}

impl actix_web::ResponseError for DynamicError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            DynamicError::Rejection(rejection) => {
                actix_web::http::StatusCode::from_u16(rejection.status_code())
                    .unwrap_or(actix_web::http::StatusCode::BAD_REQUEST)
            }
            DynamicError::UnknownInternalId => actix_web::http::StatusCode::NO_CONTENT,
        }
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
