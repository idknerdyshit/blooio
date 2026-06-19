//! Rate-limit metadata extracted from response headers.
//!
//! The Blooio API communicates throttling state through response headers. This
//! module parses them into a [`RateLimit`] and bundles them with the
//! `Retry-After` hint into [`ResponseMeta`], which both executors return from
//! their `*_with_meta` entry points.

use std::time::Duration;

use http::HeaderMap;

/// Per-request rate-limit state reported by the API.
///
/// Field availability depends on the endpoint and response. Names are matched
/// case-insensitively against both the `X-RateLimit-*` and the IETF
/// `RateLimit-*` conventions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RateLimit {
    /// The ceiling of requests permitted in the current window.
    pub limit: Option<u64>,
    /// Requests remaining in the current window.
    pub remaining: Option<u64>,
    /// The window-reset value exactly as the API reported it (typically either
    /// epoch seconds or seconds-until-reset, per the endpoint's convention).
    pub reset: Option<u64>,
}

impl RateLimit {
    /// `true` when the API reported zero remaining requests.
    pub fn is_exhausted(&self) -> bool {
        self.remaining == Some(0)
    }

    /// Parse rate-limit headers, returning `None` if none were present.
    fn from_headers(headers: &HeaderMap) -> Option<Self> {
        let limit = header_u64(headers, &["x-ratelimit-limit", "ratelimit-limit"]);
        let remaining = header_u64(headers, &["x-ratelimit-remaining", "ratelimit-remaining"]);
        let reset = header_u64(headers, &["x-ratelimit-reset", "ratelimit-reset"]);
        if limit.is_none() && remaining.is_none() && reset.is_none() {
            None
        } else {
            Some(RateLimit {
                limit,
                remaining,
                reset,
            })
        }
    }
}

/// Metadata about a single HTTP response, returned alongside the decoded body
/// by the `*_with_meta` executor methods.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ResponseMeta {
    /// The HTTP status code.
    pub status: u16,
    /// Rate-limit state, if the response carried any `*-RateLimit-*` headers.
    pub rate_limit: Option<RateLimit>,
    /// The `Retry-After` hint as a duration, if present and expressed in
    /// delta-seconds. HTTP-date forms are not interpreted.
    pub retry_after: Option<Duration>,
}

impl ResponseMeta {
    /// Build metadata from a status code and the response header map.
    pub fn from_headers(status: u16, headers: &HeaderMap) -> Self {
        ResponseMeta {
            status,
            rate_limit: RateLimit::from_headers(headers),
            retry_after: retry_after(headers),
        }
    }
}

/// Parse the `Retry-After` header as delta-seconds.
pub(crate) fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    header_u64(headers, &["retry-after"]).map(Duration::from_secs)
}

/// Look up the first present header among `names` and parse it as a `u64`.
fn header_u64(headers: &HeaderMap, names: &[&str]) -> Option<u64> {
    names
        .iter()
        .find_map(|name| headers.get(*name))
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse().ok())
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

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                http::HeaderValue::from_str(v).unwrap(),
            );
        }
        h
    }

    #[test]
    fn parses_x_ratelimit_headers() {
        let h = headers(&[
            ("x-ratelimit-limit", "100"),
            ("x-ratelimit-remaining", "0"),
            ("x-ratelimit-reset", "1700000000"),
        ]);
        let rl = RateLimit::from_headers(&h).unwrap();
        assert_eq!(rl.limit, Some(100));
        assert_eq!(rl.remaining, Some(0));
        assert_eq!(rl.reset, Some(1_700_000_000));
        assert!(rl.is_exhausted());
    }

    #[test]
    fn parses_ietf_ratelimit_headers() {
        let h = headers(&[("ratelimit-remaining", "5")]);
        let rl = RateLimit::from_headers(&h).unwrap();
        assert_eq!(rl.remaining, Some(5));
        assert!(!rl.is_exhausted());
    }

    #[test]
    fn no_headers_yields_none() {
        assert_eq!(RateLimit::from_headers(&HeaderMap::new()), None);
    }

    #[test]
    fn retry_after_parses_delta_seconds() {
        let h = headers(&[("retry-after", "30")]);
        assert_eq!(retry_after(&h), Some(Duration::from_secs(30)));
    }

    #[test]
    fn retry_after_ignores_http_date() {
        let h = headers(&[("retry-after", "Wed, 21 Oct 2015 07:28:00 GMT")]);
        assert_eq!(retry_after(&h), None);
    }

    #[test]
    fn meta_bundles_status_and_fields() {
        let h = headers(&[("x-ratelimit-remaining", "9"), ("retry-after", "2")]);
        let meta = ResponseMeta::from_headers(429, &h);
        assert_eq!(meta.status, 429);
        assert_eq!(meta.rate_limit.unwrap().remaining, Some(9));
        assert_eq!(meta.retry_after, Some(Duration::from_secs(2)));
    }
}
