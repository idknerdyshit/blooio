//! Location: list, get, and refresh shared location contacts.

use http::Method;
use serde::Deserialize;

use crate::core::operation::Operation;
use crate::error::Result;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response of `GET /location/contacts`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct LocationContactsResponse {
    pub friends: Vec<crate::types::ContactLocation>,
}

/// Response of `POST /location/contacts/refresh`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct RefreshLocationResponse {
    pub success: Option<bool>,
    pub friends: Option<Vec<crate::types::ContactLocation>>,
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// `GET /location/contacts`
#[derive(Debug, Clone, Default)]
pub struct ListLocationContacts;

impl Operation for ListLocationContacts {
    type Output = LocationContactsResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/location/contacts".into()
    }
}

/// `GET /location/contacts/{handle}`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct GetLocationContact {
    pub handle: String,
}

impl Operation for GetLocationContact {
    type Output = crate::types::ContactLocation;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/location/contacts/{}", self.handle)
    }
}

/// `POST /location/contacts/refresh`
#[derive(Debug, Clone, Default)]
pub struct RefreshLocationContacts;

impl Operation for RefreshLocationContacts {
    type Output = RefreshLocationResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/location/contacts/refresh".into()
    }
}

// ---------------------------------------------------------------------------
// Resource handle + accessors.
// ---------------------------------------------------------------------------

/// Handle for the `location` resource group. Created via
/// [`Client::location`](crate::Client::location).
#[derive(Debug)]
pub struct Location<'c, C> {
    pub(crate) client: &'c C,
}

#[cfg(feature = "async")]
impl crate::Client {
    /// Access the location resource group.
    pub fn location(&self) -> Location<'_, crate::Client> {
        Location { client: self }
    }
}

#[cfg(feature = "sync")]
impl crate::BlockingClient {
    /// Access the location resource group.
    pub fn location(&self) -> Location<'_, crate::BlockingClient> {
        Location { client: self }
    }
}

#[cfg(feature = "async")]
impl Location<'_, crate::Client> {
    /// List all location-sharing contacts.
    pub async fn list(&self) -> Result<LocationContactsResponse> {
        self.client.send(ListLocationContacts).await
    }

    /// Get a single location contact by handle.
    pub async fn get(&self, handle: impl Into<String>) -> Result<crate::types::ContactLocation> {
        self.client
            .send(GetLocationContact {
                handle: handle.into(),
            })
            .await
    }

    /// Refresh location contacts.
    pub async fn refresh(&self) -> Result<RefreshLocationResponse> {
        self.client.send(RefreshLocationContacts).await
    }
}

#[cfg(feature = "sync")]
impl Location<'_, crate::BlockingClient> {
    /// List all location-sharing contacts.
    pub fn list(&self) -> Result<LocationContactsResponse> {
        self.client.send(ListLocationContacts)
    }

    /// Get a single location contact by handle.
    pub fn get(&self, handle: impl Into<String>) -> Result<crate::types::ContactLocation> {
        self.client.send(GetLocationContact {
            handle: handle.into(),
        })
    }

    /// Refresh location contacts.
    pub fn refresh(&self) -> Result<RefreshLocationResponse> {
        self.client.send(RefreshLocationContacts)
    }
}
