//! Groups: list/create/get/update/delete, icon, and members.

use http::Method;
use serde::{Deserialize, Serialize};

use crate::core::operation::{Operation, json_body, push_opt};
use crate::core::pagination::{DEFAULT_PAGE_SIZE, Listing, Page, Pagination, Paginator};
use crate::error::Result;
use crate::types::{DeleteResponse, Group, GroupIconResponse, GroupMember, Json};

// ---------------------------------------------------------------------------
// Response types specific to this resource group.
// ---------------------------------------------------------------------------

/// Response of `GET /groups`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ListGroupsResponse {
    pub groups: Vec<Group>,
    pub pagination: Option<Pagination>,
}

impl Listing for ListGroupsResponse {
    type Item = Group;
    fn into_page(self) -> Page<Self::Item> {
        Page {
            items: self.groups,
            pagination: self.pagination,
        }
    }
}

/// Response of `GET /groups/{groupId}/members`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ListGroupMembersResponse {
    pub group_id: Option<String>,
    pub group_name: Option<String>,
    pub icon_url: Option<String>,
    pub members: Vec<GroupMember>,
    pub pagination: Option<Pagination>,
}

impl Listing for ListGroupMembersResponse {
    type Item = GroupMember;
    fn into_page(self) -> Page<Self::Item> {
        Page {
            items: self.members,
            pagination: self.pagination,
        }
    }
}

/// Response of `POST /groups/{groupId}/members`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct AddGroupMemberResponse {
    pub message: Option<String>,
    pub member: Option<GroupMember>,
    pub contact_created: Option<bool>,
}

/// Response of `DELETE /groups/{groupId}/members/{contactId}`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct RemoveGroupMemberResponse {
    pub success: Option<bool>,
    pub removed_at: Option<i64>,
}

// ---------------------------------------------------------------------------
// Operations (public escape hatch — usable via `client.send(..)`).
// ---------------------------------------------------------------------------

/// `GET /groups`
#[allow(missing_docs)]
#[derive(Debug, Clone, Default)]
pub struct ListGroups {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub q: Option<String>,
    pub sort: Option<String>,
}

impl Operation for ListGroups {
    type Output = ListGroupsResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/groups".into()
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

/// `POST /groups`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct CreateGroup {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<String>>,
}

impl Operation for CreateGroup {
    type Output = Json;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/groups".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

/// `GET /groups/{groupId}`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct GetGroup {
    pub group_id: String,
}

impl Operation for GetGroup {
    type Output = Group;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/groups/{}", self.group_id)
    }
}

/// `PATCH /groups/{groupId}`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdateGroup {
    #[serde(skip)]
    pub group_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Operation for UpdateGroup {
    type Output = Json;
    const METHOD: Method = Method::PATCH;
    fn path(&self) -> String {
        format!("/groups/{}", self.group_id)
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

/// `DELETE /groups/{groupId}`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct DeleteGroup {
    pub group_id: String,
}

impl Operation for DeleteGroup {
    type Output = DeleteResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/groups/{}", self.group_id)
    }
}

/// `POST /groups/{groupId}/icon`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct SetGroupIcon {
    #[serde(skip)]
    pub group_id: String,
    pub icon: String,
}

impl Operation for SetGroupIcon {
    type Output = GroupIconResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/groups/{}/icon", self.group_id)
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

/// `DELETE /groups/{groupId}/icon`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct RemoveGroupIcon {
    pub group_id: String,
}

impl Operation for RemoveGroupIcon {
    type Output = GroupIconResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/groups/{}/icon", self.group_id)
    }
}

/// `GET /groups/{groupId}/members`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct ListGroupMembers {
    pub group_id: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

impl Operation for ListGroupMembers {
    type Output = ListGroupMembersResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/groups/{}/members", self.group_id)
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "offset", self.offset);
        q
    }
}

/// `POST /groups/{groupId}/members`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct AddGroupMember {
    #[serde(skip)]
    pub group_id: String,
    pub contact_id: String,
}

impl Operation for AddGroupMember {
    type Output = AddGroupMemberResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/groups/{}/members", self.group_id)
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

/// `DELETE /groups/{groupId}/members/{contactId}`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct RemoveGroupMember {
    pub group_id: String,
    pub contact_id: String,
}

impl Operation for RemoveGroupMember {
    type Output = RemoveGroupMemberResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/groups/{}/members/{}", self.group_id, self.contact_id)
    }
}

// ---------------------------------------------------------------------------
// Resource handles + accessors.
// ---------------------------------------------------------------------------

/// Handle for the `groups` resource group. Created via
/// [`Client::groups`](crate::Client::groups).
#[derive(Debug)]
pub struct Groups<'c, C> {
    pub(crate) client: &'c C,
}

/// Handle for the `groups/{groupId}/members` sub-resource. Created via
/// [`Groups::members`].
#[derive(Debug)]
pub struct GroupMembers<'c, C> {
    pub(crate) client: &'c C,
    pub(crate) group_id: String,
}

#[cfg(feature = "async")]
impl crate::Client {
    /// Access the groups resource group.
    pub fn groups(&self) -> Groups<'_, crate::Client> {
        Groups { client: self }
    }
}

#[cfg(feature = "sync")]
impl crate::BlockingClient {
    /// Access the groups resource group.
    pub fn groups(&self) -> Groups<'_, crate::BlockingClient> {
        Groups { client: self }
    }
}

#[cfg(feature = "async")]
impl<'c> Groups<'c, crate::Client> {
    /// List groups (first page, no filters).
    pub async fn list(&self) -> Result<ListGroupsResponse> {
        self.client.send(ListGroups::default()).await
    }

    /// List groups with explicit filters/pagination.
    pub async fn list_with(&self, query: ListGroups) -> Result<ListGroupsResponse> {
        self.client.send(query).await
    }

    /// A paginator over all groups.
    pub fn list_all(
        &self,
    ) -> Paginator<'c, crate::Client, impl Fn(u32, u32) -> ListGroups, ListGroups> {
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, |offset, limit| ListGroups {
            offset: Some(offset),
            limit: Some(limit),
            ..Default::default()
        })
    }

    /// Create a group.
    pub async fn create(
        &self,
        name: impl Into<String>,
        chat_guid: Option<String>,
        members: Option<Vec<String>>,
    ) -> Result<Json> {
        self.client
            .send(CreateGroup {
                name: name.into(),
                chat_guid,
                members,
            })
            .await
    }

    /// Get a group by id.
    pub async fn get(&self, group_id: impl Into<String>) -> Result<Group> {
        self.client
            .send(GetGroup {
                group_id: group_id.into(),
            })
            .await
    }

    /// Update a group's name.
    pub async fn update(&self, group_id: impl Into<String>, name: Option<String>) -> Result<Json> {
        self.client
            .send(UpdateGroup {
                group_id: group_id.into(),
                name,
            })
            .await
    }

    /// Delete a group.
    pub async fn delete(&self, group_id: impl Into<String>) -> Result<DeleteResponse> {
        self.client
            .send(DeleteGroup {
                group_id: group_id.into(),
            })
            .await
    }

    /// Set the group icon.
    pub async fn set_icon(
        &self,
        group_id: impl Into<String>,
        icon: impl Into<String>,
    ) -> Result<GroupIconResponse> {
        self.client
            .send(SetGroupIcon {
                group_id: group_id.into(),
                icon: icon.into(),
            })
            .await
    }

    /// Remove the group icon.
    pub async fn remove_icon(&self, group_id: impl Into<String>) -> Result<GroupIconResponse> {
        self.client
            .send(RemoveGroupIcon {
                group_id: group_id.into(),
            })
            .await
    }

    /// Access the members sub-resource for a group.
    pub fn members(&self, group_id: impl Into<String>) -> GroupMembers<'c, crate::Client> {
        GroupMembers {
            client: self.client,
            group_id: group_id.into(),
        }
    }
}

#[cfg(feature = "sync")]
impl<'c> Groups<'c, crate::BlockingClient> {
    /// List groups (first page, no filters).
    pub fn list(&self) -> Result<ListGroupsResponse> {
        self.client.send(ListGroups::default())
    }

    /// List groups with explicit filters/pagination.
    pub fn list_with(&self, query: ListGroups) -> Result<ListGroupsResponse> {
        self.client.send(query)
    }

    /// A paginator over all groups.
    pub fn list_all(
        &self,
    ) -> Paginator<'c, crate::BlockingClient, impl Fn(u32, u32) -> ListGroups, ListGroups> {
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, |offset, limit| ListGroups {
            offset: Some(offset),
            limit: Some(limit),
            ..Default::default()
        })
    }

    /// Create a group.
    pub fn create(
        &self,
        name: impl Into<String>,
        chat_guid: Option<String>,
        members: Option<Vec<String>>,
    ) -> Result<Json> {
        self.client.send(CreateGroup {
            name: name.into(),
            chat_guid,
            members,
        })
    }

    /// Get a group by id.
    pub fn get(&self, group_id: impl Into<String>) -> Result<Group> {
        self.client.send(GetGroup {
            group_id: group_id.into(),
        })
    }

    /// Update a group's name.
    pub fn update(&self, group_id: impl Into<String>, name: Option<String>) -> Result<Json> {
        self.client.send(UpdateGroup {
            group_id: group_id.into(),
            name,
        })
    }

    /// Delete a group.
    pub fn delete(&self, group_id: impl Into<String>) -> Result<DeleteResponse> {
        self.client.send(DeleteGroup {
            group_id: group_id.into(),
        })
    }

    /// Set the group icon.
    pub fn set_icon(
        &self,
        group_id: impl Into<String>,
        icon: impl Into<String>,
    ) -> Result<GroupIconResponse> {
        self.client.send(SetGroupIcon {
            group_id: group_id.into(),
            icon: icon.into(),
        })
    }

    /// Remove the group icon.
    pub fn remove_icon(&self, group_id: impl Into<String>) -> Result<GroupIconResponse> {
        self.client.send(RemoveGroupIcon {
            group_id: group_id.into(),
        })
    }

    /// Access the members sub-resource for a group.
    pub fn members(&self, group_id: impl Into<String>) -> GroupMembers<'c, crate::BlockingClient> {
        GroupMembers {
            client: self.client,
            group_id: group_id.into(),
        }
    }
}

#[cfg(feature = "async")]
impl<'c> GroupMembers<'c, crate::Client> {
    /// List members of this group (first page).
    pub async fn list(&self) -> Result<ListGroupMembersResponse> {
        self.client
            .send(ListGroupMembers {
                group_id: self.group_id.clone(),
                limit: None,
                offset: None,
            })
            .await
    }

    /// A paginator over all members of this group.
    pub fn list_all(
        &self,
    ) -> Paginator<'c, crate::Client, impl Fn(u32, u32) -> ListGroupMembers, ListGroupMembers> {
        let group_id = self.group_id.clone();
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, move |offset, limit| {
            ListGroupMembers {
                group_id: group_id.clone(),
                offset: Some(offset),
                limit: Some(limit),
            }
        })
    }

    /// Add a member to this group.
    pub async fn add(&self, contact_id: impl Into<String>) -> Result<AddGroupMemberResponse> {
        self.client
            .send(AddGroupMember {
                group_id: self.group_id.clone(),
                contact_id: contact_id.into(),
            })
            .await
    }

    /// Remove a member from this group.
    pub async fn remove(&self, contact_id: impl Into<String>) -> Result<RemoveGroupMemberResponse> {
        self.client
            .send(RemoveGroupMember {
                group_id: self.group_id.clone(),
                contact_id: contact_id.into(),
            })
            .await
    }
}

#[cfg(feature = "sync")]
impl<'c> GroupMembers<'c, crate::BlockingClient> {
    /// List members of this group (first page).
    pub fn list(&self) -> Result<ListGroupMembersResponse> {
        self.client.send(ListGroupMembers {
            group_id: self.group_id.clone(),
            limit: None,
            offset: None,
        })
    }

    /// A paginator over all members of this group.
    pub fn list_all(
        &self,
    ) -> Paginator<'c, crate::BlockingClient, impl Fn(u32, u32) -> ListGroupMembers, ListGroupMembers>
    {
        let group_id = self.group_id.clone();
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, move |offset, limit| {
            ListGroupMembers {
                group_id: group_id.clone(),
                offset: Some(offset),
                limit: Some(limit),
            }
        })
    }

    /// Add a member to this group.
    pub fn add(&self, contact_id: impl Into<String>) -> Result<AddGroupMemberResponse> {
        self.client.send(AddGroupMember {
            group_id: self.group_id.clone(),
            contact_id: contact_id.into(),
        })
    }

    /// Remove a member from this group.
    pub fn remove(&self, contact_id: impl Into<String>) -> Result<RemoveGroupMemberResponse> {
        self.client.send(RemoveGroupMember {
            group_id: self.group_id.clone(),
            contact_id: contact_id.into(),
        })
    }
}
