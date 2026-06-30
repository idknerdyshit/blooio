//! The fully-resolved request description shared by both executors.

use http::header::AUTHORIZATION;

use crate::core::operation::Operation;
use crate::core::options::RequestOptions;
use crate::error::Result;

/// Header used to make retried mutating requests idempotent.
pub(crate) const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";

/// A concrete HTTP request, built once from an [`Operation`] and consumed by
/// whichever executor performs the IO.
#[allow(missing_docs)]
#[derive(Debug)]
pub struct RequestSpec {
    /// HTTP method resolved from the operation.
    pub method: http::Method,
    /// Path relative to the configured base URL.
    pub path: String,
    /// Query parameters before executor-level serialization.
    pub query: Vec<(String, String)>,
    /// Operation and request-option headers, excluding executor-injected auth.
    pub headers: Vec<(String, String)>,
    /// Request body bytes, if the operation has a body.
    pub body: Option<bytes::Bytes>,
}

impl RequestSpec {
    /// Resolve an [`Operation`] into a concrete request.
    pub fn build<O: Operation>(op: &O) -> Result<Self> {
        Ok(RequestSpec {
            method: O::METHOD,
            path: op.path(),
            query: op
                .query()
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
            headers: op
                .headers()
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
            body: op.body()?.map(bytes::Bytes::from),
        })
    }

    /// Apply request-scoped query/header overrides.
    ///
    /// Extra headers replace operation headers with the same name. The
    /// `Authorization` header is intentionally ignored here; executors inject
    /// it from their redacted `Secret` state.
    pub(crate) fn apply_options(&mut self, options: &RequestOptions) {
        self.query.extend(options.query.iter().cloned());

        for (key, value) in &options.headers {
            if key.eq_ignore_ascii_case(AUTHORIZATION.as_str()) {
                continue;
            }
            self.headers
                .retain(|(existing, _)| !existing.eq_ignore_ascii_case(key));
            self.headers.push((key.clone(), value.clone()));
        }
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
            self.headers.push((
                IDEMPOTENCY_KEY_HEADER.to_owned(),
                uuid::Uuid::new_v4().to_string(),
            ));
        }
    }
}

/// Build the absolute request URL, including encoded query parameters.
///
/// Both executors call this helper rather than relying on backend-specific
/// query serialization, keeping the async and blocking wire URLs identical.
pub(crate) fn url_with_query<K, V>(url: &str, query: &[(K, V)]) -> String
where
    K: AsRef<str>,
    V: AsRef<str>,
{
    if query.is_empty() {
        return url.to_owned();
    }

    let mut out = String::from(url);
    out.push(if url.contains('?') { '&' } else { '?' });
    for (i, (key, value)) in query.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        out.push_str(&encode_query_component(key.as_ref()));
        out.push('=');
        out.push_str(&encode_query_component(value.as_ref()));
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
                &[("limit", String::from("50"))]
            ),
            "https://example.com/api?existing=1&limit=50"
        );
    }

    #[test]
    fn apply_options_overrides_headers_and_appends_query() {
        let mut spec = RequestSpec {
            method: http::Method::GET,
            path: "/me".into(),
            query: vec![("limit".into(), "10".into())],
            headers: vec![("x-mode".into(), "old".into())],
            body: None,
        };

        spec.apply_options(
            &RequestOptions::new()
                .query("trace", "1")
                .header("x-mode", "new")
                .header("authorization", "Bearer wrong"),
        );

        assert_eq!(
            spec.query,
            vec![
                ("limit".to_owned(), "10".to_owned()),
                ("trace".to_owned(), "1".to_owned())
            ]
        );
        assert_eq!(spec.headers, vec![("x-mode".to_owned(), "new".to_owned())]);
    }
}
