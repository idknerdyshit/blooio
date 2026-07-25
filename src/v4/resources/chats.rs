//! V4 chat and poll operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::v4::{
    CursorPaginator,
    types::{
        ActionResponse, Chat, ChatCreated, ContactCardShareResult, ItemEnvelope, ListEnvelope,
        MarkReadResult, Poll, PollResults, PollVoteResult, TypingResult,
    },
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

/// List chats.
#[derive(Debug, Clone, Default)]
pub struct ListChats {
    pub chat_type: Option<String>,
    pub state: Option<String>,
    pub channel_id: Option<String>,
    pub contact_id: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}
impl crate::Operation for ListChats {
    type Output = ListEnvelope<Chat>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/chats".into()
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "type", self.chat_type.as_ref());
        push_opt(&mut q, "state", self.state.as_ref());
        push_opt(&mut q, "channel_id", self.channel_id.as_ref());
        push_opt(&mut q, "contact_id", self.contact_id.as_ref());
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "cursor", self.cursor.as_ref());
        q
    }
}
impl_v4_operation!(ListChats);

/// Find or create a chat.
#[derive(Debug, Clone, Serialize)]
pub struct CreateChat {
    pub channel_id: String,
    pub to: String,
}
impl CreateChat {
    /// Construct a chat request.
    #[must_use]
    pub fn new(channel_id: impl Into<String>, identifier: impl Into<String>) -> Self {
        Self {
            channel_id: channel_id.into(),
            to: identifier.into(),
        }
    }
}
impl crate::Operation for CreateChat {
    type Output = ChatCreated;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/chats".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(CreateChat);

/// Get a chat.
#[derive(Debug, Clone)]
pub struct GetChat {
    pub chat_id: String,
}
impl crate::Operation for GetChat {
    type Output = ItemEnvelope<Chat>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/chats/{}", encode_path_segment(&self.chat_id))
    }
}
impl_v4_operation!(GetChat);
/// Update a chat.
#[derive(Debug, Clone)]
pub struct UpdateChat {
    pub chat_id: String,
    pub fields: Value,
}
impl crate::Operation for UpdateChat {
    type Output = ItemEnvelope<Chat>;
    const METHOD: Method = Method::PATCH;
    fn path(&self) -> String {
        format!("/chats/{}", encode_path_segment(&self.chat_id))
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(&self.fields)
    }
}
impl_v4_operation!(UpdateChat);

/// Set the typing state.
#[derive(Debug, Clone, Serialize)]
pub struct SetTyping {
    #[serde(skip)]
    pub chat_id: String,
    pub state: String,
}
impl crate::Operation for SetTyping {
    type Output = ItemEnvelope<TypingResult>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/chats/{}/typing", encode_path_segment(&self.chat_id))
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(SetTyping);
/// Stop typing.
#[derive(Debug, Clone)]
pub struct StopTyping {
    pub chat_id: String,
}
impl crate::Operation for StopTyping {
    type Output = ItemEnvelope<TypingResult>;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/chats/{}/typing", encode_path_segment(&self.chat_id))
    }
}
impl_v4_operation!(StopTyping);
/// Mark a chat read.
#[derive(Debug, Clone)]
pub struct MarkChatRead {
    pub chat_id: String,
}
impl crate::Operation for MarkChatRead {
    type Output = ItemEnvelope<MarkReadResult>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/chats/{}/read", encode_path_segment(&self.chat_id))
    }
}
impl_v4_operation!(MarkChatRead);
/// Share the sender contact card.
#[derive(Debug, Clone)]
pub struct ShareContactCard {
    pub chat_id: String,
}
impl crate::Operation for ShareContactCard {
    type Output = ItemEnvelope<ContactCardShareResult>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/chats/{}/contact-card", encode_path_segment(&self.chat_id))
    }
}
impl_v4_operation!(ShareContactCard);

/// Get chat background metadata.
#[derive(Debug, Clone)]
pub struct GetChatBackground {
    pub chat_id: String,
}
impl crate::Operation for GetChatBackground {
    type Output = ActionResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/chats/{}/background", encode_path_segment(&self.chat_id))
    }
}
impl_v4_operation!(GetChatBackground);
/// Set a chat background.
#[derive(Debug, Clone)]
pub struct SetChatBackground {
    pub chat_id: String,
    background: Vec<u8>,
    filename: String,
    content_type: String,
}
impl SetChatBackground {
    /// Construct a background upload.
    #[must_use]
    pub fn new(chat_id: impl Into<String>, background: impl Into<Vec<u8>>) -> Self {
        Self {
            chat_id: chat_id.into(),
            background: background.into(),
            filename: "background.jpg".into(),
            content_type: "image/jpeg".into(),
        }
    }
}
impl crate::Operation for SetChatBackground {
    type Output = ActionResponse;
    const METHOD: Method = Method::PUT;
    fn path(&self) -> String {
        format!("/chats/{}/background", encode_path_segment(&self.chat_id))
    }
    fn headers(&self) -> Vec<(&'static str, String)> {
        let b = multipart::boundary_for(&self.background);
        vec![("content-type", format!("multipart/form-data; boundary={b}"))]
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        let b = multipart::boundary_for(&self.background);
        let ct = multipart::part_content_type(Some(&self.content_type))?;
        Ok(Some(multipart::file_body(
            &b,
            "background",
            &self.background,
            Some(&self.filename),
            ct,
        )))
    }
}
impl_v4_operation!(SetChatBackground);
/// Remove a chat background.
#[derive(Debug, Clone)]
pub struct RemoveChatBackground {
    pub chat_id: String,
}
impl crate::Operation for RemoveChatBackground {
    type Output = ActionResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/chats/{}/background", encode_path_segment(&self.chat_id))
    }
}
impl_v4_operation!(RemoveChatBackground);

/// Send a poll.
#[derive(Debug, Clone, Serialize)]
pub struct SendPoll {
    #[serde(skip)]
    pub chat_id: String,
    pub title: String,
    pub options: Vec<String>,
}
impl SendPoll {
    /// Construct a poll.
    #[must_use]
    pub fn new(
        chat_id: impl Into<String>,
        title: impl Into<String>,
        options: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            chat_id: chat_id.into(),
            title: title.into(),
            options: options.into_iter().map(Into::into).collect(),
        }
    }
}
impl crate::Operation for SendPoll {
    type Output = ItemEnvelope<Poll>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/chats/{}/polls", encode_path_segment(&self.chat_id))
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(SendPoll);
/// Vote in a poll.
#[derive(Debug, Clone, Serialize)]
pub struct VotePoll {
    #[serde(skip)]
    pub chat_id: String,
    #[serde(skip)]
    pub poll_id: String,
    #[serde(flatten)]
    selection: VotePollSelection,
}
/// Selector accepted by the poll-vote endpoint.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
enum VotePollSelection {
    Index { option_index: u32 },
    Option { option: String },
}
impl VotePoll {
    /// Vote by zero-based option index.
    #[must_use]
    pub fn by_index(
        chat_id: impl Into<String>,
        poll_id: impl Into<String>,
        option_index: u32,
    ) -> Self {
        Self {
            chat_id: chat_id.into(),
            poll_id: poll_id.into(),
            selection: VotePollSelection::Index { option_index },
        }
    }

    /// Vote by option text.
    #[must_use]
    pub fn by_option(
        chat_id: impl Into<String>,
        poll_id: impl Into<String>,
        option: impl Into<String>,
    ) -> Self {
        Self {
            chat_id: chat_id.into(),
            poll_id: poll_id.into(),
            selection: VotePollSelection::Option {
                option: option.into(),
            },
        }
    }
}
impl crate::Operation for VotePoll {
    type Output = ItemEnvelope<PollVoteResult>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!(
            "/chats/{}/polls/{}/vote",
            encode_path_segment(&self.chat_id),
            encode_path_segment(&self.poll_id)
        )
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(VotePoll);
/// Get poll results.
#[derive(Debug, Clone)]
pub struct GetPollResults {
    pub chat_id: String,
    pub poll_id: String,
}
impl crate::Operation for GetPollResults {
    type Output = ItemEnvelope<PollResults>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/chats/{}/polls/{}",
            encode_path_segment(&self.chat_id),
            encode_path_segment(&self.poll_id)
        )
    }
}
impl_v4_operation!(GetPollResults);

/// Chat collection handle.
#[derive(Debug)]
pub struct Chats<C> {
    pub(crate) client: C,
}
/// Handle scoped to one chat.
#[derive(Debug)]
pub struct ChatHandle<C> {
    pub(crate) client: C,
    pub(crate) chat_id: String,
}
#[cfg(feature = "async")]
impl crate::v4::BlooioAccount<'_> {
    /// Access chats.
    #[must_use]
    pub fn chats(self) -> Chats<Self> {
        Chats { client: self }
    }
    /// Scope operations to a chat.
    #[must_use]
    pub fn chat(self, chat_id: impl Into<String>) -> ChatHandle<Self> {
        ChatHandle {
            client: self,
            chat_id: chat_id.into(),
        }
    }
}
#[cfg(feature = "sync")]
impl crate::v4::BlockingBlooioAccount<'_> {
    /// Access chats.
    #[must_use]
    pub fn chats(self) -> Chats<Self> {
        Chats { client: self }
    }
    /// Scope operations to a chat.
    #[must_use]
    pub fn chat(self, chat_id: impl Into<String>) -> ChatHandle<Self> {
        ChatHandle {
            client: self,
            chat_id: chat_id.into(),
        }
    }
}
#[cfg(feature = "async")]
impl<'a> Chats<crate::v4::BlooioAccount<'a>> {
    /// List chats.
    pub async fn list(&self) -> Result<ListEnvelope<Chat>> {
        self.client.send(ListChats::default()).await
    }
    /// Cursor over chats.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListChats + use<'a>,
        ListChats,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListChats {
                cursor,
                limit: Some(limit),
                ..Default::default()
            },
        )
    }
    /// Find or create a chat.
    pub async fn create(&self, operation: CreateChat) -> Result<ChatCreated> {
        self.client.send(operation).await
    }
}
#[cfg(feature = "sync")]
impl<'a> Chats<crate::v4::BlockingBlooioAccount<'a>> {
    /// List chats.
    pub fn list(&self) -> Result<ListEnvelope<Chat>> {
        self.client.send(ListChats::default())
    }
    /// Cursor over chats.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlockingBlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListChats + use<'a>,
        ListChats,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListChats {
                cursor,
                limit: Some(limit),
                ..Default::default()
            },
        )
    }
    /// Find or create a chat.
    pub fn create(&self, operation: CreateChat) -> Result<ChatCreated> {
        self.client.send(operation)
    }
}
#[cfg(feature = "async")]
impl ChatHandle<crate::v4::BlooioAccount<'_>> {
    /// Get this chat.
    pub async fn get(&self) -> Result<ItemEnvelope<Chat>> {
        self.client
            .send(GetChat {
                chat_id: self.chat_id.clone(),
            })
            .await
    }
}
#[cfg(feature = "sync")]
impl ChatHandle<crate::v4::BlockingBlooioAccount<'_>> {
    /// Get this chat.
    pub fn get(&self) -> Result<ItemEnvelope<Chat>> {
        self.client.send(GetChat {
            chat_id: self.chat_id.clone(),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::SendPoll;

    #[test]
    fn send_poll_serializes_the_endpoint_payload() {
        let body = serde_json::to_value(SendPoll::new("chat_1", "Lunch?", ["Yes", "No"])).unwrap();
        assert_eq!(
            body,
            serde_json::json!({"title": "Lunch?", "options": ["Yes", "No"]})
        );
    }
}
