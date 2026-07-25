//! V4 message operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::v4::{
    CursorPaginator,
    types::{
        Hybrid, ItemEnvelope, ListEnvelope, Message, MessageContentFields, MessageEvent,
        MessageSendResult, MessageStatus, ReactionResult, Recipient,
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
    #[serde(flatten)]
    pub content: MessageContentFields,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
}
impl SendMessageToChat {
    /// Construct a chat send.
    #[must_use]
    pub fn new(chat_id: impl Into<String>, content: MessageContentFields) -> Self {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    pub to: Recipient,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_type: Option<crate::v4::types::ChannelType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid: Option<Hybrid>,
    #[serde(flatten)]
    pub content: MessageContentFields,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
}
impl SendMessage {
    /// Construct a globally routed send.
    #[must_use]
    pub fn new(to: Recipient, content: MessageContentFields) -> Self {
        Self {
            from: None,
            to,
            priority_id: None,
            channel_type: None,
            hybrid: None,
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
    pub async fn send(&self, content: MessageContentFields) -> Result<MessageSendResult> {
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
    pub fn send(&self, content: MessageContentFields) -> Result<MessageSendResult> {
        self.client
            .send(SendMessageToChat::new(self.chat_id.clone(), content))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::v4::types::{LinkPreview, MultipartPart, TemplateFields};
    use serde_json::json;

    #[test]
    fn message_content_fields_serializes_text_only() {
        let body = serde_json::to_value(MessageContentFields::text("hello")).unwrap();
        assert_eq!(body, json!({"text": "hello"}));
    }

    #[test]
    fn message_content_fields_serializes_media_only() {
        let body = serde_json::to_value(MessageContentFields::media(&[
            "https://example.com/img.jpg",
        ]))
        .unwrap();
        assert_eq!(
            body,
            json!({"attachments": ["https://example.com/img.jpg"]})
        );
    }

    #[test]
    fn message_content_fields_serializes_rich_link() {
        let body = MessageContentFields::rich_link("https://example.com", "Example");
        let body = serde_json::to_value(body).unwrap();
        assert_eq!(
            body,
            json!({"rich_link": {"url": "https://example.com", "title": "Example"}})
        );
    }

    #[test]
    fn message_content_fields_serializes_poll() {
        let body =
            serde_json::to_value(MessageContentFields::poll("Lunch?", &["Yes", "No"])).unwrap();
        assert_eq!(
            body,
            json!({"poll": {"title": "Lunch?", "options": ["Yes", "No"]}})
        );
    }

    #[test]
    fn message_content_fields_serializes_template() {
        let body = serde_json::to_value(MessageContentFields::template("tpl_welcome")).unwrap();
        assert_eq!(body, json!({"template": {"template_id": "tpl_welcome"}}));
    }

    #[test]
    fn message_content_fields_serializes_provider_template_without_id() {
        let mut template = TemplateFields::default();
        template
            .extra
            .insert("provider_template_name".into(), json!("welcome"));
        let body = serde_json::to_value(MessageContentFields {
            template: Some(template),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            body,
            json!({"template": {"provider_template_name": "welcome"}})
        );
    }

    #[test]
    fn message_content_fields_serializes_interactive() {
        let mut content =
            MessageContentFields::interactive(MessageContentFields::INTERACTIVE_KIND_CAROUSEL);
        content
            .interactive
            .as_mut()
            .unwrap()
            .extra
            .insert("cards".into(), json!([{"title": "One"}]));
        let body = serde_json::to_value(content).unwrap();
        assert_eq!(
            body,
            json!({
                "interactive": {
                    "kind": "carousel",
                    "cards": [{"title": "One"}]
                }
            })
        );
    }

    #[test]
    fn message_content_fields_serializes_parts() {
        let body = serde_json::to_value(MessageContentFields {
            parts: Some(vec![
                MultipartPart {
                    text: Some("hello".into()),
                    url: None,
                },
                MultipartPart {
                    text: None,
                    url: Some("https://example.com/img.jpg".into()),
                },
            ]),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            body,
            json!({"parts": [
                {"text": "hello"},
                {"url": "https://example.com/img.jpg"}
            ]})
        );
    }

    #[test]
    fn message_content_fields_serializes_reply_to() {
        let body = serde_json::to_value(MessageContentFields {
            reply_to: Some("msg_abc".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(body, json!({"reply_to": "msg_abc"}));
    }

    #[test]
    fn message_content_fields_serializes_effect() {
        let body = serde_json::to_value(MessageContentFields {
            effect: Some("slam".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(body, json!({"effect": "slam"}));
    }

    #[test]
    fn message_content_fields_serializes_link_preview() {
        let body = serde_json::to_value(MessageContentFields {
            link_preview: Some(LinkPreview {
                image_url: Some("https://example.com/thumb.jpg".into()),
                title: Some("Example".into()),
            }),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(
            body,
            json!({"link_preview": {"imageUrl": "https://example.com/thumb.jpg", "title": "Example"}})
        );
    }

    #[test]
    fn send_message_serializes_flat_body() {
        let send = SendMessage::new(
            Recipient::identifier("+15551234567"),
            MessageContentFields::text("hello"),
        );
        let body = serde_json::to_value(&send).unwrap();
        assert_eq!(
            body,
            json!({
                "to": "+15551234567",
                "text": "hello"
            })
        );
    }

    #[test]
    fn send_message_to_chat_serializes_flat_body() {
        let send = SendMessageToChat::new("chat_1", MessageContentFields::text("hello"));
        let body = serde_json::to_value(&send).unwrap();
        assert_eq!(body, json!({"text": "hello"}));
    }

    #[test]
    fn send_message_serializes_with_routing_controls() {
        let send = SendMessage {
            from: Some("+15551230001".into()),
            to: Recipient::identifier("+15551234567"),
            priority_id: Some("priority_1".into()),
            channel_type: Some(crate::v4::types::ChannelType::Blooio),
            hybrid: Some(Hybrid::On(true)),
            content: MessageContentFields::text("hello"),
            dry_run: Some(true),
        };
        let body = serde_json::to_value(&send).unwrap();
        assert_eq!(
            body,
            json!({
                "from": "+15551230001",
                "to": "+15551234567",
                "priority_id": "priority_1",
                "channel_type": "blooio",
                "hybrid": true,
                "text": "hello",
                "dry_run": true,
            })
        );
    }

    #[test]
    fn hybrid_serializes_boolean_and_string() {
        let on = serde_json::to_value(Hybrid::On(true)).unwrap();
        assert_eq!(on, json!(true));

        let number = serde_json::to_value(Hybrid::Number("+15551230001".into())).unwrap();
        assert_eq!(number, json!("+15551230001"));
    }
}
