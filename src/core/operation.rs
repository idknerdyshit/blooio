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
