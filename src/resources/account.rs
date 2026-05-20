//! Account: retrieve the authenticated user's profile.

use http::Method;

use crate::core::operation::Operation;
use crate::error::Result;

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// `GET /me`
#[derive(Debug, Clone, Default)]
pub struct GetMe;

impl Operation for GetMe {
    type Output = crate::types::MeResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/me".into()
    }
}

// ---------------------------------------------------------------------------
// Resource handle + accessors.
// ---------------------------------------------------------------------------

/// Handle for the `account` resource group. Created via
/// [`Client::account`](crate::Client::account).
#[derive(Debug)]
pub struct Account<'c, C> {
    pub(crate) client: &'c C,
}

#[cfg(feature = "async")]
impl crate::Client {
    /// Access the account resource group.
    pub fn account(&self) -> Account<'_, crate::Client> {
        Account { client: self }
    }
}

#[cfg(feature = "sync")]
impl crate::BlockingClient {
    /// Access the account resource group.
    pub fn account(&self) -> Account<'_, crate::BlockingClient> {
        Account { client: self }
    }
}

#[cfg(feature = "async")]
impl Account<'_, crate::Client> {
    /// Get the authenticated user's profile.
    pub async fn get(&self) -> Result<crate::types::MeResponse> {
        self.client.send(GetMe).await
    }
}

#[cfg(feature = "sync")]
impl Account<'_, crate::BlockingClient> {
    /// Get the authenticated user's profile.
    pub fn get(&self) -> Result<crate::types::MeResponse> {
        self.client.send(GetMe)
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
    use crate::core::operation::Operation;

    #[test]
    fn get_me_method_is_get() {
        assert_eq!(GetMe::METHOD, http::Method::GET);
    }

    #[test]
    fn get_me_path() {
        assert_eq!(GetMe.path(), "/me");
    }

    #[test]
    fn get_me_query_is_empty() {
        assert!(GetMe.query().is_empty());
    }

    #[test]
    fn get_me_headers_is_empty() {
        assert!(GetMe.headers().is_empty());
    }

    #[test]
    fn get_me_body_is_none() {
        assert!(GetMe.body().unwrap().is_none());
    }
}
