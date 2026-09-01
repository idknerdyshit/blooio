//! V4 channel operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::v4::{
    CursorPaginator,
    types::{
        AvailableBlooioNumbers, BlooioNumberType, BlooioPurchase, Channel, ChannelCapabilities,
        ChannelProfile, ChannelType, ItemEnvelope, ListEnvelope, MessageContentFields,
        MessageSendResult, RemovedBlooioNumber,
    },
};
use crate::{
    Result,
    core::operation::{encode_path_segment, json_body, push_opt},
};
use http::Method;

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

/// Browse Blooio number inventory or request an area-code quote.
#[derive(Debug, Clone, Default)]
pub struct ListAvailableBlooioNumbers {
    pub number_type: Option<BlooioNumberType>,
    pub area_codes: Vec<String>,
    pub country: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}
impl crate::Operation for ListAvailableBlooioNumbers {
    type Output = AvailableBlooioNumbers;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/channels/blooio/available".into()
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(
            &mut q,
            "type",
            self.number_type.as_ref().map(BlooioNumberType::wire_value),
        );
        q.extend(
            self.area_codes
                .iter()
                .cloned()
                .map(|area_code| ("area_code", area_code)),
        );
        push_opt(&mut q, "country", self.country.as_ref());
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "cursor", self.cursor.as_ref());
        q
    }
}
impl_v4_operation!(ListAvailableBlooioNumbers);

/// Purchase one or more Blooio numbers.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PurchaseBlooioNumbers {
    #[serde(skip)]
    pub idempotency_key: crate::Secret<String>,
    pub plan: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub area_codes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zip_codes: Vec<String>,
}
impl PurchaseBlooioNumbers {
    /// Construct a billable, idempotent number purchase.
    #[must_use]
    pub fn new(plan: impl Into<String>, idempotency_key: impl Into<String>) -> Self {
        Self {
            idempotency_key: crate::Secret::new(idempotency_key.into()),
            plan: plan.into(),
            quantity: None,
            area_codes: Vec::new(),
            zip_codes: Vec::new(),
        }
    }
}
impl crate::Operation for PurchaseBlooioNumbers {
    type Output = ItemEnvelope<BlooioPurchase>;
    const METHOD: Method = Method::POST;
    const RETRY_SAFE: bool = true;
    fn path(&self) -> String {
        "/channels/blooio/purchases".into()
    }
    fn headers(&self) -> Vec<(&'static str, String)> {
        vec![("Idempotency-Key", self.idempotency_key.expose().clone())]
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(PurchaseBlooioNumbers);

/// Get the current state of an asynchronous Blooio number purchase.
#[derive(Debug, Clone)]
pub struct GetBlooioPurchase {
    pub purchase_id: String,
}
impl crate::Operation for GetBlooioPurchase {
    type Output = ItemEnvelope<BlooioPurchase>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/channels/blooio/purchases/{}",
            encode_path_segment(&self.purchase_id)
        )
    }
}
impl_v4_operation!(GetBlooioPurchase);

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

/// Remove an owned Blooio number and cancel its subscription.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RemoveBlooioNumber {
    #[serde(skip)]
    pub channel_id: String,
    pub reasons: Vec<String>,
}
impl RemoveBlooioNumber {
    /// Construct a number removal with one or more churn reasons.
    #[must_use]
    pub fn new<I, S>(channel_id: impl Into<String>, reasons: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            channel_id: channel_id.into(),
            reasons: reasons.into_iter().map(Into::into).collect(),
        }
    }
}
impl crate::Operation for RemoveBlooioNumber {
    type Output = ItemEnvelope<RemovedBlooioNumber>;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/channels/{}", encode_path_segment(&self.channel_id))
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(RemoveBlooioNumber);

/// Update a channel profile with the provider-defined fields.
#[derive(Debug, Clone)]
pub struct UpdateChannelProfile {
    pub channel_id: String,
    pub fields: ChannelProfile,
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
    use super::{
        GetBlooioPurchase, ListAvailableBlooioNumbers, ListChannels, PurchaseBlooioNumbers,
        RemoveBlooioNumber, SendMessageToChannel,
    };
    use crate::{
        Operation,
        v4::types::{BlooioNumberType, ChannelType, MessageContentFields, Recipient},
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

    #[test]
    fn number_lifecycle_operations_match_the_wire_contract() {
        let available = ListAvailableBlooioNumbers {
            number_type: Some(BlooioNumberType::Dedicated),
            area_codes: vec!["415".into(), "628".into()],
            country: Some("US".into()),
            limit: Some(10),
            cursor: Some("next-page".into()),
        };
        assert_eq!(
            available.query(),
            [
                ("type", "dedicated".into()),
                ("area_code", "415".into()),
                ("area_code", "628".into()),
                ("country", "US".into()),
                ("limit", "10".into()),
                ("cursor", "next-page".into()),
            ]
        );

        let mut purchase = PurchaseBlooioNumbers::new("dedicated", "purchase-1");
        purchase.quantity = Some(2);
        purchase.area_codes = vec!["415".into(), "628".into()];
        assert_eq!(
            purchase.headers(),
            [("Idempotency-Key", "purchase-1".into())]
        );
        assert_eq!(
            serde_json::to_value(&purchase).unwrap(),
            json!({"plan": "dedicated", "quantity": 2, "area_codes": ["415", "628"]})
        );

        let status = GetBlooioPurchase {
            purchase_id: "purchase/1".into(),
        };
        assert_eq!(status.path(), "/channels/blooio/purchases/purchase%2F1");

        let remove = RemoveBlooioNumber::new("+15551234567", ["no_longer_needed"]);
        assert_eq!(remove.path(), "/channels/%2B15551234567");
        assert_eq!(
            serde_json::to_value(remove).unwrap(),
            json!({"reasons": ["no_longer_needed"]})
        );
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
    /// Browse Blooio number inventory or quote area codes.
    pub async fn available(
        &self,
        operation: ListAvailableBlooioNumbers,
    ) -> Result<AvailableBlooioNumbers> {
        self.client.send(operation).await
    }
    /// Cursor over all available Blooio numbers matching the supplied filters.
    pub fn available_all(
        &self,
        operation: ListAvailableBlooioNumbers,
    ) -> CursorPaginator<
        crate::v4::BlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListAvailableBlooioNumbers + use<'a>,
        ListAvailableBlooioNumbers,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            move |cursor, limit| ListAvailableBlooioNumbers {
                cursor,
                limit: Some(limit),
                ..operation.clone()
            },
        )
    }
    /// Submit a billable Blooio number purchase.
    pub async fn purchase(
        &self,
        operation: PurchaseBlooioNumbers,
    ) -> Result<ItemEnvelope<BlooioPurchase>> {
        self.client.send(operation).await
    }
    /// Get an asynchronous Blooio number purchase.
    pub async fn purchase_status(
        &self,
        purchase_id: impl Into<String>,
    ) -> Result<ItemEnvelope<BlooioPurchase>> {
        self.client
            .send(GetBlooioPurchase {
                purchase_id: purchase_id.into(),
            })
            .await
    }
    /// Remove an owned Blooio number.
    pub async fn remove(
        &self,
        operation: RemoveBlooioNumber,
    ) -> Result<ItemEnvelope<RemovedBlooioNumber>> {
        self.client.send(operation).await
    }
    /// Partially update profile metadata for a channel.
    pub async fn update_profile(
        &self,
        channel_id: impl Into<String>,
        fields: ChannelProfile,
    ) -> Result<ItemEnvelope<Channel>> {
        self.client
            .send(UpdateChannelProfile {
                channel_id: channel_id.into(),
                fields,
            })
            .await
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
    /// Browse Blooio number inventory or quote area codes.
    pub fn available(
        &self,
        operation: ListAvailableBlooioNumbers,
    ) -> Result<AvailableBlooioNumbers> {
        self.client.send(operation)
    }
    /// Cursor over all available Blooio numbers matching the supplied filters.
    pub fn available_all(
        &self,
        operation: ListAvailableBlooioNumbers,
    ) -> CursorPaginator<
        crate::v4::BlockingBlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListAvailableBlooioNumbers + use<'a>,
        ListAvailableBlooioNumbers,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            move |cursor, limit| ListAvailableBlooioNumbers {
                cursor,
                limit: Some(limit),
                ..operation.clone()
            },
        )
    }
    /// Submit a billable Blooio number purchase.
    pub fn purchase(
        &self,
        operation: PurchaseBlooioNumbers,
    ) -> Result<ItemEnvelope<BlooioPurchase>> {
        self.client.send(operation)
    }
    /// Get an asynchronous Blooio number purchase.
    pub fn purchase_status(
        &self,
        purchase_id: impl Into<String>,
    ) -> Result<ItemEnvelope<BlooioPurchase>> {
        self.client.send(GetBlooioPurchase {
            purchase_id: purchase_id.into(),
        })
    }
    /// Remove an owned Blooio number.
    pub fn remove(
        &self,
        operation: RemoveBlooioNumber,
    ) -> Result<ItemEnvelope<RemovedBlooioNumber>> {
        self.client.send(operation)
    }
    /// Partially update profile metadata for a channel.
    pub fn update_profile(
        &self,
        channel_id: impl Into<String>,
        fields: ChannelProfile,
    ) -> Result<ItemEnvelope<Channel>> {
        self.client.send(UpdateChannelProfile {
            channel_id: channel_id.into(),
            fields,
        })
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
