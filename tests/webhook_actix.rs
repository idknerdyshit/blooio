//! Integration tests for the actix-web webhook extractor.

#![cfg(feature = "actix")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{App, HttpResponse, test, web};
use blooio::webhook::{
    DEFAULT_MAX_WEBHOOK_BODY_BYTES, ResolvedWebhook, SignatureHeader, VerifiedWebhook, VerifyError,
    WebhookRejection, WebhookVerificationResolver, WebhookVerifier, peek, verify_preparsed,
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
async fn dynamic_resolver_rejects_expired_signature() {
    let app = test::init_service(dynamic_app!()).await;
    let body = br#"{"event":"message.received","protocol":"sms","message_id":"m_dynamic","sender":"+15550002222","internal_id":"+15550001111","text":"hi"}"#;
    let req = test::TestRequest::post()
        .uri("/webhooks")
        .insert_header(("x-blooio-signature", sign(1, body)))
        .set_payload(body.to_vec())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401);
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
async fn oversized_body_is_rejected() {
    let app = test::init_service(app!()).await;
    let body = vec![b'x'; DEFAULT_MAX_WEBHOOK_BODY_BYTES + 1];
    let req = test::TestRequest::post()
        .uri("/webhooks")
        .insert_header(("Blooio-Signature", "t=1700000000,v1=deadbeef"))
        .insert_header(("content-length", body.len().to_string()))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 413);
}

#[actix_web::test]
async fn streaming_body_limit_is_enforced_without_content_length() {
    let app = test::init_service(app!()).await;
    for (size, expected) in [
        (DEFAULT_MAX_WEBHOOK_BODY_BYTES, 401),
        (DEFAULT_MAX_WEBHOOK_BODY_BYTES + 1, 413),
    ] {
        let body = vec![b'x'; size];
        let mut req = test::TestRequest::post()
            .uri("/webhooks")
            .insert_header(("x-blooio-signature", "t=1700000000,v1=deadbeef"))
            .set_payload(body)
            .to_request();
        req.headers_mut().remove("content-length");
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status().as_u16(), expected);
    }
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

impl std::fmt::Display for DynamicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynamicError::Rejection(rejection) => write!(f, "{rejection}"),
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

#[actix_web::test]
async fn dynamic_resolver_hides_identifier_existence() {
    let app = test::init_service(dynamic_app!()).await;
    let known = br#"{"event":"message.received","internal_id":"+15550001111"}"#;
    let unknown = br#"{"event":"message.received","internal_id":"+15550009999"}"#;
    let timestamp = now();
    let mut responses = Vec::new();
    for body in [known.as_slice(), unknown.as_slice()] {
        let request = test::TestRequest::post()
            .uri("/webhooks")
            .insert_header(("x-blooio-signature", format!("t={timestamp},v1=deadbeef")))
            .set_payload(body)
            .to_request();
        let response = test::call_service(&app, request).await;
        let status = response.status();
        let body = test::read_body(response).await;
        responses.push((status, body));
    }
    assert_eq!(responses[0], responses[1]);
    assert_eq!(responses[0].0.as_u16(), 401);
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
