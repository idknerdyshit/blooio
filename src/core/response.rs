//! Response parsing shared by both executors.

use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::error::{ApiError, ApiErrorBody, ApiErrorDetails, Error, Result};

/// Parse a raw HTTP response into the operation's output type, or map a non-2xx
/// status to [`Error::Api`].
///
/// On success the body is deserialized into `T`. On failure the body is decoded
/// from the Blooio `Error` schema where possible. Raw server-provided prose is
/// stored only for explicit accessor use and is omitted from displayed/logged
/// error messages so reflected request data cannot leak through logs.
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
        Error::Api(ApiError::from_schema(
            status,
            body.code,
            body.error,
            body.message,
            ApiErrorDetails::new(body.details),
            retry_after,
        ))
    } else {
        Error::Api(ApiError::from_non_schema(status, retry_after))
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
        let body = br#"{"error":"rate_limited","message":"slow down","status":429,"code":"outbound_limit_reached","limit":10,"current":10}"#;
        let err = parse::<Thing>(429, body).unwrap_err();
        match err {
            Error::Api(api) => {
                assert_eq!(api.status(), 429);
                assert_eq!(api.code(), Some("outbound_limit_reached"));
                assert_eq!(
                    api.to_string(),
                    "blooio api error (status 429, code outbound_limit_reached): HTTP 429 (outbound_limit_reached)"
                );
                assert_eq!(api.error(), Some("rate_limited"));
                assert_eq!(api.server_message(), Some("slow down"));
                assert_eq!(
                    api.details().get("limit"),
                    Some(&serde_json::Value::from(10))
                );
                assert_eq!(
                    api.details().get("current"),
                    Some(&serde_json::Value::from(10))
                );
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
        assert!(
            !format!("{err:?}").contains("sk-secret-123"),
            "structured error message leaked into Debug"
        );
        let Error::Api(api) = err else {
            panic!("expected Api error");
        };
        assert_eq!(api.server_message(), Some("bad token sk-secret-123"));
    }

    #[test]
    fn structured_error_details_are_explicit_and_redacted_in_debug() {
        let body = br#"{"message":"quota hit sk-secret-123","status":429,"code":"new_conversation_limit_reached","plan_id":"plan_123","cap":50,"current":50}"#;
        let err = parse::<Thing>(429, body).unwrap_err();
        let debug = format!("{err:?}");

        assert!(!debug.contains("plan_123"));
        assert!(!debug.contains("sk-secret-123"));
        let Error::Api(api) = err else {
            panic!("expected Api error");
        };
        assert_eq!(
            api.details().get("plan_id"),
            Some(&serde_json::json!("plan_123"))
        );
        assert_eq!(api.details().get("cap"), Some(&serde_json::json!(50)));
        assert_eq!(api.details().as_object().len(), 3);
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
