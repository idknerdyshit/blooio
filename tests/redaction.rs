//! Secret-redaction tests: the API key must never appear in `Debug` output or
//! in any captured tracing span/event.

#![cfg(feature = "tracing")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unreadable_literal
)]

use std::io::Write;
use std::sync::{Arc, Mutex};

#[cfg(feature = "sync")]
use blooio::BlockingClient;
#[cfg(feature = "async")]
use blooio::Client;
use blooio::resources::webhooks::{CreateWebhookResponse, RotateSecretResponse};
use blooio::{ClientConfig, MeResponse};
#[cfg(feature = "sync")]
use httpmock::prelude::{GET, MockServer as HttpMockServer};
use tracing_subscriber::fmt::format::FmtSpan;
#[cfg(feature = "async")]
use wiremock::matchers::{method, path};
#[cfg(feature = "async")]
use wiremock::{Mock, MockServer as WireMockServer, ResponseTemplate};

const SECRET_KEY: &str = "sk-DO-NOT-LEAK-12345";

/// A `MakeWriter` that appends everything to a shared buffer.
#[derive(Clone)]
struct CaptureWriter(Arc<Mutex<Vec<u8>>>);

impl Write for CaptureWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CaptureWriter {
    type Writer = CaptureWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn capture_subscriber() -> (Arc<Mutex<Vec<u8>>>, impl tracing::Subscriber + Send + Sync) {
    let buffer = Arc::new(Mutex::new(Vec::<u8>::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_writer(CaptureWriter(buffer.clone()))
        .with_max_level(tracing::Level::TRACE)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .finish();
    (buffer, subscriber)
}

fn captured_string(buffer: &Arc<Mutex<Vec<u8>>>) -> String {
    String::from_utf8(buffer.lock().unwrap().clone()).unwrap()
}

fn assert_trace_is_redacted(captured: &str) {
    // We did emit a request span/event...
    assert!(
        captured.contains("blooio.request"),
        "expected request span to be traced"
    );
    // ...but never the secret or the Authorization header value.
    assert!(
        !captured.contains(SECRET_KEY),
        "API key leaked into tracing output:\n{captured}"
    );
    assert!(
        !captured.to_lowercase().contains("bearer "),
        "Authorization header leaked into tracing"
    );
}

#[test]
fn debug_never_reveals_key() {
    let config = ClientConfig::new(SECRET_KEY);
    let dbg_config = format!("{config:?}");
    assert!(dbg_config.contains("[REDACTED]"));
    assert!(!dbg_config.contains(SECRET_KEY));

    #[cfg(feature = "async")]
    {
        let client = Client::from_config(config.clone()).unwrap();
        let dbg_client = format!("{client:?}");
        assert!(!dbg_client.contains(SECRET_KEY));
    }

    #[cfg(feature = "sync")]
    {
        let client = BlockingClient::from_config(config).unwrap();
        let dbg_client = format!("{client:?}");
        assert!(!dbg_client.contains(SECRET_KEY));
    }
}

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

#[test]
fn webhook_signing_secrets_are_redacted_in_debug() {
    let created: CreateWebhookResponse =
        serde_json::from_value(serde_json::json!({ "signing_secret": "whsec-create-secret" }))
            .unwrap();
    let rotated: RotateSecretResponse =
        serde_json::from_value(serde_json::json!({ "signing_secret": "whsec-rotate-secret" }))
            .unwrap();

    assert_eq!(
        created
            .signing_secret
            .as_ref()
            .map(|secret| secret.expose().as_str()),
        Some("whsec-create-secret")
    );
    assert_eq!(
        rotated
            .signing_secret
            .as_ref()
            .map(|secret| secret.expose().as_str()),
        Some("whsec-rotate-secret")
    );

    let created_dbg = format!("{created:?}");
    let rotated_dbg = format!("{rotated:?}");
    assert!(created_dbg.contains("[REDACTED]"));
    assert!(rotated_dbg.contains("[REDACTED]"));
    assert!(!created_dbg.contains("whsec-create-secret"));
    assert!(!rotated_dbg.contains("whsec-rotate-secret"));
}

#[cfg(feature = "async")]
#[tokio::test]
async fn async_tracing_never_emits_the_key() {
    let (buffer, subscriber) = capture_subscriber();
    let _guard = tracing::subscriber::set_default(subscriber);

    let server = WireMockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/me"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "valid": true })),
        )
        .mount(&server)
        .await;

    let client =
        Client::from_config(ClientConfig::new(SECRET_KEY).with_base_url(server.uri())).unwrap();
    let _ = client.account().get().await.unwrap();

    assert_trace_is_redacted(&captured_string(&buffer));
}

#[cfg(feature = "sync")]
#[test]
fn blocking_tracing_never_emits_the_key() {
    let (buffer, subscriber) = capture_subscriber();
    let _guard = tracing::subscriber::set_default(subscriber);

    let server = HttpMockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/me");
        then.status(200)
            .json_body(serde_json::json!({ "valid": true }));
    });

    let client =
        BlockingClient::from_config(ClientConfig::new(SECRET_KEY).with_base_url(server.base_url()))
            .unwrap();
    let _ = client.account().get().unwrap();

    mock.assert();
    assert_trace_is_redacted(&captured_string(&buffer));
}
