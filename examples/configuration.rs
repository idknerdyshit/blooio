//! Building a client from a full `ClientConfig` (custom base URL, timeout, and
//! User-Agent) and turning on request tracing.
//!
//! With the default `tracing` feature enabled, each request emits a
//! `blooio.request` span carrying the method, path, status, and elapsed time —
//! and never the API key, which stays redacted.
//!
//! ```sh
//! BLOOIO_API_KEY=sk_... cargo run --example configuration
//! ```

#![allow(clippy::print_stdout)]

use std::env;
use std::time::Duration;

use blooio::{Client, ClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Subscribe to traces so the client's instrumentation is visible. Without
    // the `tracing` feature this is harmless — the client just emits nothing.
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let config = ClientConfig::new(env::var("BLOOIO_API_KEY").unwrap_or_else(|_| "sk_demo_key".into()))
        // Point at a staging deployment or a local mock; trailing slashes are trimmed.
        .with_base_url("https://backend.blooio.com/v2/api")
        .with_timeout(Duration::from_secs(10))
        .with_user_agent("acme-bot/1.4 (+https://acme.example)");

    let client = Client::from_config(config)?;

    // `ClientConfig` derives a redacting `Debug`: the key prints as [REDACTED].
    println!("config: {:?}", client.config());

    let me = client.account().get().await?;
    println!("ok — user {:?}", me.user_id);

    Ok(())
}
