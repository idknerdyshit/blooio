//! Internal, secret-redacted tracing helpers shared by both executors.

use std::time::{Duration, Instant};

use http::Method;

use crate::error::Error;

pub(crate) const TARGET: &str = "blooio::trace";

pub(crate) struct OperationTrace {
    operation: &'static str,
    max_retries: u32,
    start: Instant,
}

impl OperationTrace {
    pub(crate) fn new(operation: &'static str, max_retries: u32) -> Self {
        Self {
            operation,
            max_retries,
            start: Instant::now(),
        }
    }

    pub(crate) fn success(&self, method: &Method, attempts: u32, status: u16) {
        operation_success(
            method,
            self.operation,
            attempts,
            self.max_retries,
            status,
            self.start.elapsed(),
        );
    }

    pub(crate) fn failure(
        &self,
        method: &Method,
        attempts: u32,
        status: Option<u16>,
        error: &Error,
    ) {
        operation_failure(
            method,
            self.operation,
            attempts,
            self.max_retries,
            self.start.elapsed(),
            status,
            error,
        );
    }

    pub(crate) fn retry(
        &self,
        method: &Method,
        attempt: u32,
        next_attempt: u32,
        delay: Duration,
        error: &Error,
    ) {
        retry(
            method,
            self.operation,
            attempt,
            next_attempt,
            self.max_retries,
            delay,
            error,
        );
    }
}

pub(crate) fn request_span(
    method: &Method,
    operation: &'static str,
    attempt: u32,
    max_retries: u32,
) -> tracing::Span {
    tracing::info_span!(
        target: TARGET,
        "blooio.request",
        method = method.as_str(),
        operation = operation,
        attempt,
        max_retries,
        status = tracing::field::Empty,
        elapsed_ms = tracing::field::Empty,
    )
}

pub(crate) fn attempt_response(
    span: &tracing::Span,
    method: &Method,
    operation: &'static str,
    attempt: u32,
    max_retries: u32,
    status: u16,
    elapsed: Duration,
) {
    let elapsed_ms = duration_ms(elapsed);
    span.record("status", status);
    span.record("elapsed_ms", elapsed_ms);
    tracing::debug!(
        target: TARGET,
        event = "blooio.request.attempt.response",
        method = method.as_str(),
        operation = operation,
        attempt,
        max_retries,
        status,
        elapsed_ms,
    );
}

pub(crate) fn attempt_error(
    span: &tracing::Span,
    method: &Method,
    operation: &'static str,
    attempt: u32,
    max_retries: u32,
    elapsed: Duration,
    error: &Error,
) {
    let elapsed_ms = duration_ms(elapsed);
    span.record("elapsed_ms", elapsed_ms);
    tracing::warn!(
        target: TARGET,
        event = "blooio.request.attempt.error",
        method = method.as_str(),
        operation = operation,
        attempt,
        max_retries,
        elapsed_ms,
        error_kind = error_kind(error),
    );
}

pub(crate) fn retry(
    method: &Method,
    operation: &'static str,
    attempt: u32,
    next_attempt: u32,
    max_retries: u32,
    delay: Duration,
    error: &Error,
) {
    let delay_ms = duration_ms(delay);
    let delay_source = if error.retry_after().is_some() {
        "retry_after"
    } else {
        "backoff"
    };
    let error_kind = error_kind(error);
    let status = error.status();
    let code = error.code();
    let retry_after_ms = error.retry_after().map(duration_ms);

    match (status, code, retry_after_ms) {
        (Some(status), Some(code), Some(retry_after_ms)) => tracing::warn!(
            target: TARGET,
            event = "blooio.request.retry",
            method = method.as_str(),
            operation = operation,
            attempt,
            next_attempt,
            max_retries,
            delay_ms,
            delay_source,
            error_kind,
            status,
            code,
            retry_after_ms,
        ),
        (Some(status), Some(code), None) => tracing::warn!(
            target: TARGET,
            event = "blooio.request.retry",
            method = method.as_str(),
            operation = operation,
            attempt,
            next_attempt,
            max_retries,
            delay_ms,
            delay_source,
            error_kind,
            status,
            code,
        ),
        (Some(status), None, Some(retry_after_ms)) => tracing::warn!(
            target: TARGET,
            event = "blooio.request.retry",
            method = method.as_str(),
            operation = operation,
            attempt,
            next_attempt,
            max_retries,
            delay_ms,
            delay_source,
            error_kind,
            status,
            retry_after_ms,
        ),
        (Some(status), None, None) => tracing::warn!(
            target: TARGET,
            event = "blooio.request.retry",
            method = method.as_str(),
            operation = operation,
            attempt,
            next_attempt,
            max_retries,
            delay_ms,
            delay_source,
            error_kind,
            status,
        ),
        (None, _, _) => tracing::warn!(
            target: TARGET,
            event = "blooio.request.retry",
            method = method.as_str(),
            operation = operation,
            attempt,
            next_attempt,
            max_retries,
            delay_ms,
            delay_source,
            error_kind,
        ),
    }
}

pub(crate) fn operation_success(
    method: &Method,
    operation: &'static str,
    attempts: u32,
    max_retries: u32,
    status: u16,
    elapsed: Duration,
) {
    tracing::debug!(
        target: TARGET,
        event = "blooio.operation.success",
        method = method.as_str(),
        operation = operation,
        attempts,
        max_retries,
        status,
        elapsed_ms = duration_ms(elapsed),
    );
}

pub(crate) fn operation_failure(
    method: &Method,
    operation: &'static str,
    attempts: u32,
    max_retries: u32,
    elapsed: Duration,
    status: Option<u16>,
    error: &Error,
) {
    let elapsed_ms = duration_ms(elapsed);
    let error_kind = error_kind(error);
    let status = status.or_else(|| error.status());
    let code = error.code();

    match (status, code) {
        (Some(status), Some(code)) => tracing::warn!(
            target: TARGET,
            event = "blooio.operation.failure",
            method = method.as_str(),
            operation = operation,
            attempts,
            max_retries,
            elapsed_ms,
            error_kind,
            status,
            code,
        ),
        (Some(status), None) => tracing::warn!(
            target: TARGET,
            event = "blooio.operation.failure",
            method = method.as_str(),
            operation = operation,
            attempts,
            max_retries,
            elapsed_ms,
            error_kind,
            status,
        ),
        (None, _) => tracing::warn!(
            target: TARGET,
            event = "blooio.operation.failure",
            method = method.as_str(),
            operation = operation,
            attempts,
            max_retries,
            elapsed_ms,
            error_kind,
        ),
    }
}

pub(crate) fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn error_kind(error: &Error) -> &'static str {
    match error {
        Error::Api(_) => "api",
        Error::Transport(_) => "transport",
        Error::Encode(_) => "encode",
        Error::Decode(_) => "decode",
        Error::Config(_) => "config",
        #[cfg(feature = "webhooks")]
        Error::Webhook(_) => "webhook",
    }
}
