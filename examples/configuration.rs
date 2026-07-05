//! Building a client from a full `ClientConfig` (custom base URL, timeout, and
//! User-Agent) and turning on request tracing.
//!
//! With the default `tracing` feature enabled, each request emits a
//! `blooio.request` span plus structured attempt, retry, and final operation
//! events carrying the method, operation type, status, attempts, retry budget,
//! and elapsed time. URLs, paths, headers, bodies, and the API key stay
//! redacted.
//!
//! ```sh
//! BLOOIO_API_KEY=sk_... cargo run --example configuration
//! ```

#![allow(clippy::print_stdout)]

use std::env;
use std::time::Duration;

use blooio::{BlooioCreds, Client, ClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Subscribe to traces so the client's instrumentation is visible. Without
    // the `tracing` feature this is harmless — the client just emits nothing.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let config = ClientConfig::new()
        // Point at a staging deployment or a local mock; trailing slashes are trimmed.
        .with_base_url("https://backend.blooio.com/v2/api")
        .with_timeout(Duration::from_secs(10))
        .with_user_agent("acme-bot/1.4 (+https://acme.example)");

    let client = Client::from_config(config)?;
    let creds =
        BlooioCreds::new(env::var("BLOOIO_API_KEY").unwrap_or_else(|_| "sk_demo_key".into()));
    let account = client.account(&creds);

    // `ClientConfig` contains transport settings only; credentials stay separate.
    println!("config: {:?}", client.config());

    let me = account.me().get().await?;
    println!("ok — user {:?}", me.user_id);

    Ok(())
}
