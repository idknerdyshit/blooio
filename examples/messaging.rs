//! Richer outbound messaging: the `SendMessage` builder, send effects, an
//! idempotency key for safe retries, reactions, and a poll.
//!
//! ```sh
//! BLOOIO_API_KEY=sk_... CHAT_ID=chat_... cargo run --example messaging
//! ```

#![allow(clippy::print_stdout)]

use std::env;

use blooio::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(env::var("BLOOIO_API_KEY").unwrap_or_else(|_| "sk_demo_key".into()))?;
    let chat = client.chat(env::var("CHAT_ID").unwrap_or_else(|_| "chat_demo".into()));

    // `chat.message()` starts a builder pre-bound to this chat. Each setter
    // takes `self`, so they chain. Nothing is sent until `chat.send(..)`.
    let message = chat
        .message()
        .text("Launch is live 🚀")
        .effect("slam")
        .use_typing_indicator(true)
        // An idempotency key lets you retry a failed send without
        // double-delivering — the server dedupes on it.
        .idempotency_key("launch-announcement-2026-05-19");

    let sent = chat.send(message).await?;
    println!("sent: {:?}", sent.ids());

    // React to the message we just sent (direction defaults to outbound).
    if let Some(id) = sent.ids().first() {
        let reaction = chat.add_reaction(*id, "love", None).await?;
        println!("reaction applied: {reaction:?}");
    }

    // Polls take an optional title and the list of options.
    let poll = chat
        .send_poll(Some("Ship it?".into()), ["Yes", "Needs work"])
        .await?;
    println!("poll sent: {poll:?}");

    Ok(())
}
