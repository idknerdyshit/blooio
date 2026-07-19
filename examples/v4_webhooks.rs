//! Parse and verify an API v4 webhook envelope.

use blooio::Secret;
use blooio::v4::webhook::verify_and_parse;

/// Verify and parse one raw v4 webhook request.
pub fn verify_event(raw_body: &[u8], signature: &str) -> blooio::Result<()> {
    let secret = Secret::new("whsec_example".to_owned());
    let _event = verify_and_parse(secret.expose().as_bytes(), signature, raw_body)?;
    Ok(())
}

fn main() {}
