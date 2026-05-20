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
