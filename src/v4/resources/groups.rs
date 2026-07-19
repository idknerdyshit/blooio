//! V4 group operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::v4::{
    CursorPaginator,
    types::{ActionResponse, Group, GroupMember, ItemEnvelope, ListEnvelope},
};
use crate::{
    Result,
    core::{
        multipart,
        operation::{encode_path_segment, json_body, push_opt},
    },
};
use http::Method;
use serde::Serialize;
use serde_json::Value;

/// List groups.
#[derive(Debug, Clone, Default)]
pub struct ListGroups {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}
impl crate::Operation for ListGroups {
    type Output = ListEnvelope<Group>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/groups".into()
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "cursor", self.cursor.as_ref());
        q
    }
}
impl_v4_operation!(ListGroups);

/// Create a group.
#[derive(Debug, Clone, Serialize)]
pub struct CreateGroup {
    pub channel_id: String,
    pub members: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
impl CreateGroup {
    /// Construct a group request.
    #[must_use]
    pub fn new(
        channel_id: impl Into<String>,
        members: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            channel_id: channel_id.into(),
            members: members
                .into_iter()
                .map(|v| Value::String(v.into()))
                .collect(),
            name: None,
        }
    }
}
impl crate::Operation for CreateGroup {
    type Output = ItemEnvelope<Group>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/groups".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(CreateGroup);

/// Get a group.
#[derive(Debug, Clone)]
pub struct GetGroup {
    pub group_id: String,
}
impl crate::Operation for GetGroup {
    type Output = ItemEnvelope<Group>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/groups/{}", encode_path_segment(&self.group_id))
    }
}
impl_v4_operation!(GetGroup);
/// Update a group.
#[derive(Debug, Clone)]
pub struct UpdateGroup {
    pub group_id: String,
    pub name: String,
}
impl crate::Operation for UpdateGroup {
    type Output = ItemEnvelope<Group>;
    const METHOD: Method = Method::PATCH;
    fn path(&self) -> String {
        format!("/groups/{}", encode_path_segment(&self.group_id))
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(&serde_json::json!({"name": self.name}))
    }
}
impl_v4_operation!(UpdateGroup);
/// Delete a group.
#[derive(Debug, Clone)]
pub struct DeleteGroup {
    pub group_id: String,
}
impl crate::Operation for DeleteGroup {
    type Output = ActionResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/groups/{}", encode_path_segment(&self.group_id))
    }
}
impl_v4_operation!(DeleteGroup);
/// List group members.
#[derive(Debug, Clone)]
pub struct ListGroupMembers {
    pub group_id: String,
}
impl crate::Operation for ListGroupMembers {
    type Output = ListEnvelope<GroupMember>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/groups/{}/members", encode_path_segment(&self.group_id))
    }
}
impl_v4_operation!(ListGroupMembers);

/// Set a group icon.
#[derive(Debug, Clone)]
pub struct SetGroupIcon {
    pub group_id: String,
    icon: Vec<u8>,
    filename: String,
    content_type: String,
}
impl SetGroupIcon {
    /// Construct an icon upload.
    #[must_use]
    pub fn new(group_id: impl Into<String>, icon: impl Into<Vec<u8>>) -> Self {
        Self {
            group_id: group_id.into(),
            icon: icon.into(),
            filename: "icon.png".into(),
            content_type: "image/png".into(),
        }
    }
}
impl crate::Operation for SetGroupIcon {
    type Output = ActionResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/groups/{}/icon", encode_path_segment(&self.group_id))
    }
    fn headers(&self) -> Vec<(&'static str, String)> {
        let boundary = multipart::boundary_for(&self.icon);
        vec![(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )]
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        let boundary = multipart::boundary_for(&self.icon);
        let content_type = multipart::part_content_type(Some(&self.content_type))?;
        Ok(Some(multipart::file_body(
            &boundary,
            "icon",
            &self.icon,
            Some(&self.filename),
            content_type,
        )))
    }
}
impl_v4_operation!(SetGroupIcon);
/// Remove a group icon.
#[derive(Debug, Clone)]
pub struct RemoveGroupIcon {
    pub group_id: String,
}
impl crate::Operation for RemoveGroupIcon {
    type Output = ActionResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/groups/{}/icon", encode_path_segment(&self.group_id))
    }
}
impl_v4_operation!(RemoveGroupIcon);

/// Group collection handle.
#[derive(Debug)]
pub struct Groups<C> {
    pub(crate) client: C,
}
#[cfg(feature = "async")]
impl crate::v4::BlooioAccount<'_> {
    /// Access groups.
    #[must_use]
    pub fn groups(self) -> Groups<Self> {
        Groups { client: self }
    }
}
#[cfg(feature = "sync")]
impl crate::v4::BlockingBlooioAccount<'_> {
    /// Access groups.
    #[must_use]
    pub fn groups(self) -> Groups<Self> {
        Groups { client: self }
    }
}
#[cfg(feature = "async")]
impl<'a> Groups<crate::v4::BlooioAccount<'a>> {
    /// List groups.
    pub async fn list(&self) -> Result<ListEnvelope<Group>> {
        self.client.send(ListGroups::default()).await
    }
    /// Cursor over groups.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListGroups + use<'a>,
        ListGroups,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListGroups {
                cursor,
                limit: Some(limit),
            },
        )
    }
    /// Create a group.
    pub async fn create(&self, operation: CreateGroup) -> Result<ItemEnvelope<Group>> {
        self.client.send(operation).await
    }
}
#[cfg(feature = "sync")]
impl<'a> Groups<crate::v4::BlockingBlooioAccount<'a>> {
    /// List groups.
    pub fn list(&self) -> Result<ListEnvelope<Group>> {
        self.client.send(ListGroups::default())
    }
    /// Cursor over groups.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlockingBlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListGroups + use<'a>,
        ListGroups,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListGroups {
                cursor,
                limit: Some(limit),
            },
        )
    }
    /// Create a group.
    pub fn create(&self, operation: CreateGroup) -> Result<ItemEnvelope<Group>> {
        self.client.send(operation)
    }
}
