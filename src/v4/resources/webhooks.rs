//! V4 webhook subscription and delivery operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::v4::{
    CursorPaginator,
    types::{
        ActionResponse, ChannelType, ItemEnvelope, ListEnvelope, SecretResult, Webhook,
        WebhookDelivery, WebhookWithSecret,
    },
};
use crate::{
    Result,
    core::operation::{encode_path_segment, json_body, push_opt},
};
use http::Method;
use serde::Serialize;
use serde_json::Value;

/// List webhooks.
#[derive(Debug, Clone, Default)]
pub struct ListWebhooks {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}
impl crate::Operation for ListWebhooks {
    type Output = ListEnvelope<Webhook>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/webhooks".into()
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "cursor", self.cursor.as_ref());
        q
    }
}
impl_v4_operation!(ListWebhooks);

/// Create a webhook.
#[derive(Debug, Clone)]
pub struct CreateWebhook {
    pub url: String,
    /// Retired event filter. Omit this or send only `["*"]`.
    pub event_types: Option<Vec<String>>,
    /// API key used to scope delivery, redacted from diagnostics.
    pub api_key: Option<crate::Secret<String>>,
    /// Integration identifier used to scope delivery.
    pub integration_id: Option<String>,
    pub channel_id: Option<String>,
    pub channel_type: Option<ChannelType>,
}
impl CreateWebhook {
    /// Construct a webhook subscription.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            event_types: None,
            api_key: None,
            integration_id: None,
            channel_id: None,
            channel_type: None,
        }
    }

    /// Scope delivery to lines owned by one API key.
    #[must_use]
    pub fn scope_to_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(crate::Secret::new(api_key.into()));
        self.integration_id = None;
        self
    }

    /// Scope delivery to lines owned by one integration.
    #[must_use]
    pub fn scope_to_integration(mut self, integration_id: impl Into<String>) -> Self {
        self.integration_id = Some(integration_id.into());
        self.api_key = None;
        self
    }
}
impl crate::Operation for CreateWebhook {
    type Output = ItemEnvelope<WebhookWithSecret>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/webhooks".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        #[derive(Serialize)]
        struct Body<'a> {
            url: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            event_types: Option<&'a [String]>,
            #[serde(skip_serializing_if = "Option::is_none")]
            api_key: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            integration_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            channel_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            channel_type: Option<&'a ChannelType>,
        }

        json_body(&Body {
            url: &self.url,
            event_types: self.event_types.as_deref(),
            api_key: self.api_key.as_ref().map(|value| value.expose().as_str()),
            integration_id: self.integration_id.as_deref(),
            channel_id: self.channel_id.as_deref(),
            channel_type: self.channel_type.as_ref(),
        })
    }
}
impl_v4_operation!(CreateWebhook);

/// Get a webhook.
#[derive(Debug, Clone)]
pub struct GetWebhook {
    pub webhook_id: String,
}
impl crate::Operation for GetWebhook {
    type Output = ItemEnvelope<Webhook>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/webhooks/{}", encode_path_segment(&self.webhook_id))
    }
}
impl_v4_operation!(GetWebhook);
/// Update a webhook.
#[derive(Debug, Clone)]
pub struct UpdateWebhook {
    pub webhook_id: String,
    pub fields: Value,
}
impl crate::Operation for UpdateWebhook {
    type Output = ItemEnvelope<Webhook>;
    const METHOD: Method = Method::PATCH;
    fn path(&self) -> String {
        format!("/webhooks/{}", encode_path_segment(&self.webhook_id))
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(&self.fields)
    }
}
impl_v4_operation!(UpdateWebhook);
/// Delete a webhook.
#[derive(Debug, Clone)]
pub struct DeleteWebhook {
    pub webhook_id: String,
}
impl crate::Operation for DeleteWebhook {
    type Output = ActionResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/webhooks/{}", encode_path_segment(&self.webhook_id))
    }
}
impl_v4_operation!(DeleteWebhook);
/// Rotate a webhook secret.
#[derive(Debug, Clone)]
pub struct RotateWebhookSecret {
    pub webhook_id: String,
}
impl crate::Operation for RotateWebhookSecret {
    type Output = ItemEnvelope<SecretResult>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!(
            "/webhooks/{}/secret/rotate",
            encode_path_segment(&self.webhook_id)
        )
    }
}
impl_v4_operation!(RotateWebhookSecret);

/// List delivery attempts.
#[derive(Debug, Clone, Default)]
pub struct ListWebhookDeliveries {
    pub webhook_id: String,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}
impl crate::Operation for ListWebhookDeliveries {
    type Output = ListEnvelope<WebhookDelivery>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!(
            "/webhooks/{}/deliveries",
            encode_path_segment(&self.webhook_id)
        )
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "cursor", self.cursor.as_ref());
        q
    }
}
impl_v4_operation!(ListWebhookDeliveries);
/// Replay a webhook delivery.
#[derive(Debug, Clone)]
pub struct ReplayWebhookDelivery {
    pub webhook_id: String,
    pub delivery_id: String,
}
impl crate::Operation for ReplayWebhookDelivery {
    type Output = ActionResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!(
            "/webhooks/{}/deliveries/{}/replay",
            encode_path_segment(&self.webhook_id),
            encode_path_segment(&self.delivery_id)
        )
    }
}
impl_v4_operation!(ReplayWebhookDelivery);

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::CreateWebhook;
    use crate::Operation;
    use serde_json::json;

    #[test]
    fn webhook_scope_body_is_typed_and_debug_redacts_api_keys() {
        let operation =
            CreateWebhook::new("https://example.com/hook").scope_to_api_key("bl_live_sensitive");
        let body: serde_json::Value =
            serde_json::from_slice(&operation.body().unwrap().unwrap()).unwrap();
        assert_eq!(
            body,
            json!({"url": "https://example.com/hook", "api_key": "bl_live_sensitive"})
        );
        assert!(!format!("{operation:?}").contains("bl_live_sensitive"));

        let integration =
            CreateWebhook::new("https://example.com/hook").scope_to_integration("int_1");
        let body: serde_json::Value =
            serde_json::from_slice(&integration.body().unwrap().unwrap()).unwrap();
        assert_eq!(
            body,
            json!({"url": "https://example.com/hook", "integration_id": "int_1"})
        );
    }
}

/// Webhook collection handle.
#[derive(Debug)]
pub struct Webhooks<C> {
    pub(crate) client: C,
}
#[cfg(feature = "async")]
impl crate::v4::BlooioAccount<'_> {
    /// Access webhooks.
    #[must_use]
    pub fn webhooks(self) -> Webhooks<Self> {
        Webhooks { client: self }
    }
}
#[cfg(feature = "sync")]
impl crate::v4::BlockingBlooioAccount<'_> {
    /// Access webhooks.
    #[must_use]
    pub fn webhooks(self) -> Webhooks<Self> {
        Webhooks { client: self }
    }
}
#[cfg(feature = "async")]
impl<'a> Webhooks<crate::v4::BlooioAccount<'a>> {
    /// List webhooks.
    pub async fn list(&self) -> Result<ListEnvelope<Webhook>> {
        self.client.send(ListWebhooks::default()).await
    }
    /// Cursor over webhooks.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListWebhooks + use<'a>,
        ListWebhooks,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListWebhooks {
                cursor,
                limit: Some(limit),
            },
        )
    }
    /// Create a webhook.
    pub async fn create(
        &self,
        operation: CreateWebhook,
    ) -> Result<ItemEnvelope<WebhookWithSecret>> {
        self.client.send(operation).await
    }
}
#[cfg(feature = "sync")]
impl<'a> Webhooks<crate::v4::BlockingBlooioAccount<'a>> {
    /// List webhooks.
    pub fn list(&self) -> Result<ListEnvelope<Webhook>> {
        self.client.send(ListWebhooks::default())
    }
    /// Cursor over webhooks.
    pub fn list_all(
        &self,
    ) -> CursorPaginator<
        crate::v4::BlockingBlooioAccount<'a>,
        impl Fn(Option<String>, u32) -> ListWebhooks + use<'a>,
        ListWebhooks,
    > {
        CursorPaginator::new(
            self.client,
            crate::core::pagination::DEFAULT_PAGE_SIZE,
            |cursor, limit| ListWebhooks {
                cursor,
                limit: Some(limit),
            },
        )
    }
    /// Create a webhook.
    pub fn create(&self, operation: CreateWebhook) -> Result<ItemEnvelope<WebhookWithSecret>> {
        self.client.send(operation)
    }
}
