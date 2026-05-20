//! The same API without an async runtime, via `BlockingClient` (ureq).
//!
//! Requires the `sync` feature:
//!
//! ```sh
//! BLOOIO_API_KEY=sk_... cargo run --example blocking --features sync
//! ```

#![allow(clippy::print_stdout)]

use std::env;

use blooio::BlockingClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client =
        BlockingClient::new(env::var("BLOOIO_API_KEY").unwrap_or_else(|_| "sk_demo_key".into()))?;

    // Mirror of the async surface — same method names, no `.await`.
    let numbers = client.numbers().list()?;
    println!("{} sending number(s) on this account", numbers.numbers.len());

    // The blocking paginator is an `Iterator`, so a plain `for` loop walks
    // every page; each item is a `Result` you can `?` on.
    for page in client.contacts().list_all() {
        for contact in page? {
            println!("  contact {:?}", contact.identifier);
        }
    }

    if let Ok(chat_id) = env::var("CHAT_ID") {
        let sent = client.chat(chat_id).send_text("sent from a blocking thread")?;
        println!("sent: {:?}", sent.ids());
    }

    Ok(())
}
