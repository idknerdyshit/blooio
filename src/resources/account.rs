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

/// Handle for the authenticated account profile resource. Created via
/// [`BlooioAccount::me`](crate::BlooioAccount::me).
#[derive(Debug)]
pub struct Me<C> {
    pub(crate) client: C,
}

#[cfg(feature = "async")]
impl<'a> crate::BlooioAccount<'a> {
    /// Access the authenticated account profile resource.
    #[must_use]
    pub fn me(self) -> Me<crate::BlooioAccount<'a>> {
        Me { client: self }
    }
}

#[cfg(feature = "sync")]
impl<'a> crate::BlockingBlooioAccount<'a> {
    /// Access the authenticated account profile resource.
    #[must_use]
    pub fn me(self) -> Me<crate::BlockingBlooioAccount<'a>> {
        Me { client: self }
    }
}

#[cfg(feature = "async")]
impl Me<crate::BlooioAccount<'_>> {
    /// Get the authenticated user's profile.
    pub async fn get(&self) -> Result<crate::types::MeResponse> {
        self.client.send(GetMe).await
    }
}

#[cfg(feature = "sync")]
impl Me<crate::BlockingBlooioAccount<'_>> {
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
    fn get_me_method_and_path() {
        assert_eq!(GetMe::METHOD, http::Method::GET);

        assert_eq!(GetMe.path(), "/me");
    }
}
