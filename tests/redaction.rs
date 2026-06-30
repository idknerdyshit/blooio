//! Secret-redaction tests that do not require an HTTP client executor.

#![cfg(feature = "tracing")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unreadable_literal,
    clippy::unwrap_in_result
)]

use blooio::MeResponse;

#[test]
fn account_response_api_key_is_redacted_in_debug() {
    let me: MeResponse = serde_json::from_value(serde_json::json!({
        "api_key": "sk-response-secret",
        "valid": true
    }))
    .unwrap();

    assert_eq!(
        me.api_key.as_ref().map(|secret| secret.expose().as_str()),
        Some("sk-response-secret")
    );

    let dbg = format!("{me:?}");
    assert!(dbg.contains("[REDACTED]"));
    assert!(!dbg.contains("sk-response-secret"));
}
