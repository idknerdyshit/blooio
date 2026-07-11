//! Internal, secret-redacted tracing helpers shared by both executors.

use std::time::{Duration, Instant};

use http::Method;

use crate::error::Error;

pub(crate) const TARGET: &str = "blooio::trace";

pub(crate) struct OperationTrace {
    operation: &'static str,
    max_retries: u32,
    trace_label: Option<String>,
    start: Instant,
}

impl OperationTrace {
    pub(crate) fn new(
        operation: &'static str,
        max_retries: u32,
        trace_label: Option<&str>,
    ) -> Self {
        Self {
            operation,
            max_retries,
            trace_label: trace_label.map(ToOwned::to_owned),
            start: Instant::now(),
        }
    }

    pub(crate) fn success(&self, method: &Method, attempts: u32, status: u16) {
        operation_success(self, method, attempts, status, self.start.elapsed());
    }

    pub(crate) fn failure(
        &self,
        method: &Method,
        attempts: u32,
        status: Option<u16>,
        error: &Error,
    ) {
        operation_failure(self, method, attempts, self.start.elapsed(), status, error);
    }

    pub(crate) fn retry(
        &self,
        method: &Method,
        attempt: u32,
        next_attempt: u32,
        delay: Duration,
        error: &Error,
    ) {
        retry(self, method, attempt, next_attempt, delay, error);
    }
}

pub(crate) struct AttemptTrace<'a> {
    method: &'a Method,
    operation: &'static str,
    attempt: u32,
    max_retries: u32,
    trace_label: Option<&'a str>,
}

impl<'a> AttemptTrace<'a> {
    pub(crate) fn new(
        method: &'a Method,
        operation: &'static str,
        attempt: u32,
        max_retries: u32,
        trace_label: Option<&'a str>,
    ) -> Self {
        Self {
            method,
            operation,
            attempt,
            max_retries,
            trace_label,
        }
    }
}

macro_rules! trace_debug {
    ($trace_label:expr, $($field:tt)*) => {
        if let Some(trace_label) = $trace_label {
            tracing::debug!(target: TARGET, trace_label, $($field)*);
        } else {
            tracing::debug!(target: TARGET, $($field)*);
        }
    };
}

macro_rules! trace_warn {
    ($trace_label:expr, $($field:tt)*) => {
        if let Some(trace_label) = $trace_label {
            tracing::warn!(target: TARGET, trace_label, $($field)*);
        } else {
            tracing::warn!(target: TARGET, $($field)*);
        }
    };
}

pub(crate) fn request_span(trace: &AttemptTrace<'_>) -> tracing::Span {
    if let Some(trace_label) = trace.trace_label {
        tracing::info_span!(
            target: TARGET,
            "blooio.request",
            method = trace.method.as_str(),
            operation = trace.operation,
            attempt = trace.attempt,
            max_retries = trace.max_retries,
            trace_label,
            status = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty,
        )
    } else {
        tracing::info_span!(
            target: TARGET,
            "blooio.request",
            method = trace.method.as_str(),
            operation = trace.operation,
            attempt = trace.attempt,
            max_retries = trace.max_retries,
            status = tracing::field::Empty,
            elapsed_ms = tracing::field::Empty,
        )
    }
}

pub(crate) fn attempt_response(
    span: &tracing::Span,
    trace: &AttemptTrace<'_>,
    status: u16,
    elapsed: Duration,
) {
    let elapsed_ms = duration_ms(elapsed);
    span.record("status", status);
    span.record("elapsed_ms", elapsed_ms);
    trace_debug!(
        trace.trace_label,
        event = "blooio.request.attempt.response",
        method = trace.method.as_str(),
        operation = trace.operation,
        attempt = trace.attempt,
        max_retries = trace.max_retries,
        status,
        elapsed_ms,
    );
}

pub(crate) fn attempt_error(
    span: &tracing::Span,
    trace: &AttemptTrace<'_>,
    elapsed: Duration,
    error: &Error,
) {
    let elapsed_ms = duration_ms(elapsed);
    span.record("elapsed_ms", elapsed_ms);
    trace_warn!(
        trace.trace_label,
        event = "blooio.request.attempt.error",
        method = trace.method.as_str(),
        operation = trace.operation,
        attempt = trace.attempt,
        max_retries = trace.max_retries,
        elapsed_ms,
        error_kind = error_kind(error),
    );
}

fn retry(
    trace: &OperationTrace,
    method: &Method,
    attempt: u32,
    next_attempt: u32,
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
    let retry_after_ms = error.retry_after().map(duration_ms);

    match (status, retry_after_ms) {
        (Some(status), Some(retry_after_ms)) => trace_warn!(
            trace.trace_label.as_deref(),
            event = "blooio.request.retry",
            method = method.as_str(),
            operation = trace.operation,
            attempt,
            next_attempt,
            max_retries = trace.max_retries,
            delay_ms,
            delay_source,
            error_kind,
            status,
            retry_after_ms,
        ),
        (Some(status), None) => trace_warn!(
            trace.trace_label.as_deref(),
            event = "blooio.request.retry",
            method = method.as_str(),
            operation = trace.operation,
            attempt,
            next_attempt,
            max_retries = trace.max_retries,
            delay_ms,
            delay_source,
            error_kind,
            status,
        ),
        (None, _) => trace_warn!(
            trace.trace_label.as_deref(),
            event = "blooio.request.retry",
            method = method.as_str(),
            operation = trace.operation,
            attempt,
            next_attempt,
            max_retries = trace.max_retries,
            delay_ms,
            delay_source,
            error_kind,
        ),
    }
}

fn operation_success(
    trace: &OperationTrace,
    method: &Method,
    attempts: u32,
    status: u16,
    elapsed: Duration,
) {
    trace_debug!(
        trace.trace_label.as_deref(),
        event = "blooio.operation.success",
        method = method.as_str(),
        operation = trace.operation,
        attempts,
        max_retries = trace.max_retries,
        status,
        elapsed_ms = duration_ms(elapsed),
    );
}

fn operation_failure(
    trace: &OperationTrace,
    method: &Method,
    attempts: u32,
    elapsed: Duration,
    status: Option<u16>,
    error: &Error,
) {
    let elapsed_ms = duration_ms(elapsed);
    let error_kind = error_kind(error);
    let status = status.or_else(|| error.status());
    match status {
        Some(status) => trace_warn!(
            trace.trace_label.as_deref(),
            event = "blooio.operation.failure",
            method = method.as_str(),
            operation = trace.operation,
            attempts,
            max_retries = trace.max_retries,
            elapsed_ms,
            error_kind,
            status,
        ),
        None => trace_warn!(
            trace.trace_label.as_deref(),
            event = "blooio.operation.failure",
            method = method.as_str(),
            operation = trace.operation,
            attempts,
            max_retries = trace.max_retries,
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
        Error::RequestBuild => "request_build",
        Error::ResponseBodyTooLarge { .. } => "response_body_too_large",
        Error::Encode(_) => "encode",
        Error::Decode(_) => "decode",
        Error::Config(_) => "config",
        #[cfg(feature = "webhooks")]
        Error::Webhook(_) => "webhook",
    }
}
