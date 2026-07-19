//! V4 message operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::v4::{
    CursorPaginator,
    types::{
        ItemEnvelope, ListEnvelope, Message, MessageContent, MessageEvent, MessageSendResult,
        MessageStatus, ReactionResult, Recipient, SenderSelector,
    },
};
use crate::{
    Result,
    core::operation::{encode_path_segment, json_body, push_opt},
};
use http::Method;
use serde::Serialize;

/// List messages in a chat.
#[derive(Debug, Clone, Default)]
pub struct ListChatMessages {
    pub chat_id: String,
    pub order: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}
impl crate::Operation for ListChatMessages {
    type Output = ListEnvelope<Message>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/chats/{}/messages", encode_path_segment(&self.chat_id))
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "order", self.order.as_ref());
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "cursor", self.cursor.as_ref());
        q
    }
}
impl_v4_operation!(ListChatMessages);

/// Send content to an existing chat.
#[derive(Debug, Clone, Serialize)]
pub struct SendMessageToChat {
    #[serde(skip)]
    pub chat_id: String,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
}
impl SendMessageToChat {
    /// Construct a chat send.
    #[must_use]
    pub fn new(chat_id: impl Into<String>, content: MessageContent) -> Self {
        Self {
            chat_id: chat_id.into(),
            content,
            dry_run: None,
        }
    }
}
impl crate::Operation for SendMessageToChat {
    type Output = MessageSendResult;
    const METHOD: Method = Method::POST;
    const RETRY_SAFE: bool = true;
    fn path(&self) -> String {
        format!("/chats/{}/messages", encode_path_segment(&self.chat_id))
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(SendMessageToChat);

/// Send a globally routed message.
#[derive(Debug, Clone, Serialize)]
pub struct SendMessage {
    #[serde(rename = "from", skip_serializing_if = "Option::is_none")]
    pub sender: Option<SenderSelector>,
    pub to: Recipient,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
}
impl SendMessage {
    /// Construct a globally routed send.
    #[must_use]
    pub fn new(to: Recipient, content: MessageContent) -> Self {
        Self {
            sender: None,
            to,
            content,
            dry_run: None,
        }
    }
}
impl crate::Operation for SendMessage {
    type Output = MessageSendResult;
    const METHOD: Method = Method::POST;
    const RETRY_SAFE: bool = true;
    fn path(&self) -> String {
        "/messages".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(SendMessage);

/// Get a message.
#[derive(Debug, Clone)]
pub struct GetChatMessage {
    pub chat_id: String,
    pub message_id: String,
}
impl crate::Operation for GetChatMessage {
    type Output = ItemEnvelope<Message>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/chats/{}/messages/{}",
            encode_path_segment(&self.chat_id),
            encode_path_segment(&self.message_id)
        )
    }
}
impl_v4_operation!(GetChatMessage);
/// Get message status.
#[derive(Debug, Clone)]
pub struct GetMessageStatus {
    pub chat_id: String,
    pub message_id: String,
}
impl crate::Operation for GetMessageStatus {
    type Output = ItemEnvelope<MessageStatus>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/chats/{}/messages/{}/status",
            encode_path_segment(&self.chat_id),
            encode_path_segment(&self.message_id)
        )
    }
}
impl_v4_operation!(GetMessageStatus);

/// List message lifecycle events.
#[derive(Debug, Clone, Default)]
pub struct ListMessageEvents {
    pub chat_id: String,
    pub message_id: String,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}
impl crate::Operation for ListMessageEvents {
    type Output = ListEnvelope<MessageEvent>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/chats/{}/messages/{}/events",
            encode_path_segment(&self.chat_id),
            encode_path_segment(&self.message_id)
        )
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "cursor", self.cursor.as_ref());
        q
    }
}
impl_v4_operation!(ListMessageEvents);

/// Add a reaction.
#[derive(Debug, Clone, Serialize)]
pub struct AddReaction {
    #[serde(skip)]
    pub chat_id: String,
    #[serde(skip)]
    pub message_id: String,
    pub reaction: String,
}
impl crate::Operation for AddReaction {
    type Output = ItemEnvelope<ReactionResult>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!(
            "/chats/{}/messages/{}/reactions",
            encode_path_segment(&self.chat_id),
            encode_path_segment(&self.message_id)
        )
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(AddReaction);

/// Global message resource handle.
#[derive(Debug)]
pub struct Messages<C> {
    pub(crate) client: C,
}
#[cfg(feature = "async")]
impl crate::v4::BlooioAccount<'_> {
    /// Access globally routed messages.
    #[must_use]
    pub fn messages(self) -> Messages<Self> {
        Messages { client: self }
    }
}
#[cfg(feature = "sync")]
impl crate::v4::BlockingBlooioAccount<'_> {
    /// Access globally routed messages.
    #[must_use]
    pub fn messages(self) -> Messages<Self> {
        Messages { client: self }
    }
}
#[cfg(feature = "async")]
impl Messages<crate::v4::BlooioAccount<'_>> {
    /// Send a globally routed message.
    pub async fn send(&self, operation: SendMessage) -> Result<MessageSendResult> {
        self.client.send(operation).await
    }
}
#[cfg(feature = "sync")]
impl Messages<crate::v4::BlockingBlooioAccount<'_>> {
    /// Send a globally routed message.
    pub fn send(&self, operation: SendMessage) -> Result<MessageSendResult> {
        self.client.send(operation)
    }
}

#[cfg(feature = "async")]
impl<'a> crate::v4::resources::chats::ChatHandle<crate::v4::BlooioAccount<'a>> {
    /// List the first message page.
    pub async fn list_messages(&self) -> Result<ListEnvelope<Message>> {
        self.client
            .send(ListChatMessages {
                chat_id: self.chat_id.clone(),
                ..Default::default()
            })
            .await
    }
    /// Cursor over this chat's messages.
    pub fn list_messages_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListChatMessages + use<'a>,
        ListChatMessages,
    > {
        let id = self.chat_id.clone();
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            move |cursor, limit| ListChatMessages {
                chat_id: id.clone(),
                cursor,
                limit: Some(limit),
                order: None,
            },
        )
    }
    /// Send content to this chat.
    pub async fn send(&self, content: MessageContent) -> Result<MessageSendResult> {
        self.client
            .send(SendMessageToChat::new(self.chat_id.clone(), content))
            .await
    }
}
#[cfg(feature = "sync")]
impl<'a> crate::v4::resources::chats::ChatHandle<crate::v4::BlockingBlooioAccount<'a>> {
    /// List the first message page.
    pub fn list_messages(&self) -> Result<ListEnvelope<Message>> {
        self.client.send(ListChatMessages {
            chat_id: self.chat_id.clone(),
            ..Default::default()
        })
    }
    /// Cursor over this chat's messages.
    pub fn list_messages_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlockingBlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListChatMessages + use<'a>,
        ListChatMessages,
    > {
        let id = self.chat_id.clone();
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            move |cursor, limit| ListChatMessages {
                chat_id: id.clone(),
                cursor,
                limit: Some(limit),
                order: None,
            },
        )
    }
    /// Send content to this chat.
    pub fn send(&self, content: MessageContent) -> Result<MessageSendResult> {
        self.client
            .send(SendMessageToChat::new(self.chat_id.clone(), content))
    }
}
