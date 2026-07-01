//! Retry policy: a sans-IO description of *when* and *how long* to wait before
//! retrying a failed request. The executors own the actual sleeping
//! ([`tokio::time::sleep`] for async, [`std::thread::sleep`] for blocking) so
//! this module pulls in no runtime and stays unit-testable.

use std::time::Duration;

use crate::error::Error;

/// Controls automatic retries of transient failures.
///
/// A request is retried only when [`Error::is_retryable`] is `true` (transport
/// errors and the transient statuses `408`/`425`, unknown/no-code `429`, and
/// `5xx`) and the attempt budget is not yet spent. The delay before each retry
/// is the larger-priority of the server's `Retry-After` hint (when present) or
/// exponential backoff with optional full jitter, clamped to
/// [`max_delay`](Self::max_delay).
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    /// Maximum number of retries *after* the initial attempt. `0` disables
    /// retrying. Defaults to `2` (so up to three total attempts).
    pub max_retries: u32,
    /// Base delay for exponential backoff. Defaults to 500ms.
    pub base_delay: Duration,
    /// Upper bound on any single delay. Defaults to 20s.
    pub max_delay: Duration,
    /// Apply full jitter (a random value in `[0, computed]`) to backoff delays
    /// to avoid synchronized retries. Does not affect `Retry-After` delays.
    /// Defaults to `true`.
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            max_retries: 2,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(20),
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// A policy that performs no retries.
    #[must_use]
    pub fn none() -> Self {
        RetryPolicy {
            max_retries: 0,
            ..Self::default()
        }
    }

    /// Set the maximum number of retries after the initial attempt.
    #[must_use]
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the base backoff delay.
    #[must_use]
    pub fn with_base_delay(mut self, base_delay: Duration) -> Self {
        self.base_delay = base_delay;
        self
    }

    /// Set the per-delay ceiling.
    #[must_use]
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }

    /// Enable or disable full jitter on backoff delays.
    #[must_use]
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Whether a request that produced `err` should be retried, given that
    /// `retries_done` retries have already happened.
    pub fn should_retry(&self, retries_done: u32, err: &Error) -> bool {
        retries_done < self.max_retries && err.is_retryable()
    }

    /// The delay to wait before the next retry. Honors the error's
    /// `Retry-After` hint when present (clamped to [`max_delay`](Self::max_delay)),
    /// otherwise computes jittered exponential backoff. `retries_done` is the
    /// number of retries already performed (0 for the first retry).
    pub fn delay_for(&self, retries_done: u32, err: &Error) -> Duration {
        if let Some(hint) = err.retry_after() {
            return hint.min(self.max_delay);
        }
        self.backoff(retries_done)
    }

    /// Exponential backoff for the `n`-th retry (0-based), with optional jitter.
    fn backoff(&self, n: u32) -> Duration {
        let factor = 2u32.saturating_pow(n);
        let raw = self.base_delay.saturating_mul(factor).min(self.max_delay);
        if self.jitter {
            let frac = jitter_fraction();
            raw.mul_f64(frac)
        } else {
            raw
        }
    }
}

/// A cheap pseudo-random fraction in `[0, 1)` derived from the system clock and
/// a process-wide counter.
///
/// Backoff jitter only needs to de-correlate concurrent retriers, not
/// cryptographic randomness, so this avoids pulling in a RNG dependency. The
/// counter guarantees distinct seeds even when two calls read the clock at the
/// same instant.
fn jitter_fraction() -> f64 {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    // xorshift a clock+counter seed to spread the low bits, then map to [0, 1).
    let mut x = (nanos ^ COUNTER.fetch_add(1, Ordering::Relaxed)) | 1;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    f64::from(x % 1_000_000) / 1_000_000.0
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
    use crate::error::{ApiError, ApiErrorDetails, codes};

    fn api_err(status: u16, retry_after: Option<Duration>) -> Error {
        api_err_with_code(status, None, retry_after)
    }

    fn api_err_with_code(status: u16, code: Option<&str>, retry_after: Option<Duration>) -> Error {
        Error::Api(ApiError::from_schema(
            status,
            code.map(str::to_owned),
            None,
            None,
            ApiErrorDetails::default(),
            retry_after,
        ))
    }

    #[test]
    fn does_not_retry_non_retryable() {
        let p = RetryPolicy::default();
        assert!(!p.should_retry(0, &api_err(400, None)));
        assert!(!p.should_retry(0, &Error::Decode("x".into())));
    }

    #[test]
    fn retries_transient_within_budget() {
        let p = RetryPolicy::default().with_max_retries(2);
        assert!(p.should_retry(0, &api_err(429, None)));
        assert!(p.should_retry(1, &api_err(503, None)));
        assert!(!p.should_retry(2, &api_err(503, None)));
    }

    #[test]
    fn does_not_retry_documented_quota_429_codes() {
        let p = RetryPolicy::default();
        assert!(!p.should_retry(
            0,
            &api_err_with_code(429, Some(codes::OUTBOUND_LIMIT_REACHED), None)
        ));
        assert!(!p.should_retry(
            0,
            &api_err_with_code(429, Some(codes::NEW_CONVERSATION_LIMIT_REACHED), None)
        ));
    }

    #[test]
    fn retries_unknown_429_codes() {
        let p = RetryPolicy::default();
        assert!(p.should_retry(
            0,
            &api_err_with_code(429, Some("temporarily_rate_limited"), None)
        ));
    }

    #[test]
    fn none_disables_retry() {
        let p = RetryPolicy::none();
        assert!(!p.should_retry(0, &api_err(503, None)));
    }

    #[test]
    fn retry_after_hint_is_honored_and_clamped() {
        let p = RetryPolicy::default()
            .with_jitter(false)
            .with_max_delay(Duration::from_secs(5));
        let d = p.delay_for(0, &api_err(429, Some(Duration::from_secs(2))));
        assert_eq!(d, Duration::from_secs(2));
        let clamped = p.delay_for(0, &api_err(429, Some(Duration::from_secs(45))));
        assert_eq!(clamped, Duration::from_secs(5));
    }

    #[test]
    fn backoff_grows_and_clamps_without_jitter() {
        let p = RetryPolicy::default()
            .with_jitter(false)
            .with_base_delay(Duration::from_millis(100))
            .with_max_delay(Duration::from_secs(1));
        assert_eq!(
            p.delay_for(0, &api_err(503, None)),
            Duration::from_millis(100)
        );
        assert_eq!(
            p.delay_for(1, &api_err(503, None)),
            Duration::from_millis(200)
        );
        assert_eq!(
            p.delay_for(2, &api_err(503, None)),
            Duration::from_millis(400)
        );
        // 100ms * 2^4 = 1600ms, clamped to the 1s ceiling.
        assert_eq!(p.delay_for(4, &api_err(503, None)), Duration::from_secs(1));
    }

    #[test]
    fn jitter_stays_within_bounds() {
        let p = RetryPolicy::default()
            .with_jitter(true)
            .with_base_delay(Duration::from_millis(100))
            .with_max_delay(Duration::from_secs(10));
        for _ in 0..1000 {
            let d = p.delay_for(1, &api_err(503, None));
            // raw for n=1 is 200ms; jittered must land in [0, 200ms].
            assert!(
                d <= Duration::from_millis(200),
                "jitter exceeded raw: {d:?}"
            );
        }
    }
}
