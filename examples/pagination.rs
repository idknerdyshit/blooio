//! Two ways to walk a paginated list endpoint: drain everything at once, or
//! stream a page at a time and stop early.
//!
//! ```sh
//! BLOOIO_API_KEY=sk_... cargo run --example pagination
//! ```

#![allow(clippy::print_stdout)]

use std::env;

use blooio::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(env::var("BLOOIO_API_KEY").unwrap_or_else(|_| "sk_demo_key".into()))?;

    // Simplest: `*_all` returns a paginator; `collect_all` follows every page
    // and hands back one flat Vec.
    let everyone = client.contacts().list_all().collect_all().await?;
    println!("{} contacts total", everyone.len());

    // Streaming: fetch one page at a time and bail as soon as you've seen
    // enough — handy for large accounts where you don't want it all in memory.
    let mut pager = client.contacts().list_all();
    let mut seen = 0usize;
    while let Some(page) = pager.next_page().await {
        let page = page?; // each page is its own Result
        for contact in &page {
            println!("  {:?} — {:?}", contact.id, contact.identifier);
        }
        seen += page.len();
        if seen >= 100 {
            break;
        }
    }

    // A single page with explicit filters goes through `list_with`.
    let first = client
        .chats()
        .list_with(blooio::resources::chats::ListChats {
            limit: Some(10),
            ..Default::default()
        })
        .await?;
    println!("first page returned {} chats", first.chats.len());

    Ok(())
}
