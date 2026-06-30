//! [`actix-web`](::actix_web) extractor for verified webhooks.
//!
//! Register a [`WebhookVerifier`] with `App::app_data(...)` and accept
//! [`VerifiedWebhook`] in a handler; the signature is checked and the body
//! parsed before the handler runs.
//!
//! ```no_run
//! # #[cfg(feature = "actix")]
//! # {
//! use actix_web::{web, App, HttpResponse};
//! use blooio::webhook::{VerifiedWebhook, WebhookVerifier};
//!
//! async fn on_event(VerifiedWebhook(event): VerifiedWebhook) -> HttpResponse {
//!     println!("verified event: {:?}", event.kind());
//!     HttpResponse::Ok().finish()
//! }
//!
//! let app = App::new()
//!     .app_data(WebhookVerifier::new("whsec_…"))
//!     .route("/webhooks", web::post().to(on_event));
//! # }
//! ```

use std::future::Future;
use std::pin::Pin;

use ::actix_web::error::InternalError;
use ::actix_web::http::StatusCode;
use ::actix_web::http::header::HeaderMap;
use ::actix_web::{FromRequest, HttpRequest, web};
use futures_util::StreamExt as _;

use crate::webhook::WebhookEvent;
use crate::webhook::server::{
    DEFAULT_SIGNATURE_HEADER, ResolvedWebhook, VerifiedWebhook, WebhookRejection,
    WebhookVerificationResolver, WebhookVerifier, X_BLOOIO_SIGNATURE_HEADER,
};
use crate::webhook::signature::SignatureHeader;

impl FromRequest for VerifiedWebhook {
    type Error = ::actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut ::actix_web::dev::Payload) -> Self::Future {
        // The verifier is read synchronously from app data; clone it into the
        // async body so the future is `'static`.
        let Some(verifier) = req.app_data::<WebhookVerifier>().cloned() else {
            return Box::pin(async { Err(rejection_to_error(WebhookRejection::MissingVerifier)) });
        };
        let Some(signature) = signature_header(
            req.headers(),
            verifier.header_name(),
            verifier.alternate_header_name(),
        ) else {
            return Box::pin(async { Err(rejection_to_error(WebhookRejection::MissingSignature)) });
        };
        let limit = verifier.max_body_bytes();
        if let Err(rejection) = reject_oversize_content_length(req.headers(), limit) {
            return Box::pin(async { Err(rejection_to_error(rejection)) });
        }
        let payload = payload.take();

        Box::pin(async move {
            let body = read_limited_payload(payload, limit)
                .await
                .map_err(rejection_to_error)?;
            match verifier.verify_and_parse(Some(&signature), &body) {
                Ok(event) => Ok(VerifiedWebhook(event)),
                Err(rejection) => Err(rejection_to_error(rejection)),
            }
        })
    }
}

impl<R> FromRequest for ResolvedWebhook<R>
where
    R: WebhookVerificationResolver + Clone + 'static,
    R::Error: From<WebhookRejection> + Into<::actix_web::Error> + 'static,
{
    type Error = R::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut ::actix_web::dev::Payload) -> Self::Future {
        let Some(resolver) = req.app_data::<R>().cloned() else {
            return Box::pin(async { Err(WebhookRejection::MissingVerifier.into()) });
        };
        let Some(signature) = signature_header(
            req.headers(),
            DEFAULT_SIGNATURE_HEADER,
            Some(X_BLOOIO_SIGNATURE_HEADER),
        ) else {
            return Box::pin(async { Err(WebhookRejection::MissingSignature.into()) });
        };
        let signature = match SignatureHeader::parse(&signature) {
            Ok(signature) => signature,
            Err(err) => {
                return Box::pin(
                    async move { Err(WebhookRejection::InvalidSignature(err).into()) },
                );
            }
        };
        if let Err(err) = signature.check_current_tolerance(resolver.tolerance_secs()) {
            return Box::pin(async move { Err(WebhookRejection::InvalidSignature(err).into()) });
        }
        let limit = resolver.max_body_bytes();
        if let Err(rejection) = reject_oversize_content_length(req.headers(), limit) {
            return Box::pin(async move { Err(rejection.into()) });
        }
        let payload = payload.take();

        Box::pin(async move {
            let body = read_limited_payload(payload, limit).await?;
            let context = resolver.verify(&signature, &body).await?;
            let event = WebhookEvent::parse(&body).map_err(WebhookRejection::Malformed)?;
            Ok(ResolvedWebhook { event, context })
        })
    }
}

fn rejection_to_error(rejection: WebhookRejection) -> ::actix_web::Error {
    let status = StatusCode::from_u16(rejection.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
    InternalError::new(rejection, status).into()
}

fn signature_header(headers: &HeaderMap, primary: &str, alternate: Option<&str>) -> Option<String> {
    header_value(headers, primary)
        .or_else(|| alternate.and_then(|name| header_value(headers, name)))
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn reject_oversize_content_length(
    headers: &HeaderMap,
    limit: usize,
) -> Result<(), WebhookRejection> {
    if headers
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|length| length > limit)
    {
        Err(WebhookRejection::PayloadTooLarge { limit })
    } else {
        Ok(())
    }
}

async fn read_limited_payload(
    mut payload: ::actix_web::dev::Payload,
    limit: usize,
) -> Result<web::Bytes, WebhookRejection> {
    let mut bytes = Vec::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk.map_err(|e| WebhookRejection::BodyRead(e.to_string()))?;
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(WebhookRejection::PayloadTooLarge { limit });
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(web::Bytes::from(bytes))
}
