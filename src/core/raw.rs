//! Raw HTTP response containers returned by executor escape hatches.

use std::fmt;

use bytes::Bytes;
use http::HeaderMap;

use crate::core::ratelimit::ResponseMeta;

/// Raw HTTP response data captured after the body has been read.
#[derive(Clone)]
pub struct RawResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response headers.
    pub headers: HeaderMap,
    /// Raw response body bytes.
    pub body: Bytes,
}

impl RawResponse {
    pub(crate) fn new(status: u16, headers: HeaderMap, body: Bytes) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }
}

impl fmt::Debug for RawResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawResponse")
            .field("status", &self.status)
            .field(
                "headers",
                &format_args!("[REDACTED; {}]", self.headers.len()),
            )
            .field(
                "body",
                &format_args!("[REDACTED; {} bytes]", self.body.len()),
            )
            .finish()
    }
}

/// Decoded response plus raw HTTP data from the same request.
#[derive(Clone)]
pub struct ApiResponse<T> {
    /// Decoded operation output.
    pub output: T,
    /// Parsed metadata such as rate-limit and `Retry-After` headers.
    pub meta: ResponseMeta,
    /// Raw response status, headers, and body bytes.
    pub raw: RawResponse,
}

impl<T: fmt::Debug> fmt::Debug for ApiResponse<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiResponse")
            .field("output", &self.output)
            .field("meta", &self.meta)
            .field("raw", &self.raw)
            .finish()
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout
)]
mod tests {
    use super::*;

    #[test]
    fn raw_debug_redacts_headers_and_body() {
        let mut headers = HeaderMap::new();
        headers.insert("x-token", "secret-token".parse().unwrap());
        let raw = RawResponse::new(200, headers, Bytes::from_static(b"secret-body"));
        let dbg = format!("{raw:?}");
        assert!(!dbg.contains("secret-token"));
        assert!(!dbg.contains("secret-body"));
        assert!(dbg.contains("REDACTED"));
    }
}
