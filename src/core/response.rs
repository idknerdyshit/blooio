//! Response parsing shared by both executors.

use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::error::{ApiErrorBody, Error, Result};

/// Parse a raw HTTP response into the operation's output type, or map a non-2xx
/// status to [`Error::Api`].
///
/// On success the body is deserialized into `T`. On failure the body is decoded
/// from the Blooio `Error` schema where possible. Raw server-provided prose is
/// omitted from the stored/displayed error message so reflected request data
/// cannot leak through logs.
pub fn parse<T: DeserializeOwned>(status: u16, bytes: &[u8]) -> Result<T> {
    parse_with(status, bytes, None)
}

/// Like [`parse`], but attaches a `Retry-After` hint (extracted by the executor
/// from response headers) to any resulting [`Error::Api`].
pub fn parse_with<T: DeserializeOwned>(
    status: u16,
    bytes: &[u8],
    retry_after: Option<Duration>,
) -> Result<T> {
    if (200..300).contains(&status) {
        // A handful of 2xx responses may legitimately carry an empty body.
        // `serde_json` can deserialize `()` and `Option<_>` from "null", but
        // not from "", so normalize an empty body to `null`.
        let bytes: &[u8] = if bytes.is_empty() { b"null" } else { bytes };
        serde_json::from_slice(bytes).map_err(Error::decode)
    } else {
        Err(map_error(status, bytes, retry_after))
    }
}

/// Map a non-2xx response body to [`Error::Api`].
pub fn map_error(status: u16, bytes: &[u8], retry_after: Option<Duration>) -> Error {
    if let Ok(body) = serde_json::from_slice::<ApiErrorBody>(bytes) {
        let message = safe_api_message(status, body.code.as_deref(), body.error.as_deref());
        Error::Api {
            status,
            code: body.code,
            message,
            error: body.error,
            retry_after,
        }
    } else {
        Error::Api {
            status,
            code: None,
            message: format!("HTTP {status} (non-schema error body omitted)"),
            error: None,
            retry_after,
        }
    }
}

fn safe_api_message(status: u16, code: Option<&str>, error: Option<&str>) -> String {
    if let Some(code) = code {
        format!("HTTP {status} ({code})")
    } else if let Some(error) = error {
        format!("HTTP {status} ({error})")
    } else {
        format!("HTTP {status}")
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
                ..
            } => {
                assert_eq!(status, 429);
                assert_eq!(code.as_deref(), Some("outbound_limit_reached"));
                assert_eq!(message, "HTTP 429 (outbound_limit_reached)");
                assert_eq!(error.as_deref(), Some("rate_limited"));
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[test]
    fn structured_error_message_does_not_reflect_server_prose() {
        let body =
            br#"{"message":"bad token sk-secret-123","status":400,"code":"invalid_request"}"#;
        let err = parse::<Thing>(400, body).unwrap_err();
        assert!(
            !err.to_string().contains("sk-secret-123"),
            "structured error message leaked into Display"
        );
    }

    #[test]
    fn maps_non_schema_error_body() {
        let err = parse::<Thing>(500, b"upstream exploded").unwrap_err();
        assert_eq!(err.status(), Some(500));
        assert_eq!(err.code(), None);
        assert!(
            !err.to_string().contains("upstream exploded"),
            "raw error body leaked into Display"
        );
    }
}
