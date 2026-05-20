//! A redacting, zero-on-drop wrapper for sensitive values.
//!
//! [`Secret`] holds a value that must never be logged or serialized in
//! cleartext: the API key, a webhook signing secret, and so on. It implements:
//!
//! - [`Zeroize`] + [`ZeroizeOnDrop`], so the underlying buffer is wiped from
//!   memory when the secret is dropped.
//! - [`Debug`](fmt::Debug) and [`Display`](fmt::Display) that emit
//!   `"[REDACTED]"`, so it can never be accidentally printed — including inside
//!   a derived `Debug` on a struct that contains a `Secret`.
//! - [`serde::Deserialize`] only (never `Serialize`), so it cannot leak through
//!   JSON.
//!
//! The only way to read the inner value is the explicit [`Secret::expose`]
//! accessor, which is used solely at the point of building an `Authorization`
//! header or computing an HMAC — never passed to a logging macro.

use std::fmt;

use serde::{Deserialize, Deserializer};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A wrapper around a sensitive value that redacts on debug/display and wipes
/// its buffer on drop.
#[derive(Clone, ZeroizeOnDrop)]
pub struct Secret<T: Zeroize>(T);

impl<T: Zeroize> Secret<T> {
    /// Wrap a sensitive value.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Borrow the inner secret value.
    ///
    /// Use this only at the boundary where the secret must be used (e.g.
    /// building an `Authorization` header). Never pass the result to a logging
    /// macro.
    pub fn expose(&self) -> &T {
        &self.0
    }
}

impl<T: Zeroize> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl<T: Zeroize> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}

impl From<String> for Secret<String> {
    fn from(value: String) -> Self {
        Secret(value)
    }
}

impl From<&str> for Secret<String> {
    fn from(value: &str) -> Self {
        Secret(value.to_owned())
    }
}

impl From<Vec<u8>> for Secret<Vec<u8>> {
    fn from(value: Vec<u8>) -> Self {
        Secret(value)
    }
}

impl<'de, T> Deserialize<'de> for Secret<T>
where
    T: Zeroize + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Secret(T::deserialize(deserializer)?))
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

    #[test]
    fn debug_and_display_are_redacted() {
        let s = Secret::from("super-secret-key");
        assert_eq!(format!("{s:?}"), "[REDACTED]");
        assert_eq!(format!("{s}"), "[REDACTED]");
        assert!(!format!("{s:?}").contains("super-secret"));
    }

    #[test]
    fn expose_returns_inner() {
        let s = Secret::from("abc");
        assert_eq!(s.expose(), "abc");
    }

    #[test]
    fn deserialize_from_json_string() {
        let s: Secret<String> = serde_json::from_str("\"k\"").unwrap();
        assert_eq!(s.expose(), "k");
    }
}
