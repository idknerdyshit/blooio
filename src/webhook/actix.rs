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
        let verifier = req.app_data::<WebhookVerifier>().cloned();
        let signature = verifier.as_ref().and_then(|v| {
            signature_header(req.headers(), v.header_name(), v.alternate_header_name())
        });
        let body_fut = web::Bytes::from_request(req, payload);

        Box::pin(async move {
            let Some(verifier) = verifier else {
                return Err(rejection_to_error(WebhookRejection::MissingVerifier));
            };
            let body = body_fut.await?;
            match verifier.verify_and_parse(signature.as_deref(), &body) {
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
        let resolver = req.app_data::<R>().cloned();
        let signature = signature_header(
            req.headers(),
            DEFAULT_SIGNATURE_HEADER,
            Some(X_BLOOIO_SIGNATURE_HEADER),
        );
        let body_fut = web::Bytes::from_request(req, payload);

        Box::pin(async move {
            let Some(resolver) = resolver else {
                return Err(WebhookRejection::MissingVerifier.into());
            };
            let signature = signature.ok_or(WebhookRejection::MissingSignature)?;
            let signature =
                SignatureHeader::parse(&signature).map_err(WebhookRejection::InvalidSignature)?;
            let body = body_fut
                .await
                .map_err(|e| WebhookRejection::BodyRead(e.to_string()))?;
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
