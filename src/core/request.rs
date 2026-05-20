//! The fully-resolved request description shared by both executors.

use crate::core::operation::Operation;
use crate::error::Result;

/// Header used to make retried mutating requests idempotent.
pub(crate) const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";

/// A concrete HTTP request, built once from an [`Operation`] and consumed by
/// whichever executor performs the IO.
#[allow(missing_docs)]
#[derive(Debug)]
pub struct RequestSpec {
    pub method: http::Method,
    pub path: String,
    pub query: Vec<(&'static str, String)>,
    pub headers: Vec<(&'static str, String)>,
    pub body: Option<Vec<u8>>,
}

impl RequestSpec {
    /// Resolve an [`Operation`] into a concrete request.
    pub fn build<O: Operation>(op: &O) -> Result<Self> {
        Ok(RequestSpec {
            method: O::METHOD,
            path: op.path(),
            query: op.query(),
            headers: op.headers(),
            body: op.body()?,
        })
    }

    /// Whether this request mutates server state (anything but `GET`/`HEAD`/
    /// `OPTIONS`). Used to decide when an idempotency key is warranted.
    pub fn is_mutating(&self) -> bool {
        !matches!(
            self.method,
            http::Method::GET | http::Method::HEAD | http::Method::OPTIONS
        )
    }

    /// Attach an `Idempotency-Key` header to a mutating request if it doesn't
    /// already carry one, so that automatic retries cannot duplicate side
    /// effects. No-op for non-mutating requests or when a key is already set.
    pub fn ensure_idempotency_key(&mut self) {
        if self.is_mutating()
            && !self
                .headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case(IDEMPOTENCY_KEY_HEADER))
        {
            self.headers
                .push((IDEMPOTENCY_KEY_HEADER, uuid::Uuid::new_v4().to_string()));
        }
    }
}
