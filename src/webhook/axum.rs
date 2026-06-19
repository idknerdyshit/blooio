//! [`axum`] extractor for verified webhooks.
//!
//! Put a [`WebhookVerifier`] in your router state (or make it reachable via
//! `FromRef`) and accept [`VerifiedWebhook`] in a
//! handler; the signature is checked and the body parsed before the handler
//! runs.
//!
//! ```no_run
//! # #[cfg(feature = "axum")]
//! # {
//! use axum::{routing::post, Router};
//! use blooio::webhook::{VerifiedWebhook, WebhookVerifier};
//!
//! async fn on_event(VerifiedWebhook(event): VerifiedWebhook) {
//!     println!("verified event: {:?}", event.kind());
//! }
//!
//! let app: Router = Router::new()
//!     .route("/webhooks", post(on_event))
//!     .with_state(WebhookVerifier::new("whsec_…"));
//! # }
//! ```

use ::axum::body::Bytes;
use ::axum::extract::{FromRef, FromRequest, Request};
use ::axum::http::StatusCode;
use ::axum::response::{IntoResponse, Response};

use crate::webhook::server::{VerifiedWebhook, WebhookRejection, WebhookVerifier};

impl<S> FromRequest<S> for VerifiedWebhook
where
    WebhookVerifier: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = WebhookRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let verifier = WebhookVerifier::from_ref(state);
        let signature = req
            .headers()
            .get(verifier.header_name())
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let body = Bytes::from_request(req, state)
            .await
            .map_err(|e| WebhookRejection::BodyRead(e.to_string()))?;
        let event = verifier.verify_and_parse(signature.as_deref(), &body)?;
        Ok(VerifiedWebhook(event))
    }
}

impl IntoResponse for WebhookRejection {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
        (status, self.to_string()).into_response()
    }
}
