//! V4 channel operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::v4::{
    CursorPaginator,
    types::{
        Channel, ChannelCapabilities, ChannelType, ItemEnvelope, ListEnvelope,
        MessageContentFields, MessageSendResult,
    },
};
use crate::{
    Result,
    core::operation::{encode_path_segment, json_body, push_opt},
};
use http::Method;
use serde_json::Value;

/// List channels with cursor pagination.
#[derive(Debug, Clone, Default)]
pub struct ListChannels {
    pub channel_type: Option<ChannelType>,
    pub status: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}
impl crate::Operation for ListChannels {
    type Output = ListEnvelope<Channel>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/channels".into()
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(
            &mut q,
            "type",
            self.channel_type.as_ref().map(ChannelType::wire_value),
        );
        push_opt(&mut q, "status", self.status.as_ref());
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "cursor", self.cursor.as_ref());
        q
    }
}
impl_v4_operation!(ListChannels);

/// Get a channel.
#[derive(Debug, Clone)]
pub struct GetChannel {
    pub channel_id: String,
}
impl crate::Operation for GetChannel {
    type Output = ItemEnvelope<Channel>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/channels/{}", encode_path_segment(&self.channel_id))
    }
}
impl_v4_operation!(GetChannel);

/// Get channel capabilities.
#[derive(Debug, Clone)]
pub struct GetChannelCapabilities {
    pub channel_id: String,
}
impl crate::Operation for GetChannelCapabilities {
    type Output = ItemEnvelope<ChannelCapabilities>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/channels/{}/capabilities",
            encode_path_segment(&self.channel_id)
        )
    }
}
impl_v4_operation!(GetChannelCapabilities);

/// Update a channel profile with the provider-defined fields.
#[derive(Debug, Clone)]
pub struct UpdateChannelProfile {
    pub channel_id: String,
    pub fields: Value,
}
impl crate::Operation for UpdateChannelProfile {
    type Output = ItemEnvelope<Channel>;
    const METHOD: Method = Method::PUT;
    fn path(&self) -> String {
        format!(
            "/channels/{}/profile",
            encode_path_segment(&self.channel_id)
        )
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(&self.fields)
    }
}
impl_v4_operation!(UpdateChannelProfile);

/// Send content through an exact channel.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SendMessageToChannel {
    #[serde(skip)]
    pub channel_id: String,
    pub to: crate::v4::types::Recipient,
    #[serde(flatten)]
    pub content: MessageContentFields,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_run: Option<bool>,
}
impl SendMessageToChannel {
    /// Construct a channel send.
    #[must_use]
    pub fn new(
        channel_id: impl Into<String>,
        to: crate::v4::types::Recipient,
        content: MessageContentFields,
    ) -> Self {
        Self {
            channel_id: channel_id.into(),
            to,
            content,
            dry_run: None,
        }
    }
}
impl crate::Operation for SendMessageToChannel {
    type Output = MessageSendResult;
    const METHOD: Method = Method::POST;
    const RETRY_SAFE: bool = true;
    fn path(&self) -> String {
        format!(
            "/channels/{}/messages",
            encode_path_segment(&self.channel_id)
        )
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(SendMessageToChannel);

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::{ListChannels, SendMessageToChannel};
    use crate::{
        Operation,
        v4::types::{ChannelType, MessageContentFields, Recipient},
    };
    use serde_json::json;

    #[test]
    fn list_channels_preserves_an_unknown_channel_filter() {
        let operation = ListChannels {
            channel_type: Some(ChannelType::Unknown("future_provider".into())),
            ..Default::default()
        };

        assert_eq!(operation.query(), [("type", "future_provider".into())]);
    }

    #[test]
    fn send_message_to_channel_serializes_flat_body() {
        let send = SendMessageToChannel::new(
            "channel_1",
            Recipient::identifier("+15551234567"),
            MessageContentFields::text("hello"),
        );
        let body = serde_json::to_value(&send).unwrap();
        assert_eq!(body, json!({"to": "+15551234567", "text": "hello"}));
    }
}

/// Channel collection handle.
#[derive(Debug)]
pub struct Channels<C> {
    pub(crate) client: C,
}
#[cfg(feature = "async")]
impl crate::v4::BlooioAccount<'_> {
    /// Access channels.
    #[must_use]
    pub fn channels(self) -> Channels<Self> {
        Channels { client: self }
    }
}
#[cfg(feature = "sync")]
impl crate::v4::BlockingBlooioAccount<'_> {
    /// Access channels.
    #[must_use]
    pub fn channels(self) -> Channels<Self> {
        Channels { client: self }
    }
}

#[cfg(feature = "async")]
impl<'a> Channels<crate::v4::BlooioAccount<'a>> {
    /// List the first channel page.
    pub async fn list(&self) -> Result<ListEnvelope<Channel>> {
        self.client.send(ListChannels::default()).await
    }
    /// Cursor over all channels.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListChannels + use<'a>,
        ListChannels,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListChannels {
                cursor,
                limit: Some(limit),
                ..Default::default()
            },
        )
    }
}
#[cfg(feature = "sync")]
impl<'a> Channels<crate::v4::BlockingBlooioAccount<'a>> {
    /// List the first channel page.
    pub fn list(&self) -> Result<ListEnvelope<Channel>> {
        self.client.send(ListChannels::default())
    }
    /// Cursor over all channels.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlockingBlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListChannels + use<'a>,
        ListChannels,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListChannels {
                cursor,
                limit: Some(limit),
                ..Default::default()
            },
        )
    }
}
