//! HMAC-SHA256 webhook signature verification (Stripe-style).
//!
//! The signature header has the form `t=<unix_seconds>,v1=<hex_hmac>`. The
//! signed payload is `"{t}.{raw_body}"`, and `v1` is the lowercase hex
//! `HMAC-SHA256(secret, signed_payload)`. Verification is constant-time and
//! rejects timestamps outside a tolerance window to prevent replay.

use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

/// Default replay-protection tolerance, in seconds.
pub const DEFAULT_TOLERANCE_SECS: u64 = 300;

/// Why a webhook signature failed to verify.
#[allow(missing_docs)]
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum VerifyError {
    /// The signature header could not be parsed (missing `t`/`v1`, bad hex…).
    #[error("malformed signature header")]
    MalformedHeader,
    /// The timestamp was outside the allowed tolerance window.
    #[error("timestamp outside tolerance: |now - {timestamp}| > {tolerance}s")]
    TimestampOutOfTolerance { timestamp: i64, tolerance: u64 },
    /// The computed signature did not match any provided `v1` signature.
    #[error("signature mismatch")]
    Mismatch,
}

/// Parsed components of a signature header.
struct Header {
    timestamp: i64,
    signatures: Vec<Vec<u8>>,
}

fn parse_header(value: &str) -> Result<Header, VerifyError> {
    let mut timestamp: Option<i64> = None;
    let mut signatures = Vec::new();
    for part in value.split(',') {
        let (k, v) = part.split_once('=').ok_or(VerifyError::MalformedHeader)?;
        match k.trim() {
            "t" => {
                timestamp = Some(v.trim().parse().map_err(|_| VerifyError::MalformedHeader)?);
            }
            "v1" => {
                let raw = hex::decode(v.trim()).map_err(|_| VerifyError::MalformedHeader)?;
                signatures.push(raw);
            }
            _ => {} // ignore unknown schemes for forward-compat
        }
    }
    let timestamp = timestamp.ok_or(VerifyError::MalformedHeader)?;
    if signatures.is_empty() {
        return Err(VerifyError::MalformedHeader);
    }
    Ok(Header {
        timestamp,
        signatures,
    })
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// Verify a webhook signature with an explicit tolerance window.
///
/// `secret` is the webhook signing secret, `header_value` the raw signature
/// header, and `raw_body` the **unparsed** request body bytes. Returns `Ok(())`
/// if a provided signature matches and the timestamp is within `tolerance`.
pub fn verify(
    secret: &[u8],
    header_value: &str,
    raw_body: &[u8],
    tolerance: u64,
) -> Result<(), VerifyError> {
    verify_at(secret, header_value, raw_body, tolerance, now_unix())
}

/// Verify using default tolerance ([`DEFAULT_TOLERANCE_SECS`]).
pub fn verify_default(
    secret: &[u8],
    header_value: &str,
    raw_body: &[u8],
) -> Result<(), VerifyError> {
    verify(secret, header_value, raw_body, DEFAULT_TOLERANCE_SECS)
}

/// Verify against a caller-supplied "now" — the testable core.
pub fn verify_at(
    secret: &[u8],
    header_value: &str,
    raw_body: &[u8],
    tolerance: u64,
    now: i64,
) -> Result<(), VerifyError> {
    let header = parse_header(header_value)?;

    if (now - header.timestamp).unsigned_abs() > tolerance {
        return Err(VerifyError::TimestampOutOfTolerance {
            timestamp: header.timestamp,
            tolerance,
        });
    }

    let mut mac = HmacSha256::new_from_slice(secret).map_err(|_| VerifyError::MalformedHeader)?;
    mac.update(header.timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(raw_body);
    let expected = mac.finalize().into_bytes();

    // Constant-time compare against every provided v1 signature.
    let mut matched = 0u8;
    for sig in &header.signatures {
        matched |= u8::from(bool::from(expected.as_slice().ct_eq(sig.as_slice())));
    }
    if matched == 1 {
        Ok(())
    } else {
        Err(VerifyError::Mismatch)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::print_stdout, clippy::unreadable_literal)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"whsec_test_secret";

    fn sign(timestamp: i64, body: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(SECRET).unwrap();
        mac.update(timestamp.to_string().as_bytes());
        mac.update(b".");
        mac.update(body);
        let sig = hex::encode(mac.finalize().into_bytes());
        format!("t={timestamp},v1={sig}")
    }

    #[test]
    fn valid_signature_passes() {
        let body = br#"{"event":"message.received"}"#;
        let header = sign(1_700_000_000, body);
        assert!(verify_at(SECRET, &header, body, 300, 1_700_000_000).is_ok());
    }

    #[test]
    fn tampered_body_fails() {
        let body = br#"{"event":"message.received"}"#;
        let header = sign(1_700_000_000, body);
        let tampered = br#"{"event":"message.read"}"#;
        assert_eq!(
            verify_at(SECRET, &header, tampered, 300, 1_700_000_000),
            Err(VerifyError::Mismatch)
        );
    }

    #[test]
    fn expired_timestamp_fails() {
        let body = b"{}";
        let header = sign(1_700_000_000, body);
        let now = 1_700_000_000 + 1000;
        assert!(matches!(
            verify_at(SECRET, &header, body, 300, now),
            Err(VerifyError::TimestampOutOfTolerance { .. })
        ));
    }

    #[test]
    fn malformed_header_fails() {
        assert_eq!(
            verify_at(SECRET, "garbage", b"{}", 300, 0),
            Err(VerifyError::MalformedHeader)
        );
        assert_eq!(
            verify_at(SECRET, "t=123", b"{}", 300, 123),
            Err(VerifyError::MalformedHeader)
        );
    }

    #[test]
    fn wrong_secret_fails() {
        let body = b"{}";
        let header = sign(1_700_000_000, body);
        assert_eq!(
            verify_at(b"other", &header, body, 300, 1_700_000_000),
            Err(VerifyError::Mismatch)
        );
    }
}
