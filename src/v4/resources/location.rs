//! V4 Find My location operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::core::operation::encode_path_segment;
use crate::v4::types::{ActionResponse, ItemEnvelope, ListEnvelope, LocationContact};
use http::Method;

/// List location contacts.
#[derive(Debug, Clone, Copy)]
pub struct ListLocationContacts;
impl crate::Operation for ListLocationContacts {
    type Output = ListEnvelope<LocationContact>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/location/contacts".into()
    }
}
impl_v4_operation!(ListLocationContacts);
/// Get one location contact.
#[derive(Debug, Clone)]
pub struct GetLocationContact {
    pub handle: String,
}
impl crate::Operation for GetLocationContact {
    type Output = ItemEnvelope<LocationContact>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/location/contacts/{}", encode_path_segment(&self.handle))
    }
}
impl_v4_operation!(GetLocationContact);
/// Refresh locations with POST.
#[derive(Debug, Clone, Copy)]
pub struct RefreshLocationContacts;
impl crate::Operation for RefreshLocationContacts {
    type Output = ActionResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/location/contacts/refresh".into()
    }
}
impl_v4_operation!(RefreshLocationContacts);
/// Refresh locations with GET.
#[derive(Debug, Clone, Copy)]
pub struct RefreshLocationContactsGet;
impl crate::Operation for RefreshLocationContactsGet {
    type Output = ActionResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/location/contacts/refresh".into()
    }
}
impl_v4_operation!(RefreshLocationContactsGet);

/// Location resource handle.
#[derive(Debug)]
pub struct Location<C> {
    pub(crate) client: C,
}
#[cfg(feature = "async")]
impl crate::v4::BlooioAccount<'_> {
    /// Access locations.
    #[must_use]
    pub fn location(self) -> Location<Self> {
        Location { client: self }
    }
}
#[cfg(feature = "sync")]
impl crate::v4::BlockingBlooioAccount<'_> {
    /// Access locations.
    #[must_use]
    pub fn location(self) -> Location<Self> {
        Location { client: self }
    }
}
#[cfg(feature = "async")]
impl Location<crate::v4::BlooioAccount<'_>> {
    /// List cached locations.
    pub async fn list(&self) -> crate::Result<ListEnvelope<LocationContact>> {
        self.client.send(ListLocationContacts).await
    }
    /// Get one cached location.
    pub async fn get(
        &self,
        handle: impl Into<String>,
    ) -> crate::Result<ItemEnvelope<LocationContact>> {
        self.client
            .send(GetLocationContact {
                handle: handle.into(),
            })
            .await
    }
    /// Trigger a refresh with POST.
    pub async fn refresh(&self) -> crate::Result<ActionResponse> {
        self.client.send(RefreshLocationContacts).await
    }
    /// Trigger a refresh with GET.
    pub async fn refresh_get(&self) -> crate::Result<ActionResponse> {
        self.client.send(RefreshLocationContactsGet).await
    }
}
#[cfg(feature = "sync")]
impl Location<crate::v4::BlockingBlooioAccount<'_>> {
    /// List cached locations.
    pub fn list(&self) -> crate::Result<ListEnvelope<LocationContact>> {
        self.client.send(ListLocationContacts)
    }
    /// Get one cached location.
    pub fn get(&self, handle: impl Into<String>) -> crate::Result<ItemEnvelope<LocationContact>> {
        self.client.send(GetLocationContact {
            handle: handle.into(),
        })
    }
    /// Trigger a refresh with POST.
    pub fn refresh(&self) -> crate::Result<ActionResponse> {
        self.client.send(RefreshLocationContacts)
    }
    /// Trigger a refresh with GET.
    pub fn refresh_get(&self) -> crate::Result<ActionResponse> {
        self.client.send(RefreshLocationContactsGet)
    }
}
