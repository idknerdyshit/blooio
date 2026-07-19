//! V4 routing-priority operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::v4::{
    CursorPaginator,
    types::{ActionResponse, ItemEnvelope, ListEnvelope, Priority},
};
use crate::{
    Result,
    core::operation::{encode_path_segment, json_body, push_opt},
};
use http::Method;
use serde_json::Value;

/// List priorities.
#[derive(Debug, Clone, Default)]
pub struct ListPriorities {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}
impl crate::Operation for ListPriorities {
    type Output = ListEnvelope<Priority>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/priorities".into()
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "cursor", self.cursor.as_ref());
        q
    }
}
impl_v4_operation!(ListPriorities);
/// Create a priority.
#[derive(Debug, Clone)]
pub struct CreatePriority {
    pub fields: Value,
}
impl crate::Operation for CreatePriority {
    type Output = ItemEnvelope<Priority>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/priorities".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(&self.fields)
    }
}
impl_v4_operation!(CreatePriority);
/// Get a priority.
#[derive(Debug, Clone)]
pub struct GetPriority {
    pub priority_id: String,
}
impl crate::Operation for GetPriority {
    type Output = ItemEnvelope<Priority>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/priorities/{}", encode_path_segment(&self.priority_id))
    }
}
impl_v4_operation!(GetPriority);
/// Update a priority.
#[derive(Debug, Clone)]
pub struct UpdatePriority {
    pub priority_id: String,
    pub fields: Value,
}
impl crate::Operation for UpdatePriority {
    type Output = ItemEnvelope<Priority>;
    const METHOD: Method = Method::PATCH;
    fn path(&self) -> String {
        format!("/priorities/{}", encode_path_segment(&self.priority_id))
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(&self.fields)
    }
}
impl_v4_operation!(UpdatePriority);
/// Delete a priority.
#[derive(Debug, Clone)]
pub struct DeletePriority {
    pub priority_id: String,
}
impl crate::Operation for DeletePriority {
    type Output = ActionResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/priorities/{}", encode_path_segment(&self.priority_id))
    }
}
impl_v4_operation!(DeletePriority);

/// Priority collection handle.
#[derive(Debug)]
pub struct Priorities<C> {
    pub(crate) client: C,
}
#[cfg(feature = "async")]
impl crate::v4::BlooioAccount<'_> {
    /// Access priorities.
    #[must_use]
    pub fn priorities(self) -> Priorities<Self> {
        Priorities { client: self }
    }
}
#[cfg(feature = "sync")]
impl crate::v4::BlockingBlooioAccount<'_> {
    /// Access priorities.
    #[must_use]
    pub fn priorities(self) -> Priorities<Self> {
        Priorities { client: self }
    }
}
#[cfg(feature = "async")]
impl<'a> Priorities<crate::v4::BlooioAccount<'a>> {
    /// List priorities.
    pub async fn list(&self) -> Result<ListEnvelope<Priority>> {
        self.client.send(ListPriorities::default()).await
    }
    /// Cursor over priorities.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListPriorities + use<'a>,
        ListPriorities,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListPriorities {
                cursor,
                limit: Some(limit),
            },
        )
    }
}
#[cfg(feature = "sync")]
impl<'a> Priorities<crate::v4::BlockingBlooioAccount<'a>> {
    /// List priorities.
    pub fn list(&self) -> Result<ListEnvelope<Priority>> {
        self.client.send(ListPriorities::default())
    }
    /// Cursor over priorities.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlockingBlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListPriorities + use<'a>,
        ListPriorities,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListPriorities {
                cursor,
                limit: Some(limit),
            },
        )
    }
}
