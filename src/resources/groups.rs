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

impl CreateGroup {
    /// Create a new builder with only the required `name`.
    pub fn new(name: impl Into<String>) -> Self {
        CreateGroup {
            name: name.into(),
            chat_guid: None,
            members: None,
        }
    }

    /// Set the originating chat GUID.
    #[must_use]
    pub fn chat_guid(mut self, v: impl Into<String>) -> Self {
        self.chat_guid = Some(v.into());
        self
    }

    /// Set the initial member list.
    #[must_use]
    pub fn members(mut self, v: Vec<String>) -> Self {
        self.members = Some(v);
        self
    }
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
    ) -> Paginator<'c, crate::Client, impl Fn(u32, u32) -> ListGroups + use<'c>, ListGroups> {
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, |offset, limit| ListGroups {
            offset: Some(offset),
            limit: Some(limit),
            ..Default::default()
        })
    }

    /// Create a group. Build the request with [`CreateGroup::new`].
    pub async fn create(&self, op: CreateGroup) -> Result<Json> {
        self.client.send(op).await
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
    ) -> Paginator<'c, crate::BlockingClient, impl Fn(u32, u32) -> ListGroups + use<'c>, ListGroups>
    {
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, |offset, limit| ListGroups {
            offset: Some(offset),
            limit: Some(limit),
            ..Default::default()
        })
    }

    /// Create a group. Build the request with [`CreateGroup::new`].
    pub fn create(&self, op: CreateGroup) -> Result<Json> {
        self.client.send(op)
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
    ) -> Paginator<
        'c,
        crate::Client,
        impl Fn(u32, u32) -> ListGroupMembers + use<'c>,
        ListGroupMembers,
    > {
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
    ) -> Paginator<
        'c,
        crate::BlockingClient,
        impl Fn(u32, u32) -> ListGroupMembers + use<'c>,
        ListGroupMembers,
    > {
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

    // --- ListGroups ---

    #[test]
    fn list_groups_method_and_path() {
        assert_eq!(ListGroups::METHOD, http::Method::GET);
        let op = ListGroups::default();
        assert_eq!(op.path(), "/groups");
    }

    #[test]
    fn list_groups_query_empty_when_no_options() {
        let op = ListGroups::default();
        assert!(op.query().is_empty());
    }

    #[test]
    fn list_groups_query_with_all_options() {
        let op = ListGroups {
            limit: Some(10),
            offset: Some(20),
            q: Some("test".into()),
            sort: Some("asc".into()),
        };
        let q = op.query();
        assert!(q.contains(&("limit", "10".into())));
        assert!(q.contains(&("offset", "20".into())));
        assert!(q.contains(&("q", "test".into())));
        assert!(q.contains(&("sort", "asc".into())));
        assert_eq!(q.len(), 4);
    }

    #[test]
    fn list_groups_query_omits_unset_optionals() {
        let op = ListGroups {
            limit: Some(5),
            offset: None,
            q: None,
            sort: None,
        };
        let q = op.query();
        assert_eq!(q.len(), 1);
        assert!(q.contains(&("limit", "5".into())));
    }

    // --- CreateGroup ---

    #[test]
    fn create_group_method_and_path() {
        assert_eq!(CreateGroup::METHOD, http::Method::POST);
        let op = CreateGroup {
            name: "MyGroup".into(),
            chat_guid: None,
            members: None,
        };
        assert_eq!(op.path(), "/groups");
    }

    #[test]
    fn create_group_body_minimal() {
        let op = CreateGroup {
            name: "MyGroup".into(),
            chat_guid: None,
            members: None,
        };
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v, serde_json::json!({ "name": "MyGroup" }));
    }

    #[test]
    fn create_group_body_populated() {
        let op = CreateGroup {
            name: "MyGroup".into(),
            chat_guid: Some("chat-abc".into()),
            members: Some(vec!["m1".into(), "m2".into()]),
        };
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "name": "MyGroup",
                "chat_guid": "chat-abc",
                "members": ["m1", "m2"]
            })
        );
    }

    // --- GetGroup ---

    #[test]
    fn get_group_method_and_path() {
        assert_eq!(GetGroup::METHOD, http::Method::GET);
        let op = GetGroup {
            group_id: "g1".into(),
        };
        assert_eq!(op.path(), "/groups/g1");
    }

    // --- UpdateGroup ---

    #[test]
    fn update_group_method_and_path() {
        assert_eq!(UpdateGroup::METHOD, http::Method::PATCH);
        let op = UpdateGroup {
            group_id: "g1".into(),
            name: None,
        };
        assert_eq!(op.path(), "/groups/g1");
    }

    #[test]
    fn update_group_body_minimal_omits_name() {
        let op = UpdateGroup {
            group_id: "g1".into(),
            name: None,
        };
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v, serde_json::json!({}));
    }

    #[test]
    fn update_group_body_with_name() {
        let op = UpdateGroup {
            group_id: "g1".into(),
            name: Some("Renamed".into()),
        };
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v, serde_json::json!({ "name": "Renamed" }));
    }

    // --- DeleteGroup ---

    #[test]
    fn delete_group_method_and_path() {
        assert_eq!(DeleteGroup::METHOD, http::Method::DELETE);
        let op = DeleteGroup {
            group_id: "g1".into(),
        };
        assert_eq!(op.path(), "/groups/g1");
    }

    // --- SetGroupIcon ---

    #[test]
    fn set_group_icon_method_and_path() {
        assert_eq!(SetGroupIcon::METHOD, http::Method::POST);
        let op = SetGroupIcon {
            group_id: "g1".into(),
            icon: "https://example.com/icon.png".into(),
        };
        assert_eq!(op.path(), "/groups/g1/icon");
    }

    #[test]
    fn set_group_icon_body() {
        let op = SetGroupIcon {
            group_id: "g1".into(),
            icon: "https://example.com/icon.png".into(),
        };
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "icon": "https://example.com/icon.png" })
        );
    }

    // --- RemoveGroupIcon ---

    #[test]
    fn remove_group_icon_method_and_path() {
        assert_eq!(RemoveGroupIcon::METHOD, http::Method::DELETE);
        let op = RemoveGroupIcon {
            group_id: "g1".into(),
        };
        assert_eq!(op.path(), "/groups/g1/icon");
    }

    // --- ListGroupMembers ---

    #[test]
    fn list_group_members_method_and_path() {
        assert_eq!(ListGroupMembers::METHOD, http::Method::GET);
        let op = ListGroupMembers {
            group_id: "g1".into(),
            limit: None,
            offset: None,
        };
        assert_eq!(op.path(), "/groups/g1/members");
    }

    #[test]
    fn list_group_members_query_empty_when_no_options() {
        let op = ListGroupMembers {
            group_id: "g1".into(),
            limit: None,
            offset: None,
        };
        assert!(op.query().is_empty());
    }

    #[test]
    fn list_group_members_query_with_options() {
        let op = ListGroupMembers {
            group_id: "g1".into(),
            limit: Some(25),
            offset: Some(50),
        };
        let q = op.query();
        assert_eq!(q.len(), 2);
        assert!(q.contains(&("limit", "25".into())));
        assert!(q.contains(&("offset", "50".into())));
    }

    // --- AddGroupMember ---

    #[test]
    fn add_group_member_method_and_path() {
        assert_eq!(AddGroupMember::METHOD, http::Method::POST);
        let op = AddGroupMember {
            group_id: "g1".into(),
            contact_id: "m1".into(),
        };
        assert_eq!(op.path(), "/groups/g1/members");
    }

    #[test]
    fn add_group_member_body() {
        let op = AddGroupMember {
            group_id: "g1".into(),
            contact_id: "m1".into(),
        };
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // group_id is #[serde(skip)], only contact_id in body
        assert_eq!(v, serde_json::json!({ "contact_id": "m1" }));
    }

    // --- RemoveGroupMember ---

    #[test]
    fn remove_group_member_method_and_path() {
        assert_eq!(RemoveGroupMember::METHOD, http::Method::DELETE);
        let op = RemoveGroupMember {
            group_id: "g1".into(),
            contact_id: "m1".into(),
        };
        assert_eq!(op.path(), "/groups/g1/members/m1");
    }
}
