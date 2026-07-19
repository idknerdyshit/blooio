//! V4 contact and identity operations.
#![allow(missing_docs)]

use http::Method;
use serde::Serialize;
use serde_json::Value;

use super::impl_v4_operation;
use crate::v4::{
    CursorPaginator,
    types::{
        ActionResponse, ChannelType, Contact, ContactCapability, ContactIdentity, ItemEnvelope,
        ListEnvelope, TimelineEntry,
    },
};
use crate::{
    Result,
    core::operation::{encode_path_segment, json_body, push_opt},
};

/// Cursor-mode contact listing.
#[derive(Debug, Clone, Default)]
pub struct ListContacts {
    pub identifier: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}
impl crate::Operation for ListContacts {
    type Output = ListEnvelope<Contact>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/contacts".into()
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "identifier", self.identifier.as_ref());
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "cursor", self.cursor.as_ref());
        q
    }
}
impl_v4_operation!(ListContacts);

/// Offset-mode contact search.
#[derive(Debug, Clone, Default)]
pub struct SearchContacts {
    pub identifier: Option<String>,
    pub q: Option<String>,
    pub tag: Option<String>,
    pub sort: Option<String>,
    pub filter: Option<String>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
}
impl crate::Operation for SearchContacts {
    type Output = ListEnvelope<Contact>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/contacts".into()
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut x = Vec::new();
        push_opt(&mut x, "identifier", self.identifier.as_ref());
        push_opt(&mut x, "q", self.q.as_ref());
        push_opt(&mut x, "tag", self.tag.as_ref());
        push_opt(&mut x, "sort", self.sort.as_ref());
        push_opt(&mut x, "filter", self.filter.as_ref());
        push_opt(&mut x, "offset", self.offset);
        push_opt(&mut x, "limit", self.limit);
        x
    }
}
impl_v4_operation!(SearchContacts);

/// Create a contact.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateContact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_type: Option<ChannelType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
}
impl CreateContact {
    /// Construct a contact with its first identity.
    #[must_use]
    pub fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: Some(identifier.into()),
            ..Default::default()
        }
    }
}
impl crate::Operation for CreateContact {
    type Output = ItemEnvelope<Contact>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/contacts".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(CreateContact);

/// Get a contact.
#[derive(Debug, Clone)]
pub struct GetContact {
    pub contact_id: String,
}
impl crate::Operation for GetContact {
    type Output = ItemEnvelope<Contact>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/contacts/{}", encode_path_segment(&self.contact_id))
    }
}
impl_v4_operation!(GetContact);

/// Delete a contact.
#[derive(Debug, Clone)]
pub struct DeleteContact {
    pub contact_id: String,
}
impl crate::Operation for DeleteContact {
    type Output = ActionResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/contacts/{}", encode_path_segment(&self.contact_id))
    }
}
impl_v4_operation!(DeleteContact);

/// List identities attached to a contact.
#[derive(Debug, Clone)]
pub struct ListContactIdentities {
    pub contact_id: String,
}
impl crate::Operation for ListContactIdentities {
    type Output = ListEnvelope<ContactIdentity>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/contacts/{}/identities",
            encode_path_segment(&self.contact_id)
        )
    }
}
impl_v4_operation!(ListContactIdentities);

/// Get contact capabilities.
#[derive(Debug, Clone)]
pub struct GetContactCapabilities {
    pub contact_id: String,
}
impl crate::Operation for GetContactCapabilities {
    type Output = ListEnvelope<ContactCapability>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/contacts/{}/capabilities",
            encode_path_segment(&self.contact_id)
        )
    }
}
impl_v4_operation!(GetContactCapabilities);

/// List contact tags.
#[derive(Debug, Clone)]
pub struct ListContactTags {
    pub contact_id: String,
}
impl crate::Operation for ListContactTags {
    type Output = ListEnvelope<String>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/contacts/{}/tags", encode_path_segment(&self.contact_id))
    }
}
impl_v4_operation!(ListContactTags);

/// Update a contact.
#[derive(Debug, Clone)]
pub struct UpdateContact {
    pub contact_id: String,
    pub name: String,
}
impl crate::Operation for UpdateContact {
    type Output = ItemEnvelope<Contact>;
    const METHOD: Method = Method::PATCH;
    fn path(&self) -> String {
        format!("/contacts/{}", encode_path_segment(&self.contact_id))
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(&serde_json::json!({"name": self.name}))
    }
}
impl_v4_operation!(UpdateContact);

/// Merge another contact into the path contact.
#[derive(Debug, Clone, Serialize)]
pub struct MergeContact {
    #[serde(skip)]
    pub contact_id: String,
    pub source_contact_id: String,
}
impl crate::Operation for MergeContact {
    type Output = ItemEnvelope<Contact>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/contacts/{}/merge", encode_path_segment(&self.contact_id))
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(MergeContact);

/// Attach an identity.
#[derive(Debug, Clone, Serialize)]
pub struct AttachContactIdentity {
    #[serde(skip)]
    pub contact_id: String,
    pub identifier: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_type: Option<ChannelType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
}
impl AttachContactIdentity {
    /// Construct an identity attachment and let the service infer its channel type.
    #[must_use]
    pub fn new(contact_id: impl Into<String>, identifier: impl Into<String>) -> Self {
        Self {
            contact_id: contact_id.into(),
            identifier: identifier.into(),
            channel_type: None,
            channel_id: None,
        }
    }
}
impl crate::Operation for AttachContactIdentity {
    type Output = ItemEnvelope<ContactIdentity>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!(
            "/contacts/{}/identities",
            encode_path_segment(&self.contact_id)
        )
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(AttachContactIdentity);

/// Update an attached identity.
#[derive(Debug, Clone)]
pub struct UpdateContactIdentity {
    pub contact_id: String,
    pub identity_id: String,
    pub fields: Value,
}
impl crate::Operation for UpdateContactIdentity {
    type Output = ItemEnvelope<ContactIdentity>;
    const METHOD: Method = Method::PATCH;
    fn path(&self) -> String {
        format!(
            "/contacts/{}/identities/{}",
            encode_path_segment(&self.contact_id),
            encode_path_segment(&self.identity_id)
        )
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(&self.fields)
    }
}
impl_v4_operation!(UpdateContactIdentity);

/// Detach an identity.
#[derive(Debug, Clone)]
pub struct DetachContactIdentity {
    pub contact_id: String,
    pub identity_id: String,
}
impl crate::Operation for DetachContactIdentity {
    type Output = ActionResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!(
            "/contacts/{}/identities/{}",
            encode_path_segment(&self.contact_id),
            encode_path_segment(&self.identity_id)
        )
    }
}
impl_v4_operation!(DetachContactIdentity);

/// Get a contact timeline.
#[derive(Debug, Clone, Default)]
pub struct GetContactTimeline {
    pub contact_id: String,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
    pub include_events: Option<bool>,
}
impl crate::Operation for GetContactTimeline {
    type Output = ListEnvelope<TimelineEntry>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/contacts/{}/timeline",
            encode_path_segment(&self.contact_id)
        )
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "cursor", self.cursor.as_ref());
        push_opt(&mut q, "include_events", self.include_events);
        q
    }
}
impl_v4_operation!(GetContactTimeline);

/// Add one or more tags.
#[derive(Debug, Clone, Serialize)]
pub struct AddContactTags {
    #[serde(skip)]
    pub contact_id: String,
    pub tags: Vec<String>,
}
impl crate::Operation for AddContactTags {
    type Output = ItemEnvelope<Vec<String>>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/contacts/{}/tags", encode_path_segment(&self.contact_id))
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(AddContactTags);

/// Remove a tag.
#[derive(Debug, Clone)]
pub struct RemoveContactTag {
    pub contact_id: String,
    pub tag: String,
}
impl crate::Operation for RemoveContactTag {
    type Output = ItemEnvelope<Vec<String>>;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!(
            "/contacts/{}/tags/{}",
            encode_path_segment(&self.contact_id),
            encode_path_segment(&self.tag)
        )
    }
}
impl_v4_operation!(RemoveContactTag);

#[cfg(test)]
#[allow(clippy::assertions_on_constants, clippy::unwrap_used)]
mod tests {
    use super::{AttachContactIdentity, MergeContact};
    use crate::{Operation, v4::types::ChannelType};

    #[test]
    fn attach_identity_only_requires_an_identifier() {
        let body =
            serde_json::to_value(AttachContactIdentity::new("ct_1", "+15551234567")).unwrap();
        assert_eq!(body, serde_json::json!({"identifier": "+15551234567"}));
    }

    #[test]
    fn attach_identity_preserves_an_unknown_channel_type() {
        let mut operation = AttachContactIdentity::new("ct_1", "+15551234567");
        operation.channel_type = Some(ChannelType::Unknown("future_provider".into()));

        let body = serde_json::from_slice::<serde_json::Value>(&operation.body().unwrap().unwrap())
            .unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "identifier": "+15551234567",
                "channel_type": "future_provider"
            })
        );
    }

    #[test]
    fn merge_is_not_retried_automatically() {
        assert!(!<MergeContact as Operation>::RETRY_SAFE);
    }
}

/// Contact collection handle.
#[derive(Debug)]
pub struct Contacts<C> {
    pub(crate) client: C,
}
#[cfg(feature = "async")]
impl crate::v4::BlooioAccount<'_> {
    /// Access contacts.
    #[must_use]
    pub fn contacts(self) -> Contacts<Self> {
        Contacts { client: self }
    }
}
#[cfg(feature = "sync")]
impl crate::v4::BlockingBlooioAccount<'_> {
    /// Access contacts.
    #[must_use]
    pub fn contacts(self) -> Contacts<Self> {
        Contacts { client: self }
    }
}
#[cfg(feature = "async")]
impl<'a> Contacts<crate::v4::BlooioAccount<'a>> {
    /// List the first cursor page.
    pub async fn list(&self) -> Result<ListEnvelope<Contact>> {
        self.client.send(ListContacts::default()).await
    }
    /// List with cursor options.
    pub async fn list_with(&self, query: ListContacts) -> Result<ListEnvelope<Contact>> {
        self.client.send(query).await
    }
    /// Cursor over all contacts.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListContacts + use<'a>,
        ListContacts,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListContacts {
                cursor,
                limit: Some(limit),
                identifier: None,
            },
        )
    }
    /// Search contacts using offset pagination.
    pub async fn search(&self, query: SearchContacts) -> Result<ListEnvelope<Contact>> {
        self.client.send(query).await
    }
    /// Offset paginator for a search/filter query.
    pub fn search_all(
        &self,
        query: SearchContacts,
    ) -> crate::Paginator<
        crate::v4::BlooioAccount<'a>,
        impl Fn(u32, u32) -> SearchContacts + use<'a>,
        SearchContacts,
    > {
        crate::Paginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            move |offset, limit| SearchContacts {
                offset: Some(offset),
                limit: Some(limit),
                ..query.clone()
            },
        )
    }
    /// Create a contact.
    pub async fn create(&self, operation: CreateContact) -> Result<ItemEnvelope<Contact>> {
        self.client.send(operation).await
    }
}
#[cfg(feature = "sync")]
impl<'a> Contacts<crate::v4::BlockingBlooioAccount<'a>> {
    /// List the first cursor page.
    pub fn list(&self) -> Result<ListEnvelope<Contact>> {
        self.client.send(ListContacts::default())
    }
    /// List with cursor options.
    pub fn list_with(&self, query: ListContacts) -> Result<ListEnvelope<Contact>> {
        self.client.send(query)
    }
    /// Cursor over all contacts.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlockingBlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListContacts + use<'a>,
        ListContacts,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListContacts {
                cursor,
                limit: Some(limit),
                identifier: None,
            },
        )
    }
    /// Search contacts using offset pagination.
    pub fn search(&self, query: SearchContacts) -> Result<ListEnvelope<Contact>> {
        self.client.send(query)
    }
    /// Offset paginator for a search/filter query.
    pub fn search_all(
        &self,
        query: SearchContacts,
    ) -> crate::Paginator<
        crate::v4::BlockingBlooioAccount<'a>,
        impl Fn(u32, u32) -> SearchContacts + use<'a>,
        SearchContacts,
    > {
        crate::Paginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            move |offset, limit| SearchContacts {
                offset: Some(offset),
                limit: Some(limit),
                ..query.clone()
            },
        )
    }
    /// Create a contact.
    pub fn create(&self, operation: CreateContact) -> Result<ItemEnvelope<Contact>> {
        self.client.send(operation)
    }
}
