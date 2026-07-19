//! V4 unified event-feed operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::v4::{
    CursorPaginator,
    types::{Event, ItemEnvelope, ListEnvelope},
};
use crate::{
    Result,
    core::operation::{encode_path_segment, push_opt},
};
use http::Method;

/// List unified events.
#[derive(Debug, Clone, Default)]
pub struct ListEvents {
    pub event_type: Option<String>,
    pub chat_id: Option<String>,
    pub message_id: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}
impl crate::Operation for ListEvents {
    type Output = ListEnvelope<Event>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/events".into()
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "type", self.event_type.as_ref());
        push_opt(&mut q, "chat_id", self.chat_id.as_ref());
        push_opt(&mut q, "message_id", self.message_id.as_ref());
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "cursor", self.cursor.as_ref());
        q
    }
}
impl_v4_operation!(ListEvents);
/// Get an event.
#[derive(Debug, Clone)]
pub struct GetEvent {
    pub event_id: String,
}
impl crate::Operation for GetEvent {
    type Output = ItemEnvelope<Event>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/events/{}", encode_path_segment(&self.event_id))
    }
}
impl_v4_operation!(GetEvent);

/// Unified events handle.
#[derive(Debug)]
pub struct Events<C> {
    pub(crate) client: C,
}
#[cfg(feature = "async")]
impl crate::v4::BlooioAccount<'_> {
    /// Access unified events.
    #[must_use]
    pub fn events(self) -> Events<Self> {
        Events { client: self }
    }
}
#[cfg(feature = "sync")]
impl crate::v4::BlockingBlooioAccount<'_> {
    /// Access unified events.
    #[must_use]
    pub fn events(self) -> Events<Self> {
        Events { client: self }
    }
}
#[cfg(feature = "async")]
impl<'a> Events<crate::v4::BlooioAccount<'a>> {
    /// List events.
    pub async fn list(&self) -> Result<ListEnvelope<Event>> {
        self.client.send(ListEvents::default()).await
    }
    /// Cursor over events.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListEvents + use<'a>,
        ListEvents,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListEvents {
                cursor,
                limit: Some(limit),
                ..Default::default()
            },
        )
    }
    /// Get one event.
    pub async fn get(&self, id: impl Into<String>) -> Result<ItemEnvelope<Event>> {
        self.client
            .send(GetEvent {
                event_id: id.into(),
            })
            .await
    }
}
#[cfg(feature = "sync")]
impl<'a> Events<crate::v4::BlockingBlooioAccount<'a>> {
    /// List events.
    pub fn list(&self) -> Result<ListEnvelope<Event>> {
        self.client.send(ListEvents::default())
    }
    /// Cursor over events.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlockingBlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListEvents + use<'a>,
        ListEvents,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListEvents {
                cursor,
                limit: Some(limit),
                ..Default::default()
            },
        )
    }
    /// Get one event.
    pub fn get(&self, id: impl Into<String>) -> Result<ItemEnvelope<Event>> {
        self.client.send(GetEvent {
            event_id: id.into(),
        })
    }
}
