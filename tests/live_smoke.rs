//! Optional live smoke test against the real Blooio API.
//!
//! Ignored by default. Run with a real key:
//!
//! ```sh
//! BLOOIO_API_KEY=sk-... cargo test --test live_smoke -- --ignored --nocapture
//! ```

#![cfg(feature = "async")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unused_async, clippy::print_stdout, clippy::unreadable_literal)]

use blooio::Client;

#[tokio::test]
#[ignore = "requires a live BLOOIO_API_KEY and network access"]
async fn account_get_smoke() {
    let key = std::env::var("BLOOIO_API_KEY")
        .expect("set BLOOIO_API_KEY to run the live smoke test");
    let client = Client::new(key).unwrap();
    let me = client.account().get().await.expect("GET /me failed");
    println!("authenticated: valid={:?} user_id={:?}", me.valid, me.user_id);
    assert_eq!(me.valid, Some(true));
}
