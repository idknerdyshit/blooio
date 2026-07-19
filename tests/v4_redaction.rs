//! Redaction coverage for v4 DTOs and cursor diagnostics.

#![cfg(feature = "api-v4")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout
)]

#[test]
fn webhook_creation_secret_is_redacted_in_debug() {
    let response: blooio::v4::types::ItemEnvelope<blooio::v4::types::WebhookWithSecret> =
        serde_json::from_value(serde_json::json!({
            "data": {"id":"wh_1","url":"https://example.test/hook","signing_secret":"whsec-secret"}
        }))
        .unwrap();
    assert_eq!(response.data.signing_secret.expose(), "whsec-secret");
    let debug = format!("{response:?}");
    assert!(!debug.contains("whsec-secret"));
    assert!(debug.contains("REDACTED"));
}
