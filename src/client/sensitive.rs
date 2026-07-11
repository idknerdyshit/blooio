//! Shared sensitive diagnostics dispatch for the HTTP executors.

use crate::config::ClientConfig;
use crate::core::diagnostics::{
    SensitiveDiagnosticEvent, SensitiveDiagnostics, SensitiveRequestSnapshot,
    SensitiveResponseSnapshot, SensitiveTransportErrorSnapshot, SensitiveTransportErrorStage,
    emit_sensitive_trace,
};
use crate::core::options::RequestOptions;
use crate::core::raw::RawResponse;
use crate::core::request::RequestSpec;

#[derive(Debug)]
pub(crate) struct SensitiveAttempt<'a> {
    diagnostics: Option<&'a SensitiveDiagnostics>,
    sensitive_tracing: bool,
    request: Option<SensitiveRequestSnapshot>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SensitiveAttemptParts<'a> {
    pub(crate) config: &'a ClientConfig,
    pub(crate) options: &'a RequestOptions,
    pub(crate) spec: &'a RequestSpec,
    pub(crate) url: &'a str,
    pub(crate) auth_header: &'a str,
    pub(crate) operation: &'static str,
    pub(crate) attempt: u32,
    pub(crate) max_retries: u32,
}

impl<'a> SensitiveAttempt<'a> {
    pub(crate) fn new(parts: SensitiveAttemptParts<'a>) -> Self {
        let diagnostics = parts
            .options
            .sensitive_diagnostics
            .as_ref()
            .or(parts.config.sensitive_diagnostics.as_ref());
        let sensitive_tracing = parts
            .options
            .sensitive_tracing
            .unwrap_or(parts.config.sensitive_tracing);
        let request = (diagnostics.is_some() || sensitive_tracing).then(|| {
            SensitiveRequestSnapshot::from_spec(
                parts.spec,
                parts.url,
                parts.auth_header,
                parts.operation,
                parts.attempt,
                parts.max_retries,
                parts.options.trace_label.as_deref(),
            )
        });
        Self {
            diagnostics,
            sensitive_tracing,
            request,
        }
    }

    fn record(&self, event: SensitiveDiagnosticEvent) {
        if self.sensitive_tracing {
            emit_sensitive_trace(&event);
        }
        if let Some(diagnostics) = self.diagnostics {
            diagnostics.record(event);
        }
    }

    pub(crate) fn request(&self) {
        let Some(request) = &self.request else {
            return;
        };
        self.record(SensitiveDiagnosticEvent::Request(request.clone()));
    }

    pub(crate) fn response(&self, raw: &RawResponse) {
        let Some(request) = &self.request else {
            return;
        };
        self.record(SensitiveDiagnosticEvent::Response(
            SensitiveResponseSnapshot::from_raw(request, raw),
        ));
    }

    pub(crate) fn transport_error(&self, stage: SensitiveTransportErrorStage, error: String) {
        let Some(request) = &self.request else {
            return;
        };
        self.record(SensitiveDiagnosticEvent::TransportError(
            SensitiveTransportErrorSnapshot::new(request.clone(), stage, error),
        ));
    }
}
