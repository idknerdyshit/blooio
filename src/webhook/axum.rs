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

use ::axum::body::{Body, Bytes};
use ::axum::extract::{FromRef, FromRequest, Request};
use ::axum::http::{HeaderMap, StatusCode};
use ::axum::response::{IntoResponse, Response};
use futures_util::StreamExt as _;

use crate::webhook::WebhookEvent;
use crate::webhook::server::{
    DEFAULT_SIGNATURE_HEADER, ResolvedWebhook, VerifiedWebhook, WebhookRejection,
    WebhookVerificationResolver, WebhookVerifier, X_BLOOIO_SIGNATURE_HEADER,
};
use crate::webhook::signature::SignatureHeader;

impl<S> FromRequest<S> for VerifiedWebhook
where
    WebhookVerifier: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = WebhookRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let verifier = WebhookVerifier::from_ref(state);
        let signature = signature_header(
            req.headers(),
            verifier.header_name(),
            verifier.alternate_header_name(),
        )
        .ok_or(WebhookRejection::MissingSignature)?;
        let limit = verifier.max_body_bytes();
        reject_oversize_content_length(req.headers(), limit)?;
        let body = read_limited_body(req.into_body(), limit).await?;
        let event = verifier.verify_and_parse(Some(&signature), &body)?;
        Ok(VerifiedWebhook(event))
    }
}

impl<S, R> FromRequest<S> for ResolvedWebhook<R>
where
    R: WebhookVerificationResolver + FromRef<S> + Send + Sync,
    R::Error: From<WebhookRejection> + IntoResponse,
    S: Send + Sync,
{
    type Rejection = R::Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let signature = signature_header(
            req.headers(),
            DEFAULT_SIGNATURE_HEADER,
            Some(X_BLOOIO_SIGNATURE_HEADER),
        )
        .ok_or(WebhookRejection::MissingSignature)?;
        let signature =
            SignatureHeader::parse(&signature).map_err(WebhookRejection::InvalidSignature)?;
        let resolver = R::from_ref(state);
        signature
            .check_current_tolerance(resolver.tolerance_secs())
            .map_err(WebhookRejection::InvalidSignature)
            .map_err(R::Error::from)?;
        let limit = resolver.max_body_bytes();
        reject_oversize_content_length(req.headers(), limit).map_err(R::Error::from)?;
        let body = read_limited_body(req.into_body(), limit)
            .await
            .map_err(R::Error::from)?;
        let context = resolver.verify(&signature, &body).await?;
        let event = WebhookEvent::parse(&body).map_err(WebhookRejection::Malformed)?;
        Ok(ResolvedWebhook { event, context })
    }
}

impl IntoResponse for WebhookRejection {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status_code()).unwrap_or(StatusCode::BAD_REQUEST);
        (status, self.to_string()).into_response()
    }
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

async fn read_limited_body(body: Body, limit: usize) -> Result<Bytes, WebhookRejection> {
    let mut stream = body.into_data_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| WebhookRejection::BodyRead(e.to_string()))?;
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(WebhookRejection::PayloadTooLarge { limit });
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(bytes))
}
