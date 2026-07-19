//! V4 account operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::Result;
use crate::v4::types::{Account, ItemEnvelope, Priority};
use http::Method;

/// Get the authenticated account.
#[derive(Debug, Clone, Copy)]
pub struct GetMe;
impl crate::Operation for GetMe {
    type Output = ItemEnvelope<Account>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/me".into()
    }
}
impl_v4_operation!(GetMe);

/// Get the API key's default routing priority.
#[derive(Debug, Clone, Copy)]
pub struct GetMyDefaultPriority;
impl crate::Operation for GetMyDefaultPriority {
    type Output = ItemEnvelope<Option<Priority>>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/me/priority".into()
    }
}
impl_v4_operation!(GetMyDefaultPriority);

/// Account resource handle.
#[derive(Debug)]
pub struct Me<C> {
    pub(crate) client: C,
}

#[cfg(feature = "async")]
impl crate::v4::BlooioAccount<'_> {
    /// Access account information.
    #[must_use]
    pub fn me(self) -> Me<Self> {
        Me { client: self }
    }
}
#[cfg(feature = "sync")]
impl crate::v4::BlockingBlooioAccount<'_> {
    /// Access account information.
    #[must_use]
    pub fn me(self) -> Me<Self> {
        Me { client: self }
    }
}
#[cfg(feature = "async")]
impl Me<crate::v4::BlooioAccount<'_>> {
    /// Get account information.
    pub async fn get(&self) -> Result<ItemEnvelope<Account>> {
        self.client.send(GetMe).await
    }
    /// Get the default routing priority.
    pub async fn priority(&self) -> Result<ItemEnvelope<Option<Priority>>> {
        self.client.send(GetMyDefaultPriority).await
    }
}
#[cfg(feature = "sync")]
impl Me<crate::v4::BlockingBlooioAccount<'_>> {
    /// Get account information.
    pub fn get(&self) -> Result<ItemEnvelope<Account>> {
        self.client.send(GetMe)
    }
    /// Get the default routing priority.
    pub fn priority(&self) -> Result<ItemEnvelope<Option<Priority>>> {
        self.client.send(GetMyDefaultPriority)
    }
}
