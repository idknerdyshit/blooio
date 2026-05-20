//! Response parsing shared by both executors.

use serde::de::DeserializeOwned;

use crate::error::{ApiErrorBody, Error, Result};

/// Parse a raw HTTP response into the operation's output type, or map a non-2xx
/// status to [`Error::Api`].
///
/// On success the body is deserialized into `T`. On failure the body is decoded
/// from the Blooio `Error` schema; if that fails, the raw (truncated) body text
/// is used as the message so the caller still gets something actionable. The
/// raw body is never logged here — only the status and machine `code` are safe
/// to log, which the executors do.
pub fn parse<T: DeserializeOwned>(status: u16, bytes: &[u8]) -> Result<T> {
    if (200..300).contains(&status) {
        // A handful of 2xx responses may legitimately carry an empty body.
        // `serde_json` can deserialize `()` and `Option<_>` from "null", but
        // not from "", so normalize an empty body to `null`.
        let bytes: &[u8] = if bytes.is_empty() { b"null" } else { bytes };
        serde_json::from_slice(bytes).map_err(Error::decode)
    } else {
        Err(map_error(status, bytes))
    }
}

/// Map a non-2xx response body to [`Error::Api`].
pub fn map_error(status: u16, bytes: &[u8]) -> Error {
    if let Ok(body) = serde_json::from_slice::<ApiErrorBody>(bytes) {
        Error::Api {
            status,
            code: body.code,
            message: body
                .message
                .or(body.error.clone())
                .unwrap_or_else(|| format!("HTTP {status}")),
            error: body.error,
        }
    } else {
        // Body wasn't the documented error schema. Fall back to the raw
        // text, truncated to keep the message bounded.
        let raw = String::from_utf8_lossy(bytes);
        let message = if raw.trim().is_empty() {
            format!("HTTP {status}")
        } else {
            raw.chars().take(512).collect()
        };
        Error::Api {
            status,
            code: None,
            message,
            error: None,
        }
    }
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
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Thing {
        id: String,
    }

    #[test]
    fn parses_success_body() {
        let t: Thing = parse(200, br#"{"id":"abc"}"#).unwrap();
        assert_eq!(t, Thing { id: "abc".into() });
    }

    #[test]
    fn maps_error_schema() {
        let body = br#"{"error":"rate_limited","message":"slow down","status":429,"code":"outbound_limit_reached"}"#;
        let err = parse::<Thing>(429, body).unwrap_err();
        match err {
            Error::Api {
                status,
                code,
                message,
                error,
            } => {
                assert_eq!(status, 429);
                assert_eq!(code.as_deref(), Some("outbound_limit_reached"));
                assert_eq!(message, "slow down");
                assert_eq!(error.as_deref(), Some("rate_limited"));
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[test]
    fn maps_non_schema_error_body() {
        let err = parse::<Thing>(500, b"upstream exploded").unwrap_err();
        assert_eq!(err.status(), Some(500));
        assert_eq!(err.code(), None);
    }
}
