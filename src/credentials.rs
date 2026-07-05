//! Account-scoped API credentials.

use crate::error::{Error, Result};
use crate::secret::Secret;

/// Credentials for one Blooio account-scoped API handle.
///
/// Root [`Client`](crate::Client) and [`BlockingClient`](crate::BlockingClient)
/// values do not store credentials. Pass this value to `client.account(&creds)`
/// to create a temporary authenticated handle.
#[derive(Clone, Debug)]
pub struct BlooioCreds {
    api_key: Secret<String>,
}

impl BlooioCreds {
    /// Create credentials from an API key.
    pub fn new(api_key: impl Into<Secret<String>>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }

    /// Create credentials from environment variables.
    ///
    /// Reads `BLOOIO_API_KEY` and treats an empty value as missing. The API key
    /// is never reflected in error messages.
    pub fn from_env() -> Result<Self> {
        let api_key = env_var("BLOOIO_API_KEY")?
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| Error::config("BLOOIO_API_KEY is not set"))?;
        Ok(Self::new(api_key))
    }

    pub(crate) fn bearer_header(&self) -> Secret<String> {
        Secret::new(format!("Bearer {}", self.api_key.expose()))
    }
}

fn env_var(name: &'static str) -> Result<Option<String>> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(Error::config(format!("{name} is not valid Unicode")))
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_api_key() {
        let creds = BlooioCreds::new("super-secret-key");
        let debug = format!("{creds:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("super-secret-key"));
    }
}
