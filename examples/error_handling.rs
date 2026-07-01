//! Distinguishing the kinds of failure a call can return, and branching on the
//! stable machine-readable API error `code`.
//!
//! ```sh
//! BLOOIO_API_KEY=sk_... CHAT_ID=chat_... cargo run --example error_handling
//! ```

#![allow(clippy::print_stdout)]

use std::env;

use blooio::{Client, Error, error::codes};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(env::var("BLOOIO_API_KEY").unwrap_or_else(|_| "sk_demo_key".into()))?;
    let chat = client.chat(env::var("CHAT_ID").unwrap_or_else(|_| "chat_demo".into()));

    match chat.send_text("hello").await {
        Ok(resp) => println!("delivered: {:?}", resp.ids()),

        // Documented quota/cap errors are not automatically retryable.
        Err(Error::Api(api)) if api.is_quota_error() => {
            println!(
                "quota cap reached: status={} code={:?} current={:?}",
                api.status(),
                api.code(),
                api.details().get("current")
            );
        }

        // A non-2xx response. Match on `code` for stable handling rather than
        // scraping the human-readable message.
        Err(Error::Api(api)) => match api.code() {
            Some(codes::REPLY_TARGET_NOT_FOUND) => {
                println!("the message being replied to no longer exists");
            }
            Some("invalid_chat") => println!("that chat id doesn't exist"),
            _ => println!("api error {}: {api}", api.status()),
        },

        // Connection / DNS / TLS / timeout — usually worth retrying.
        Err(Error::Transport(e)) => println!("transport failure (retryable): {e}"),

        // The server sent a 2xx body we couldn't decode — a client/server
        // version skew. Not retryable on its own.
        Err(Error::Decode(e)) => println!("could not decode response: {e}"),

        Err(other) => println!("unexpected error: {other}"),
    }

    // The same information is available without destructuring, via accessors:
    if let Err(e) = client.account().get().await {
        println!("status={:?} code={:?}", e.status(), e.code());
    }

    Ok(())
}
