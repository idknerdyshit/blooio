//! Chats and everything scoped to a chat: messages (incl. the marquee
//! `sendMessage`), reactions, typing indicators, read receipts, polls, chat
//! background, and contact-card sharing.

use http::{Method, header::HeaderValue};
use serde::{Deserialize, Serialize};

use crate::core::operation::{Operation, encode_path_segment, json_body, push_opt};
use crate::core::pagination::{DEFAULT_PAGE_SIZE, Listing, Page, Pagination, Paginator};
use crate::error::{Error, Result};
use crate::types::{
    Chat, ChatBackgroundResponse, IntoStringList, Json, LinkPreview, Message, MessageDetail,
    MessageStatus, ReactionResponse, ReadResponse, SendMessageResponse, TypingResponse,
};

// ===========================================================================
// Send-message value types
// ===========================================================================

/// Message text: a single string, or an array where each element becomes its
/// own message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Text {
    /// A single message.
    One(String),
    /// One message per element.
    Many(Vec<String>),
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Text::One(s.to_owned())
    }
}
impl From<String> for Text {
    fn from(s: String) -> Self {
        Text::One(s)
    }
}
impl From<Vec<String>> for Text {
    fn from(v: Vec<String>) -> Self {
        Text::Many(v)
    }
}

impl Text {
    /// Build multi-message text from a string collection.
    pub fn many(values: impl IntoStringList) -> Self {
        Text::Many(values.into_string_vec())
    }
}

/// An attachment: either a bare URL or an object with a URL and optional name.
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Attachment {
    /// A plain attachment URL.
    Url(String),
    /// A URL with an explicit filename.
    Named {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
}

impl From<&str> for Attachment {
    fn from(s: &str) -> Self {
        Attachment::Url(s.to_owned())
    }
}
impl From<String> for Attachment {
    fn from(s: String) -> Self {
        Attachment::Url(s)
    }
}

/// One ordered part of a multipart or URL-balloon-batch message.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Serialize)]
pub struct MessagePart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mention: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_preview: Option<LinkPreview>,
}

/// Inline reply target for `POST /chats/{chatId}/messages`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Serialize)]
pub struct ReplyToRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_index: Option<u32>,
}

impl ReplyToRequest {
    /// Target a Blooio-minted parent message id.
    pub fn message_id(message_id: impl Into<String>) -> Self {
        Self {
            message_id: Some(message_id.into()),
            guid: None,
            part_index: None,
        }
    }

    /// Target a raw iMessage parent GUID.
    pub fn guid(guid: impl Into<String>) -> Self {
        Self {
            message_id: None,
            guid: Some(guid.into()),
            part_index: None,
        }
    }

    /// Set the parent message part index. Defaults server-side to `0`.
    #[must_use]
    pub fn part_index(mut self, part_index: u32) -> Self {
        self.part_index = Some(part_index);
        self
    }
}

// ===========================================================================
// Resource-specific response types
// ===========================================================================

/// Response of `GET /chats`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ListChatsResponse {
    pub chats: Vec<Chat>,
    pub pagination: Option<Pagination>,
}

impl Listing for ListChatsResponse {
    type Item = Chat;
    fn into_page(self) -> Page<Self::Item> {
        Page {
            items: self.chats,
            pagination: self.pagination,
        }
    }
}

/// Response of `GET /chats/{chatId}/messages`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ListChatMessagesResponse {
    pub chat_id: Option<String>,
    pub messages: Vec<Message>,
    pub pagination: Option<Pagination>,
}

impl Listing for ListChatMessagesResponse {
    type Item = Message;
    fn into_page(self) -> Page<Self::Item> {
        Page {
            items: self.messages,
            pagination: self.pagination,
        }
    }
}

/// Response of `POST /chats/{chatId}/polls`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct SendPollResponse {
    pub poll_id: Option<String>,
    pub chat_id: Option<String>,
    pub poll: Option<Json>,
    pub sent_at: Option<f64>,
}

/// A single poll option with its tally.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct PollOptionResult {
    pub text: Option<String>,
    pub votes: Option<i64>,
}

/// Response of `GET /chats/{chatId}/polls/{pollId}`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct PollResults {
    pub poll_id: Option<String>,
    pub chat_id: Option<String>,
    pub title: Option<String>,
    pub options: Option<Vec<PollOptionResult>>,
    pub total_votes: Option<i64>,
}

/// Response of `POST /chats/{chatId}/contact-card`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ShareContactCardResponse {
    pub success: Option<bool>,
    pub chat_id: Option<String>,
    pub message: Option<String>,
}

fn multipart_background_body(
    boundary: &str,
    background: &[u8],
    filename: Option<&str>,
    content_type: &str,
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"\r\nContent-Disposition: form-data; name=\"background\"");
    if let Some(filename) = filename {
        body.extend_from_slice(b"; filename=\"");
        body.extend_from_slice(escape_multipart_header_value(filename).as_bytes());
        body.extend_from_slice(b"\"");
    }
    body.extend_from_slice(b"\r\nContent-Type: ");
    body.extend_from_slice(content_type.as_bytes());
    body.extend_from_slice(b"\r\n\r\n");
    body.extend_from_slice(background);
    body.extend_from_slice(b"\r\n--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"--\r\n");
    body
}

fn multipart_part_content_type(content_type: Option<&str>) -> Result<&str> {
    let content_type = content_type.unwrap_or("application/octet-stream");
    HeaderValue::from_str(content_type).map_err(Error::config)?;
    Ok(content_type)
}

fn escape_multipart_header_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\r' | '\n' => escaped.push('_'),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

// ===========================================================================
// Operations
// ===========================================================================

/// `GET /chats`
#[allow(missing_docs)]
#[derive(Debug, Clone, Default)]
pub struct ListChats {
    /// Maximum number of chats to return.
    pub limit: Option<u32>,
    /// Number of chats to skip before returning results.
    pub offset: Option<u32>,
    /// Search text used by the API to filter chats.
    pub q: Option<String>,
    /// API sort expression, passed through to Blooio unchanged.
    pub sort: Option<String>,
}

impl Operation for ListChats {
    type Output = ListChatsResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/chats".into()
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

/// `GET /chats/{chatId}`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct GetChat {
    /// Blooio chat id.
    pub chat_id: String,
}

impl Operation for GetChat {
    type Output = Chat;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/chats/{}", encode_path_segment(&self.chat_id))
    }
}

/// `GET /chats/{chatId}/messages`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct ListChatMessages {
    /// Blooio chat id.
    pub chat_id: String,
    /// Maximum number of messages to return.
    pub limit: Option<u32>,
    /// Number of messages to skip before returning results.
    pub offset: Option<u32>,
    /// API sort expression, passed through to Blooio unchanged.
    pub sort: Option<String>,
    /// Message sort direction, passed through to Blooio unchanged.
    pub direction: Option<String>,
    /// Lower timestamp bound for messages, expressed as the API expects.
    pub since: Option<i64>,
    /// Upper timestamp bound for messages, expressed as the API expects.
    pub until: Option<i64>,
}

impl ListChatMessages {
    /// Create a new list-messages request for the given chat id.
    pub fn new(chat_id: impl Into<String>) -> Self {
        ListChatMessages {
            chat_id: chat_id.into(),
            limit: None,
            offset: None,
            sort: None,
            direction: None,
            since: None,
            until: None,
        }
    }
}

impl Operation for ListChatMessages {
    type Output = ListChatMessagesResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/chats/{}/messages", encode_path_segment(&self.chat_id))
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "offset", self.offset);
        push_opt(&mut q, "sort", self.sort.as_ref());
        push_opt(&mut q, "direction", self.direction.as_ref());
        push_opt(&mut q, "since", self.since);
        push_opt(&mut q, "until", self.until);
        q
    }
}

/// `POST /chats/{chatId}/messages` — send a message.
///
/// Build with [`SendMessage::new`] and the chained setters. The builder seeds a
/// fresh `UUIDv4` idempotency key so accidental duplicate sends are
/// de-duplicated server-side.
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct SendMessage {
    #[serde(skip)]
    /// Blooio chat id.
    pub chat_id: String,
    #[serde(skip)]
    /// Idempotency key sent as the `Idempotency-Key` header.
    pub idempotency_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// Message text, either a single message or multiple message bubbles.
    pub text: Option<Text>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Attachments to include with the message.
    pub attachments: Option<Vec<Attachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Whether Blooio should show a typing indicator before sending.
    pub use_typing_indicator: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Originating phone number to send from.
    pub from_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Whether to include the account contact card with the message.
    pub share_contact: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Ordered rich message parts.
    pub parts: Option<Vec<MessagePart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Rich link-preview override.
    pub link_preview: Option<LinkPreview>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// iMessage effect name, passed through to Blooio unchanged.
    pub effect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Parent message target for an inline reply.
    pub reply_to: Option<ReplyToRequest>,
}

impl SendMessage {
    /// Start building a message for `chat_id`.
    pub fn new(chat_id: impl Into<String>) -> Self {
        SendMessage {
            chat_id: chat_id.into(),
            idempotency_key: Some(uuid::Uuid::new_v4().to_string()),
            text: None,
            attachments: None,
            use_typing_indicator: None,
            from_number: None,
            share_contact: None,
            parts: None,
            link_preview: None,
            effect: None,
            reply_to: None,
        }
    }

    /// Set the message text (single string or array).
    #[must_use]
    pub fn text(mut self, text: impl Into<Text>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Set the attachments.
    #[must_use]
    pub fn attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments = Some(attachments);
        self
    }

    /// Set the ordered message parts.
    #[must_use]
    pub fn parts(mut self, parts: Vec<MessagePart>) -> Self {
        self.parts = Some(parts);
        self
    }

    /// Send with an iMessage effect (e.g. `"confetti"`), passed through to
    /// Blooio unchanged.
    #[must_use]
    pub fn effect(mut self, effect: impl Into<String>) -> Self {
        self.effect = Some(effect.into());
        self
    }

    /// Send this message as an inline reply to a previous iMessage.
    #[must_use]
    pub fn reply_to(mut self, reply_to: ReplyToRequest) -> Self {
        self.reply_to = Some(reply_to);
        self
    }

    /// Send this message as an inline reply to a Blooio-minted parent message id.
    #[must_use]
    pub fn reply_to_message_id(mut self, message_id: impl Into<String>) -> Self {
        self.reply_to = Some(ReplyToRequest::message_id(message_id));
        self
    }

    /// Send this message as an inline reply to a raw iMessage parent GUID.
    #[must_use]
    pub fn reply_to_guid(mut self, guid: impl Into<String>) -> Self {
        self.reply_to = Some(ReplyToRequest::guid(guid));
        self
    }

    /// Choose the originating phone number.
    #[must_use]
    pub fn from_number(mut self, number: impl Into<String>) -> Self {
        self.from_number = Some(number.into());
        self
    }

    /// Override the rich link preview.
    #[must_use]
    pub fn link_preview(mut self, preview: LinkPreview) -> Self {
        self.link_preview = Some(preview);
        self
    }

    /// Show a typing indicator before sending.
    #[must_use]
    pub fn use_typing_indicator(mut self, yes: bool) -> Self {
        self.use_typing_indicator = Some(yes);
        self
    }

    /// Piggyback the contact card onto the message.
    #[must_use]
    pub fn share_contact(mut self, yes: bool) -> Self {
        self.share_contact = Some(yes);
        self
    }

    /// Supply an explicit idempotency key (otherwise one is auto-generated).
    #[must_use]
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }
}

impl Operation for SendMessage {
    type Output = SendMessageResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/chats/{}/messages", encode_path_segment(&self.chat_id))
    }
    fn headers(&self) -> Vec<(&'static str, String)> {
        self.idempotency_key
            .clone()
            .map(|key| vec![("Idempotency-Key", key)])
            .unwrap_or_default()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

/// `GET /chats/{chatId}/messages/{messageId}`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct GetMessage {
    /// Blooio chat id.
    pub chat_id: String,
    /// Blooio message id.
    pub message_id: String,
}

impl Operation for GetMessage {
    type Output = MessageDetail;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/chats/{}/messages/{}",
            encode_path_segment(&self.chat_id),
            encode_path_segment(&self.message_id)
        )
    }
}

/// `GET /chats/{chatId}/messages/{messageId}/status`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct GetMessageStatus {
    /// Blooio chat id.
    pub chat_id: String,
    /// Blooio message id.
    pub message_id: String,
}

impl Operation for GetMessageStatus {
    type Output = MessageStatus;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/chats/{}/messages/{}/status",
            encode_path_segment(&self.chat_id),
            encode_path_segment(&self.message_id)
        )
    }
}

/// `POST /chats/{chatId}/messages/{messageId}/reactions`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct AddReaction {
    #[serde(skip)]
    /// Blooio chat id.
    pub chat_id: String,
    #[serde(skip)]
    /// Blooio message id to react to.
    pub message_id: String,
    /// Reaction value accepted by the API, passed through unchanged.
    pub reaction: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Reaction direction, passed through unchanged when the API distinguishes
    /// add/remove style actions.
    pub direction: Option<String>,
}

impl Operation for AddReaction {
    type Output = ReactionResponse;
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

/// `POST /chats/{chatId}/polls`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct SendPoll {
    #[serde(skip)]
    /// Blooio chat id.
    pub chat_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional poll title.
    pub title: Option<String>,
    /// Poll options to present.
    pub options: Vec<String>,
}

impl Operation for SendPoll {
    type Output = SendPollResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/chats/{}/polls", encode_path_segment(&self.chat_id))
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

impl SendPoll {
    /// Create a poll request from a string collection of option values.
    pub fn new(
        chat_id: impl Into<String>,
        title: Option<String>,
        options: impl IntoStringList,
    ) -> Self {
        SendPoll {
            chat_id: chat_id.into(),
            title,
            options: options.into_string_vec(),
        }
    }
}

/// `GET /chats/{chatId}/polls/{pollId}`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct GetPollResults {
    /// Blooio chat id.
    pub chat_id: String,
    /// Blooio poll id.
    pub poll_id: String,
}

impl Operation for GetPollResults {
    type Output = PollResults;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/chats/{}/polls/{}",
            encode_path_segment(&self.chat_id),
            encode_path_segment(&self.poll_id)
        )
    }
}

/// `POST /chats/{chatId}/typing`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct StartTyping {
    /// Blooio chat id.
    pub chat_id: String,
}

impl Operation for StartTyping {
    type Output = TypingResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/chats/{}/typing", encode_path_segment(&self.chat_id))
    }
}

/// `DELETE /chats/{chatId}/typing`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct StopTyping {
    /// Blooio chat id.
    pub chat_id: String,
}

impl Operation for StopTyping {
    type Output = TypingResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/chats/{}/typing", encode_path_segment(&self.chat_id))
    }
}

/// `POST /chats/{chatId}/read`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct MarkChatRead {
    /// Blooio chat id.
    pub chat_id: String,
}

impl Operation for MarkChatRead {
    type Output = ReadResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/chats/{}/read", encode_path_segment(&self.chat_id))
    }
}

/// `POST /chats/{chatId}/contact-card`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct ShareContactCard {
    /// Blooio chat id.
    pub chat_id: String,
}

impl Operation for ShareContactCard {
    type Output = ShareContactCardResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/chats/{}/contact-card", encode_path_segment(&self.chat_id))
    }
}

/// `GET /chats/{chatId}/background`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct GetChatBackground {
    /// Blooio chat id.
    pub chat_id: String,
}

impl Operation for GetChatBackground {
    type Output = ChatBackgroundResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/chats/{}/background", encode_path_segment(&self.chat_id))
    }
}

/// `PUT /chats/{chatId}/background`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct SetChatBackground {
    /// Blooio chat id.
    pub chat_id: String,
    /// Raw image bytes uploaded as a multipart `background` part.
    pub background: Vec<u8>,
    /// Optional multipart filename for the uploaded image.
    pub filename: Option<String>,
    /// Optional MIME type for the uploaded image; defaults to
    /// `application/octet-stream`.
    pub content_type: Option<String>,
}

impl SetChatBackground {
    /// Create a chat background upload operation from raw image bytes.
    pub fn new(chat_id: impl Into<String>, background: impl Into<Vec<u8>>) -> Self {
        Self {
            chat_id: chat_id.into(),
            background: background.into(),
            filename: None,
            content_type: None,
        }
    }

    /// Set the multipart filename for the uploaded image.
    #[must_use]
    pub fn filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// Set the MIME type for the uploaded image part.
    #[must_use]
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    fn boundary(&self) -> String {
        for index in 0u64.. {
            let boundary = format!("blooio-form-boundary-{index}");
            if !contains_bytes(&self.background, boundary.as_bytes()) {
                return boundary;
            }
        }
        unreachable!("unbounded boundary search must find a value absent from a finite payload")
    }
}

impl Operation for SetChatBackground {
    type Output = ChatBackgroundResponse;
    const METHOD: Method = Method::PUT;
    fn path(&self) -> String {
        format!("/chats/{}/background", encode_path_segment(&self.chat_id))
    }
    fn headers(&self) -> Vec<(&'static str, String)> {
        let boundary = self.boundary();
        vec![(
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        )]
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        let boundary = self.boundary();
        let content_type = multipart_part_content_type(self.content_type.as_deref())?;
        Ok(Some(multipart_background_body(
            &boundary,
            &self.background,
            self.filename.as_deref(),
            content_type,
        )))
    }
}

/// `DELETE /chats/{chatId}/background`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct RemoveChatBackground {
    /// Blooio chat id.
    pub chat_id: String,
}

impl Operation for RemoveChatBackground {
    type Output = ChatBackgroundResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/chats/{}/background", encode_path_segment(&self.chat_id))
    }
}

// ===========================================================================
// Handles
// ===========================================================================

/// Handle for the top-level `chats` collection (listing).
#[derive(Debug)]
pub struct Chats<'c, C> {
    pub(crate) client: &'c C,
}

/// Handle scoped to a single chat.
#[derive(Debug)]
pub struct Chat_<'c, C> {
    pub(crate) client: &'c C,
    pub(crate) chat_id: String,
}

#[cfg(feature = "async")]
impl crate::Client {
    /// Access the chats collection (listing).
    pub fn chats(&self) -> Chats<'_, crate::Client> {
        Chats { client: self }
    }

    /// Operate on a single chat.
    pub fn chat(&self, chat_id: impl Into<String>) -> Chat_<'_, crate::Client> {
        Chat_ {
            client: self,
            chat_id: chat_id.into(),
        }
    }
}

#[cfg(feature = "sync")]
impl crate::BlockingClient {
    /// Access the chats collection (listing).
    pub fn chats(&self) -> Chats<'_, crate::BlockingClient> {
        Chats { client: self }
    }

    /// Operate on a single chat.
    pub fn chat(&self, chat_id: impl Into<String>) -> Chat_<'_, crate::BlockingClient> {
        Chat_ {
            client: self,
            chat_id: chat_id.into(),
        }
    }
}

#[cfg(feature = "async")]
impl<'c> Chats<'c, crate::Client> {
    /// List chats (first page).
    pub async fn list(&self) -> Result<ListChatsResponse> {
        self.client.send(ListChats::default()).await
    }
    /// List chats with explicit filters.
    pub async fn list_with(&self, query: ListChats) -> Result<ListChatsResponse> {
        self.client.send(query).await
    }
    /// Paginate over all chats.
    pub fn list_all(
        &self,
    ) -> Paginator<'c, crate::Client, impl Fn(u32, u32) -> ListChats + use<'c>, ListChats> {
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, |offset, limit| ListChats {
            offset: Some(offset),
            limit: Some(limit),
            ..Default::default()
        })
    }
}

#[cfg(feature = "sync")]
impl<'c> Chats<'c, crate::BlockingClient> {
    /// List chats (first page).
    pub fn list(&self) -> Result<ListChatsResponse> {
        self.client.send(ListChats::default())
    }
    /// List chats with explicit filters.
    pub fn list_with(&self, query: ListChats) -> Result<ListChatsResponse> {
        self.client.send(query)
    }
    /// Paginate over all chats.
    pub fn list_all(
        &self,
    ) -> Paginator<'c, crate::BlockingClient, impl Fn(u32, u32) -> ListChats + use<'c>, ListChats>
    {
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, |offset, limit| ListChats {
            offset: Some(offset),
            limit: Some(limit),
            ..Default::default()
        })
    }
}

#[cfg(feature = "async")]
impl<'c> Chat_<'c, crate::Client> {
    /// Fetch this chat's detail.
    pub async fn get(&self) -> Result<Chat> {
        self.client
            .send(GetChat {
                chat_id: self.chat_id.clone(),
            })
            .await
    }

    /// List this chat's messages (first page).
    pub async fn list_messages(&self) -> Result<ListChatMessagesResponse> {
        self.client
            .send(ListChatMessages::new(self.chat_id.clone()))
            .await
    }

    /// List messages with explicit filters.
    pub async fn list_messages_with(
        &self,
        query: ListChatMessages,
    ) -> Result<ListChatMessagesResponse> {
        self.client.send(query).await
    }

    /// Paginate over all of this chat's messages.
    pub fn list_messages_all(
        &self,
    ) -> Paginator<
        'c,
        crate::Client,
        impl Fn(u32, u32) -> ListChatMessages + use<'c>,
        ListChatMessages,
    > {
        let chat_id = self.chat_id.clone();
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, move |offset, limit| {
            ListChatMessages {
                offset: Some(offset),
                limit: Some(limit),
                ..ListChatMessages::new(chat_id.clone())
            }
        })
    }

    /// A [`SendMessage`] builder pre-seeded with this chat's id.
    pub fn message(&self) -> SendMessage {
        SendMessage::new(self.chat_id.clone())
    }

    /// Send a fully-built message.
    pub async fn send(&self, message: SendMessage) -> Result<SendMessageResponse> {
        self.client.send(message).await
    }

    /// Convenience: send plain text.
    pub async fn send_text(&self, text: impl Into<Text>) -> Result<SendMessageResponse> {
        self.client.send(self.message().text(text)).await
    }

    /// Fetch a single message.
    pub async fn get_message(&self, message_id: impl Into<String>) -> Result<MessageDetail> {
        self.client
            .send(GetMessage {
                chat_id: self.chat_id.clone(),
                message_id: message_id.into(),
            })
            .await
    }

    /// Fetch a message's delivery status.
    pub async fn message_status(&self, message_id: impl Into<String>) -> Result<MessageStatus> {
        self.client
            .send(GetMessageStatus {
                chat_id: self.chat_id.clone(),
                message_id: message_id.into(),
            })
            .await
    }

    /// React to a message.
    pub async fn add_reaction(
        &self,
        message_id: impl Into<String>,
        reaction: impl Into<String>,
        direction: Option<String>,
    ) -> Result<ReactionResponse> {
        self.client
            .send(AddReaction {
                chat_id: self.chat_id.clone(),
                message_id: message_id.into(),
                reaction: reaction.into(),
                direction,
            })
            .await
    }

    /// Send a poll.
    pub async fn send_poll(
        &self,
        title: Option<String>,
        options: impl IntoStringList,
    ) -> Result<SendPollResponse> {
        self.client
            .send(SendPoll::new(self.chat_id.clone(), title, options))
            .await
    }

    /// Fetch poll results.
    pub async fn poll_results(&self, poll_id: impl Into<String>) -> Result<PollResults> {
        self.client
            .send(GetPollResults {
                chat_id: self.chat_id.clone(),
                poll_id: poll_id.into(),
            })
            .await
    }

    /// Start the typing indicator.
    pub async fn start_typing(&self) -> Result<TypingResponse> {
        self.client
            .send(StartTyping {
                chat_id: self.chat_id.clone(),
            })
            .await
    }

    /// Stop the typing indicator.
    pub async fn stop_typing(&self) -> Result<TypingResponse> {
        self.client
            .send(StopTyping {
                chat_id: self.chat_id.clone(),
            })
            .await
    }

    /// Mark this chat as read.
    pub async fn mark_read(&self) -> Result<ReadResponse> {
        self.client
            .send(MarkChatRead {
                chat_id: self.chat_id.clone(),
            })
            .await
    }

    /// Share the contact card into this chat.
    pub async fn share_contact_card(&self) -> Result<ShareContactCardResponse> {
        self.client
            .send(ShareContactCard {
                chat_id: self.chat_id.clone(),
            })
            .await
    }

    /// Get the chat background state.
    pub async fn background(&self) -> Result<ChatBackgroundResponse> {
        self.client
            .send(GetChatBackground {
                chat_id: self.chat_id.clone(),
            })
            .await
    }

    /// Set the chat background.
    pub async fn set_background(
        &self,
        background: impl Into<Vec<u8>>,
    ) -> Result<ChatBackgroundResponse> {
        self.client
            .send(SetChatBackground::new(self.chat_id.clone(), background))
            .await
    }

    /// Set the chat background with a fully-built upload operation.
    pub async fn set_background_with(
        &self,
        op: SetChatBackground,
    ) -> Result<ChatBackgroundResponse> {
        self.client.send(op).await
    }

    /// Remove the chat background.
    pub async fn remove_background(&self) -> Result<ChatBackgroundResponse> {
        self.client
            .send(RemoveChatBackground {
                chat_id: self.chat_id.clone(),
            })
            .await
    }
}

#[cfg(feature = "sync")]
impl<'c> Chat_<'c, crate::BlockingClient> {
    /// Fetch this chat's detail.
    pub fn get(&self) -> Result<Chat> {
        self.client.send(GetChat {
            chat_id: self.chat_id.clone(),
        })
    }

    /// List this chat's messages (first page).
    pub fn list_messages(&self) -> Result<ListChatMessagesResponse> {
        self.client
            .send(ListChatMessages::new(self.chat_id.clone()))
    }

    /// List messages with explicit filters.
    pub fn list_messages_with(&self, query: ListChatMessages) -> Result<ListChatMessagesResponse> {
        self.client.send(query)
    }

    /// Paginate over all of this chat's messages.
    pub fn list_messages_all(
        &self,
    ) -> Paginator<
        'c,
        crate::BlockingClient,
        impl Fn(u32, u32) -> ListChatMessages + use<'c>,
        ListChatMessages,
    > {
        let chat_id = self.chat_id.clone();
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, move |offset, limit| {
            ListChatMessages {
                offset: Some(offset),
                limit: Some(limit),
                ..ListChatMessages::new(chat_id.clone())
            }
        })
    }

    /// A [`SendMessage`] builder pre-seeded with this chat's id.
    pub fn message(&self) -> SendMessage {
        SendMessage::new(self.chat_id.clone())
    }

    /// Send a fully-built message.
    pub fn send(&self, message: SendMessage) -> Result<SendMessageResponse> {
        self.client.send(message)
    }

    /// Convenience: send plain text.
    pub fn send_text(&self, text: impl Into<Text>) -> Result<SendMessageResponse> {
        self.client.send(self.message().text(text))
    }

    /// Fetch a single message.
    pub fn get_message(&self, message_id: impl Into<String>) -> Result<MessageDetail> {
        self.client.send(GetMessage {
            chat_id: self.chat_id.clone(),
            message_id: message_id.into(),
        })
    }

    /// Fetch a message's delivery status.
    pub fn message_status(&self, message_id: impl Into<String>) -> Result<MessageStatus> {
        self.client.send(GetMessageStatus {
            chat_id: self.chat_id.clone(),
            message_id: message_id.into(),
        })
    }

    /// React to a message.
    pub fn add_reaction(
        &self,
        message_id: impl Into<String>,
        reaction: impl Into<String>,
        direction: Option<String>,
    ) -> Result<ReactionResponse> {
        self.client.send(AddReaction {
            chat_id: self.chat_id.clone(),
            message_id: message_id.into(),
            reaction: reaction.into(),
            direction,
        })
    }

    /// Send a poll.
    pub fn send_poll(
        &self,
        title: Option<String>,
        options: impl IntoStringList,
    ) -> Result<SendPollResponse> {
        self.client
            .send(SendPoll::new(self.chat_id.clone(), title, options))
    }

    /// Fetch poll results.
    pub fn poll_results(&self, poll_id: impl Into<String>) -> Result<PollResults> {
        self.client.send(GetPollResults {
            chat_id: self.chat_id.clone(),
            poll_id: poll_id.into(),
        })
    }

    /// Start the typing indicator.
    pub fn start_typing(&self) -> Result<TypingResponse> {
        self.client.send(StartTyping {
            chat_id: self.chat_id.clone(),
        })
    }

    /// Stop the typing indicator.
    pub fn stop_typing(&self) -> Result<TypingResponse> {
        self.client.send(StopTyping {
            chat_id: self.chat_id.clone(),
        })
    }

    /// Mark this chat as read.
    pub fn mark_read(&self) -> Result<ReadResponse> {
        self.client.send(MarkChatRead {
            chat_id: self.chat_id.clone(),
        })
    }

    /// Share the contact card into this chat.
    pub fn share_contact_card(&self) -> Result<ShareContactCardResponse> {
        self.client.send(ShareContactCard {
            chat_id: self.chat_id.clone(),
        })
    }

    /// Get the chat background state.
    pub fn background(&self) -> Result<ChatBackgroundResponse> {
        self.client.send(GetChatBackground {
            chat_id: self.chat_id.clone(),
        })
    }

    /// Set the chat background.
    pub fn set_background(&self, background: impl Into<Vec<u8>>) -> Result<ChatBackgroundResponse> {
        self.client
            .send(SetChatBackground::new(self.chat_id.clone(), background))
    }

    /// Set the chat background with a fully-built upload operation.
    pub fn set_background_with(&self, op: SetChatBackground) -> Result<ChatBackgroundResponse> {
        self.client.send(op)
    }

    /// Remove the chat background.
    pub fn remove_background(&self) -> Result<ChatBackgroundResponse> {
        self.client.send(RemoveChatBackground {
            chat_id: self.chat_id.clone(),
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

    #[test]
    fn text_serializes_untagged() {
        let one = serde_json::to_string(&Text::One("hi".into())).unwrap();
        assert_eq!(one, "\"hi\"");
        let many = serde_json::to_string(&Text::Many(vec!["a".into(), "b".into()])).unwrap();
        assert_eq!(many, "[\"a\",\"b\"]");
    }

    #[test]
    fn text_many_accepts_iterable_strings() {
        let many = serde_json::to_string(&Text::many(["a", "b"])).unwrap();
        assert_eq!(many, "[\"a\",\"b\"]");
    }

    #[test]
    fn text_many_preserves_vec_string_literal_inference() {
        let many = serde_json::to_string(&Text::many(vec!["a".into(), "b".into()])).unwrap();
        assert_eq!(many, "[\"a\",\"b\"]");
    }

    #[test]
    fn auto_idempotency_key_is_generated() {
        let msg = SendMessage::new("chat1").text("hello");
        let headers = msg.headers();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "Idempotency-Key");
        // Looks like a UUID (36 chars with dashes).
        assert_eq!(headers[0].1.len(), 36);
        assert_eq!(headers, msg.headers());
    }

    #[test]
    fn explicit_idempotency_key_is_used() {
        let msg = SendMessage::new("chat1")
            .text("hi")
            .idempotency_key("my-key");
        assert_eq!(msg.headers()[0].1, "my-key");
    }

    #[test]
    fn send_body_only_includes_set_fields() {
        let msg = SendMessage::new("chat1").text("hi");
        let body = msg.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v, serde_json::json!({ "text": "hi" }));
    }

    #[test]
    fn send_body_serializes_reply_to_message_id() {
        let msg = SendMessage::new("chat1")
            .text("reply")
            .reply_to_message_id("msg_parent");
        let body = msg.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "text": "reply",
                "reply_to": {
                    "message_id": "msg_parent"
                }
            })
        );
    }

    #[test]
    fn send_body_serializes_reply_to_guid_with_part_index() {
        let msg = SendMessage::new("chat1")
            .text("reply")
            .reply_to(ReplyToRequest::guid("raw-guid").part_index(2));
        let body = msg.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "text": "reply",
                "reply_to": {
                    "guid": "raw-guid",
                    "part_index": 2
                }
            })
        );
    }

    #[test]
    fn send_poll_new_accepts_iterable_options() {
        let op = SendPoll::new("chat1", Some("Choose".into()), ["yes", "no"]);
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "title": "Choose",
                "options": ["yes", "no"]
            })
        );
    }

    #[test]
    fn send_poll_new_preserves_vec_string_literal_inference() {
        let op = SendPoll::new(
            "chat1",
            Some("Choose".into()),
            vec!["yes".into(), "no".into()],
        );
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "title": "Choose",
                "options": ["yes", "no"]
            })
        );
    }

    #[test]
    fn set_background_path_encodes_chat_id() {
        let op = SetChatBackground::new("+15551234567", vec![1, 2, 3]);
        assert_eq!(op.path(), "/chats/%2B15551234567/background");
    }

    #[test]
    fn set_background_uses_multipart_content_type() {
        let op = SetChatBackground::new("chat1", vec![1, 2, 3]);
        assert_eq!(
            op.headers(),
            vec![(
                "Content-Type",
                "multipart/form-data; boundary=blooio-form-boundary-0".into()
            )]
        );
    }

    #[test]
    fn set_background_body_contains_raw_image_part() {
        let op = SetChatBackground::new("chat1", b"png-bytes".to_vec())
            .filename("wallpaper.png")
            .content_type("image/png");
        let body = String::from_utf8(op.body().unwrap().unwrap()).unwrap();
        assert!(body.contains("--blooio-form-boundary-0\r\n"));
        assert!(body.contains(
            "Content-Disposition: form-data; name=\"background\"; filename=\"wallpaper.png\""
        ));
        assert!(body.contains("Content-Type: image/png\r\n\r\npng-bytes"));
        assert!(body.ends_with("\r\n--blooio-form-boundary-0--\r\n"));
    }

    #[test]
    fn set_background_boundary_avoids_image_bytes() {
        let op = SetChatBackground::new("chat1", b"blooio-form-boundary-0".to_vec());
        assert_eq!(
            op.headers(),
            vec![(
                "Content-Type",
                "multipart/form-data; boundary=blooio-form-boundary-1".into()
            )]
        );
    }

    #[test]
    fn set_background_boundary_searches_past_all_collisions() {
        let background =
            b"blooio-form-boundary-0 blooio-form-boundary-1 blooio-form-boundary-2".to_vec();
        let op = SetChatBackground::new("chat1", background);
        assert_eq!(
            op.headers(),
            vec![(
                "Content-Type",
                "multipart/form-data; boundary=blooio-form-boundary-3".into()
            )]
        );
        let body = String::from_utf8(op.body().unwrap().unwrap()).unwrap();
        assert!(body.contains("--blooio-form-boundary-3\r\n"));
        assert!(body.ends_with("\r\n--blooio-form-boundary-3--\r\n"));
    }

    #[test]
    fn set_background_rejects_malformed_content_type() {
        let err = SetChatBackground::new("chat1", vec![1, 2, 3])
            .content_type("image/png\r\nX-Injected: yes")
            .body()
            .unwrap_err();
        assert!(matches!(err, crate::Error::Config(_)));
    }
}
