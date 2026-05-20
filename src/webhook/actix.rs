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
use ::actix_web::{FromRequest, HttpRequest, web};

use crate::webhook::server::{VerifiedWebhook, WebhookVerifier};

impl FromRequest for VerifiedWebhook {
    type Error = ::actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut ::actix_web::dev::Payload) -> Self::Future {
        // The verifier is read synchronously from app data; clone it into the
        // async body so the future is `'static`.
        let verifier = req.app_data::<WebhookVerifier>().cloned();
        let signature = verifier.as_ref().and_then(|v| {
            req.headers()
                .get(v.header_name())
                .and_then(|h| h.to_str().ok())
                .map(str::to_owned)
        });
        let body_fut = web::Bytes::from_request(req, payload);

        Box::pin(async move {
            let Some(verifier) = verifier else {
                return Err(InternalError::new(
                    "blooio: WebhookVerifier not registered as app_data",
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
                .into());
            };
            let body = body_fut.await?;
            match verifier.verify_and_parse(signature.as_deref(), &body) {
                Ok(event) => Ok(VerifiedWebhook(event)),
                Err(rejection) => {
                    let status = StatusCode::from_u16(rejection.status_code())
                        .unwrap_or(StatusCode::BAD_REQUEST);
                    Err(InternalError::new(rejection, status).into())
                }
            }
        })
    }
}
