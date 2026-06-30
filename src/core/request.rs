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
    pub body: Option<bytes::Bytes>,
}

impl RequestSpec {
    /// Resolve an [`Operation`] into a concrete request.
    pub fn build<O: Operation>(op: &O) -> Result<Self> {
        Ok(RequestSpec {
            method: O::METHOD,
            path: op.path(),
            query: op.query(),
            headers: op.headers(),
            body: op.body()?.map(bytes::Bytes::from),
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

/// Build the absolute request URL, including encoded query parameters.
///
/// Both executors call this helper rather than relying on backend-specific
/// query serialization, keeping the async and blocking wire URLs identical.
pub(crate) fn url_with_query(url: &str, query: &[(&'static str, String)]) -> String {
    if query.is_empty() {
        return url.to_owned();
    }

    let mut out = String::from(url);
    out.push(if url.contains('?') { '&' } else { '?' });
    for (i, (key, value)) in query.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        out.push_str(&encode_query_component(key));
        out.push('=');
        out.push_str(&encode_query_component(value));
    }
    out
}

fn encode_query_component(component: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let mut encoded = String::new();
    for b in component.bytes() {
        if b.is_ascii_alphanumeric()
            || matches!(b, b'!' | b'(' | b')' | b'*' | b'-' | b'.' | b'_' | b'~')
        {
            encoded.push(char::from(b));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(b >> 4)]));
            encoded.push(char::from(HEX[usize::from(b & 0x0F)]));
        }
    }
    encoded
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;

    #[test]
    fn url_with_query_encodes_query_components() {
        assert_eq!(
            url_with_query(
                "https://example.com/api",
                &[("q", "+1555/a b?x=1&y=☃".to_owned())]
            ),
            "https://example.com/api?q=%2B1555%2Fa%20b%3Fx%3D1%26y%3D%E2%98%83"
        );
    }

    #[test]
    fn url_with_query_appends_to_existing_query() {
        assert_eq!(
            url_with_query(
                "https://example.com/api?existing=1",
                &[("limit", "50".into())]
            ),
            "https://example.com/api?existing=1&limit=50"
        );
    }
}
