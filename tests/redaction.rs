//! Secret-redaction tests: the API key must never appear in `Debug` output or
//! in any captured tracing span/event.

#![cfg(all(feature = "tracing", feature = "async"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unreadable_literal
)]

use std::io::Write;
use std::sync::{Arc, Mutex};

use blooio::{Client, ClientConfig};
use tracing_subscriber::fmt::format::FmtSpan;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

#[test]
fn debug_never_reveals_key() {
    let config = ClientConfig::new(SECRET_KEY);
    let dbg_config = format!("{config:?}");
    assert!(dbg_config.contains("[REDACTED]"));
    assert!(!dbg_config.contains(SECRET_KEY));

    let client = Client::from_config(config).unwrap();
    let dbg_client = format!("{client:?}");
    assert!(!dbg_client.contains(SECRET_KEY));
}

#[tokio::test(flavor = "multi_thread")]
async fn tracing_never_emits_the_key() {
    let buffer = Arc::new(Mutex::new(Vec::<u8>::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_writer(CaptureWriter(buffer.clone()))
        .with_max_level(tracing::Level::TRACE)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .finish();
    // Global so spans recorded on tokio worker threads are captured too.
    let _ = tracing::subscriber::set_global_default(subscriber);

    let server = MockServer::start().await;
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

    let captured = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
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
