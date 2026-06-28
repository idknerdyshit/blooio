//! Verifying and parsing an inbound webhook. This example is fully
//! self-contained — it runs no server and makes no network calls — so it works
//! offline:
//!
//! ```sh
//! cargo run --example webhooks
//! ```
//!
//! In a real handler, `signature_header` comes from either `Blooio-Signature`
//! or `x-blooio-signature`, and `body` is the raw, unparsed request body.
//! Always verify *before* trusting the payload.

#![allow(clippy::print_stdout)]

use blooio::webhook::{
    DEFAULT_TOLERANCE_SECS, MessageEventKind, SignatureHeader, WebhookEvent, peek, verify_preparsed,
};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let secret = b"whsec_example_secret";
    let body = br#"{"event":"message.received","protocol":"sms","message_id":"m_123","internal_id":"+15555550100","text":"Hi there!","sender":"+15555550123"}"#;

    // A `t=<unix>,v1=<hex-hmac>` header, as Blooio sends it. We generate it
    // locally so this example stays deterministic and offline.
    let timestamp = 1_700_000_000;
    let signature_header = sign(secret, timestamp, body)?;

    // Apps with org-specific webhook secrets can parse and precheck the
    // timestamp before choosing the secret.
    let signature = SignatureHeader::parse(&signature_header)?;
    signature.check_tolerance(timestamp, DEFAULT_TOLERANCE_SECS)?;

    // Peek is intentionally untrusted: use it only to select the secret.
    let peek = peek(body)?;
    println!("routing internal_id: {:?}", peek.internal_id);

    verify_preparsed(secret, &signature, body)?;
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

    let sms = event.try_into_received_sms()?;
    println!("received SMS {} from {}", sms.message_id, sms.sender);

    // Tampering invalidates the signature — verification fails closed.
    let tampered = br#"{"event":"message.received","text":"gotcha"}"#;
    match verify_preparsed(secret, &signature, tampered) {
        Ok(()) => println!("unexpected: tampered body verified"),
        Err(e) => println!("tampered body rejected: {e}"),
    }

    Ok(())
}

fn sign(secret: &[u8], timestamp: i64, body: &[u8]) -> Result<String, hmac::digest::InvalidLength> {
    let mut mac = HmacSha256::new_from_slice(secret)?;
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    Ok(format!(
        "t={timestamp},v1={}",
        hex::encode(mac.finalize().into_bytes())
    ))
}
