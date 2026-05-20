//! Contacts: list/create/get/update/delete, capabilities, and tags.

use http::Method;
use serde::{Deserialize, Serialize};

use crate::core::operation::{json_body, push_opt, Operation};
use crate::core::pagination::{Listing, Page, Pagination, Paginator, DEFAULT_PAGE_SIZE};
use crate::error::Result;
use crate::types::{Contact, DeleteResponse};

// ---------------------------------------------------------------------------
// Response types specific to this resource group.
// ---------------------------------------------------------------------------

/// Response of `GET /contacts`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ListContactsResponse {
    pub contacts: Vec<Contact>,
    pub pagination: Option<Pagination>,
}

impl Listing for ListContactsResponse {
    type Item = Contact;
    fn into_page(self) -> Page<Self::Item> {
        Page {
            items: self.contacts,
            pagination: self.pagination,
        }
    }
}

/// Capabilities of a contact (iMessage/SMS/FaceTime reachability).
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ContactCapabilities {
    pub contact: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub capabilities: Option<Capabilities>,
    pub last_checked: Option<i64>,
}

/// The per-protocol capability flags.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct Capabilities {
    pub imessage: Option<bool>,
    pub sms: Option<bool>,
    pub facetime: Option<bool>,
}

/// A single contact tag with its creation time.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ContactTag {
    pub tag: Option<String>,
    pub created_at: Option<i64>,
}

/// Response of `GET /contacts/{contactId}/tags`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ContactTagsResponse {
    pub tags: Vec<ContactTag>,
}

/// Response of `POST /contacts/{contactId}/tags`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct AddTagsResponse {
    pub success: Option<bool>,
    pub tags_added: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Operations (public escape hatch — usable via `client.send(..)`).
// ---------------------------------------------------------------------------

/// `GET /contacts`
#[allow(missing_docs)]
#[derive(Debug, Clone, Default)]
pub struct ListContacts {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub q: Option<String>,
    pub sort: Option<String>,
}

impl Operation for ListContacts {
    type Output = ListContactsResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/contacts".into()
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "offset", self.offset);
        push_opt(&mut q, "q", self.q.as_ref());
        push_opt(&mut q, "sort", self.sort.as_ref());
        q
    }
}

/// `POST /contacts`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct CreateContact {
    pub identifier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Operation for CreateContact {
    type Output = Contact;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/contacts".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

/// `GET /contacts/{contactId}`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct GetContact {
    pub contact_id: String,
}

impl Operation for GetContact {
    type Output = Contact;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/contacts/{}", self.contact_id)
    }
}

/// `PATCH /contacts/{contactId}`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdateContact {
    #[serde(skip)]
    pub contact_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Operation for UpdateContact {
    type Output = Contact;
    const METHOD: Method = Method::PATCH;
    fn path(&self) -> String {
        format!("/contacts/{}", self.contact_id)
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

/// `DELETE /contacts/{contactId}`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct DeleteContact {
    pub contact_id: String,
}

impl Operation for DeleteContact {
    type Output = DeleteResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/contacts/{}", self.contact_id)
    }
}

/// `GET /contacts/{contactId}/capabilities`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct GetContactCapabilities {
    pub contact_id: String,
}

impl Operation for GetContactCapabilities {
    type Output = ContactCapabilities;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/contacts/{}/capabilities", self.contact_id)
    }
}

/// `GET /contacts/{contactId}/tags`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct ListContactTags {
    pub contact_id: String,
}

impl Operation for ListContactTags {
    type Output = ContactTagsResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/contacts/{}/tags", self.contact_id)
    }
}

/// `POST /contacts/{contactId}/tags`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct AddContactTags {
    #[serde(skip)]
    pub contact_id: String,
    pub tags: Vec<String>,
}

impl Operation for AddContactTags {
    type Output = AddTagsResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/contacts/{}/tags", self.contact_id)
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

/// `DELETE /contacts/{contactId}/tags/{tag}`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct RemoveContactTag {
    pub contact_id: String,
    pub tag: String,
}

impl Operation for RemoveContactTag {
    type Output = DeleteResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/contacts/{}/tags/{}", self.contact_id, self.tag)
    }
}

// ---------------------------------------------------------------------------
// Resource handle + accessors.
// ---------------------------------------------------------------------------

/// Handle for the `contacts` resource group. Created via
/// [`Client::contacts`](crate::Client::contacts).
#[derive(Debug)]
pub struct Contacts<'c, C> {
    pub(crate) client: &'c C,
}

#[cfg(feature = "async")]
impl crate::Client {
    /// Access the contacts resource group.
    pub fn contacts(&self) -> Contacts<'_, crate::Client> {
        Contacts { client: self }
    }
}

#[cfg(feature = "sync")]
impl crate::BlockingClient {
    /// Access the contacts resource group.
    pub fn contacts(&self) -> Contacts<'_, crate::BlockingClient> {
        Contacts { client: self }
    }
}

#[cfg(feature = "async")]
impl<'c> Contacts<'c, crate::Client> {
    /// List contacts (first page, no filters).
    pub async fn list(&self) -> Result<ListContactsResponse> {
        self.client.send(ListContacts::default()).await
    }

    /// List contacts with explicit filters/pagination.
    pub async fn list_with(&self, query: ListContacts) -> Result<ListContactsResponse> {
        self.client.send(query).await
    }

    /// A paginator over all contacts.
    pub fn list_all(
        &self,
    ) -> Paginator<'c, crate::Client, impl Fn(u32, u32) -> ListContacts, ListContacts> {
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, |offset, limit| ListContacts {
            offset: Some(offset),
            limit: Some(limit),
            ..Default::default()
        })
    }

    /// Create a contact.
    pub async fn create(
        &self,
        identifier: impl Into<String>,
        name: Option<String>,
    ) -> Result<Contact> {
        self.client
            .send(CreateContact {
                identifier: identifier.into(),
                name,
            })
            .await
    }

    /// Get a contact by id.
    pub async fn get(&self, contact_id: impl Into<String>) -> Result<Contact> {
        self.client
            .send(GetContact {
                contact_id: contact_id.into(),
            })
            .await
    }

    /// Update a contact's name.
    pub async fn update(
        &self,
        contact_id: impl Into<String>,
        name: Option<String>,
    ) -> Result<Contact> {
        self.client
            .send(UpdateContact {
                contact_id: contact_id.into(),
                name,
            })
            .await
    }

    /// Delete a contact.
    pub async fn delete(&self, contact_id: impl Into<String>) -> Result<DeleteResponse> {
        self.client
            .send(DeleteContact {
                contact_id: contact_id.into(),
            })
            .await
    }

    /// Get a contact's reachability capabilities.
    pub async fn capabilities(
        &self,
        contact_id: impl Into<String>,
    ) -> Result<ContactCapabilities> {
        self.client
            .send(GetContactCapabilities {
                contact_id: contact_id.into(),
            })
            .await
    }

    /// List a contact's tags.
    pub async fn tags(&self, contact_id: impl Into<String>) -> Result<ContactTagsResponse> {
        self.client
            .send(ListContactTags {
                contact_id: contact_id.into(),
            })
            .await
    }

    /// Add tags to a contact.
    pub async fn add_tags(
        &self,
        contact_id: impl Into<String>,
        tags: Vec<String>,
    ) -> Result<AddTagsResponse> {
        self.client
            .send(AddContactTags {
                contact_id: contact_id.into(),
                tags,
            })
            .await
    }

    /// Remove a single tag from a contact.
    pub async fn remove_tag(
        &self,
        contact_id: impl Into<String>,
        tag: impl Into<String>,
    ) -> Result<DeleteResponse> {
        self.client
            .send(RemoveContactTag {
                contact_id: contact_id.into(),
                tag: tag.into(),
            })
            .await
    }
}

#[cfg(feature = "sync")]
impl<'c> Contacts<'c, crate::BlockingClient> {
    /// List contacts (first page, no filters).
    pub fn list(&self) -> Result<ListContactsResponse> {
        self.client.send(ListContacts::default())
    }

    /// List contacts with explicit filters/pagination.
    pub fn list_with(&self, query: ListContacts) -> Result<ListContactsResponse> {
        self.client.send(query)
    }

    /// A paginator over all contacts.
    pub fn list_all(
        &self,
    ) -> Paginator<'c, crate::BlockingClient, impl Fn(u32, u32) -> ListContacts, ListContacts> {
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, |offset, limit| ListContacts {
            offset: Some(offset),
            limit: Some(limit),
            ..Default::default()
        })
    }

    /// Create a contact.
    pub fn create(&self, identifier: impl Into<String>, name: Option<String>) -> Result<Contact> {
        self.client.send(CreateContact {
            identifier: identifier.into(),
            name,
        })
    }

    /// Get a contact by id.
    pub fn get(&self, contact_id: impl Into<String>) -> Result<Contact> {
        self.client.send(GetContact {
            contact_id: contact_id.into(),
        })
    }

    /// Update a contact's name.
    pub fn update(&self, contact_id: impl Into<String>, name: Option<String>) -> Result<Contact> {
        self.client.send(UpdateContact {
            contact_id: contact_id.into(),
            name,
        })
    }

    /// Delete a contact.
    pub fn delete(&self, contact_id: impl Into<String>) -> Result<DeleteResponse> {
        self.client.send(DeleteContact {
            contact_id: contact_id.into(),
        })
    }

    /// Get a contact's reachability capabilities.
    pub fn capabilities(&self, contact_id: impl Into<String>) -> Result<ContactCapabilities> {
        self.client.send(GetContactCapabilities {
            contact_id: contact_id.into(),
        })
    }

    /// List a contact's tags.
    pub fn tags(&self, contact_id: impl Into<String>) -> Result<ContactTagsResponse> {
        self.client.send(ListContactTags {
            contact_id: contact_id.into(),
        })
    }

    /// Add tags to a contact.
    pub fn add_tags(
        &self,
        contact_id: impl Into<String>,
        tags: Vec<String>,
    ) -> Result<AddTagsResponse> {
        self.client.send(AddContactTags {
            contact_id: contact_id.into(),
            tags,
        })
    }

    /// Remove a single tag from a contact.
    pub fn remove_tag(
        &self,
        contact_id: impl Into<String>,
        tag: impl Into<String>,
    ) -> Result<DeleteResponse> {
        self.client.send(RemoveContactTag {
            contact_id: contact_id.into(),
            tag: tag.into(),
        })
    }
}
