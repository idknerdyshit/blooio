//! Verifying and parsing an inbound webhook. This example is fully
//! self-contained — it runs no server and makes no network calls — so it works
//! offline:
//!
//! ```sh
//! cargo run --example webhooks
//! ```
//!
//! In a real handler, `secret` comes from your webhook's signing secret,
//! `signature_header` from the `Blooio-Signature` request header, and `body`
//! is the raw, unparsed request body. Always verify *before* trusting the
//! payload.

#![allow(clippy::print_stdout)]

use blooio::webhook::{MessageEventKind, WebhookEvent, verify_at, verify_default};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let secret = b"whsec_example_secret";
    let body = br#"{"event":"message.received","message_id":"m_123","text":"Hi there!","sender":"+15555550123"}"#;

    // A `t=<unix>,v1=<hex-hmac>` header, as Blooio sends it.
    let signature_header =
        "t=1700000000,v1=a031050aeb41a7ef072aefefdece109e8ca7db26d85348b8b64a911dac4e2987";

    // Production code calls `verify_default`, which checks the signature *and*
    // that the timestamp is within ~5 minutes of now (replay protection):
    //
    //     verify_default(secret, signature_header, body)?;
    //
    // Here we pin "now" to the signed timestamp via `verify_at` so the fixed
    // vector above verifies deterministically regardless of the wall clock.
    let _ = verify_default; // (referenced for the doc note above)
    verify_at(secret, signature_header, body, 300, 1_700_000_000)?;
    println!("signature OK");

    // Only now is it safe to parse and act on the payload.
    let event = WebhookEvent::parse(body)?;
    match event.kind() {
        Some(MessageEventKind::Received) => {
            println!(
                "inbound message from {:?}: {:?}",
                event.payload.sender, event.payload.text
            );
        }
        Some(MessageEventKind::Delivered | MessageEventKind::Read) => {
            println!("status update for {:?}", event.payload.message_id);
        }
        Some(other) => println!("other event: {other:?}"),
        None => println!("payload carried no event field"),
    }

    // Tampering invalidates the signature — verification fails closed.
    let tampered = br#"{"event":"message.received","text":"gotcha"}"#;
    match verify_at(secret, signature_header, tampered, 300, 1_700_000_000) {
        Ok(()) => println!("unexpected: tampered body verified"),
        Err(e) => println!("tampered body rejected: {e}"),
    }

    Ok(())
}
