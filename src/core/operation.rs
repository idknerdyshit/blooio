//! The [`Operation`] trait: a sans-IO description of a single API call.

use crate::error::{Error, Result};

/// Describes one endpoint's HTTP shape and its decoded output type.
///
/// Each endpoint implements this exactly once. The two executors (async and
/// blocking) consume `Operation`s and perform the actual IO, so endpoint logic
/// is never duplicated between them.
///
/// `Operation` types are public and can be passed directly to
/// [`Client::send`](crate::Client::send) /
/// [`BlockingClient::send`](crate::BlockingClient::send) as an escape hatch.
pub trait Operation {
    /// The type the response body deserializes into.
    type Output: serde::de::DeserializeOwned;

    /// The HTTP method.
    const METHOD: http::Method;

    /// The path, relative to the configured base URL (must begin with `/`).
    fn path(&self) -> String;

    /// Query-string parameters. Empty values are still included; callers that
    /// want to omit an optional parameter should not push it.
    fn query(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    /// Extra request headers (e.g. `Idempotency-Key`).
    fn headers(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    /// The JSON request body, if any.
    fn body(&self) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
}

/// Helper for `Operation::body` implementations: serialize a value to JSON
/// bytes, mapping failures to [`Error::Encode`].
pub fn json_body<T: serde::Serialize>(value: &T) -> Result<Option<Vec<u8>>> {
    serde_json::to_vec(value).map(Some).map_err(Error::encode)
}

/// Helper for `Operation::query` implementations: push a `(key, value)` pair
/// only when the value is present. Pass `Copy` values (`Option<u32>`) directly
/// and string-like values by reference (`self.q.as_ref()`).
pub fn push_opt<T: ToString>(
    q: &mut Vec<(&'static str, String)>,
    key: &'static str,
    value: Option<T>,
) {
    if let Some(v) = value {
        q.push((key, v.to_string()));
    }
}

/// Percent-encode one URL path segment.
///
/// Use this for user/API-provided identifiers that are interpolated between
/// `/` separators. Unreserved RFC 3986 characters are left untouched; every
/// other byte is encoded so values like phone numbers, handles, tags, and
/// provider IDs cannot alter the path structure.
pub fn encode_path_segment(segment: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let mut encoded = String::new();
    for b in segment.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
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
    fn encode_path_segment_leaves_unreserved_chars() {
        assert_eq!(encode_path_segment("abc-123._~"), "abc-123._~");
    }

    #[test]
    fn encode_path_segment_escapes_reserved_and_unicode_bytes() {
        assert_eq!(
            encode_path_segment("+1555/a b?#☃"),
            "%2B1555%2Fa%20b%3F%23%E2%98%83"
        );
    }
}
