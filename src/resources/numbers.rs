//! Numbers: list phone numbers attached to the account.

use http::Method;
use serde::Deserialize;

use crate::core::operation::Operation;
use crate::error::Result;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// A single phone number entry.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct NumberInfo {
    pub phone_number: Option<String>,
    pub is_active: Option<bool>,
    pub last_active: Option<String>,
    pub plan_kind: Option<String>,
}

/// Response of `GET /me/numbers`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ListNumbersResponse {
    pub numbers: Vec<NumberInfo>,
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// `GET /me/numbers`
#[derive(Debug, Clone, Default)]
pub struct ListNumbers;

impl Operation for ListNumbers {
    type Output = ListNumbersResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/me/numbers".into()
    }
}

// ---------------------------------------------------------------------------
// Resource handle + accessors.
// ---------------------------------------------------------------------------

/// Handle for the `numbers` resource group. Created via
/// [`Client::numbers`](crate::Client::numbers).
#[derive(Debug)]
pub struct Numbers<'c, C> {
    pub(crate) client: &'c C,
}

#[cfg(feature = "async")]
impl crate::Client {
    /// Access the numbers resource group.
    pub fn numbers(&self) -> Numbers<'_, crate::Client> {
        Numbers { client: self }
    }
}

#[cfg(feature = "sync")]
impl crate::BlockingClient {
    /// Access the numbers resource group.
    pub fn numbers(&self) -> Numbers<'_, crate::BlockingClient> {
        Numbers { client: self }
    }
}

#[cfg(feature = "async")]
impl Numbers<'_, crate::Client> {
    /// List all phone numbers on the account.
    pub async fn list(&self) -> Result<ListNumbersResponse> {
        self.client.send(ListNumbers).await
    }
}

#[cfg(feature = "sync")]
impl Numbers<'_, crate::BlockingClient> {
    /// List all phone numbers on the account.
    pub fn list(&self) -> Result<ListNumbersResponse> {
        self.client.send(ListNumbers)
    }
}
