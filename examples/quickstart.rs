//! The smallest useful program: authenticate, confirm the key works, and send
//! a one-line iMessage/SMS.
//!
//! Run with your key in the environment:
//!
//! ```sh
//! BLOOIO_API_KEY=sk_... CHAT_ID=chat_... cargo run --example quickstart
//! ```

// Examples print their results; the library itself forbids stdout writes.
#![allow(clippy::print_stdout)]

use std::env;

use blooio::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("BLOOIO_API_KEY").unwrap_or_else(|_| "sk_demo_key".into());
    let chat_id = env::var("CHAT_ID").unwrap_or_else(|_| "chat_demo".into());

    // The async client is cheap to clone; share one across your app.
    let client = Client::new(api_key)?;

    // `GET /me` — a good liveness/credential check.
    let me = client.account().get().await?;
    println!("authenticated as user {:?}", me.user_id);

    // A resource handle is just a typed view over one chat. Building it does no
    // IO; the request happens on `.send_text(...)`.
    let chat = client.chat(chat_id);
    let sent = chat.send_text("hello from the blooio rust client").await?;

    // `ids()` borrows from the response, covering both single- and batch-sends.
    println!("sent message ids: {:?}", sent.ids());

    Ok(())
}
