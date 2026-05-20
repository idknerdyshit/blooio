//! Webhooks: list/create/get/update/delete, secret rotation, logs, and replay.

use http::Method;
use serde::{Deserialize, Serialize};

use crate::core::operation::{Operation, json_body, push_opt};
use crate::core::pagination::{DEFAULT_PAGE_SIZE, Listing, Page, Pagination, Paginator};
use crate::error::Result;
use crate::types::{Webhook, WebhookLog};

// ---------------------------------------------------------------------------
// Response types specific to this resource group.
// ---------------------------------------------------------------------------

/// Response of `GET /webhooks`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ListWebhooksResponse {
    pub webhooks: Vec<Webhook>,
}

/// Response of `POST /webhooks` (200 and 201 share one struct; all fields optional).
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct CreateWebhookResponse {
    pub webhook_id: Option<String>,
    pub webhook_url: Option<String>,
    pub webhook_type: Option<String>,
    pub scope: Option<String>,
    pub message: Option<String>,
    pub created_at: Option<i64>,
    pub signing_secret: Option<String>,
}

/// Response of `DELETE /webhooks/{webhookId}`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct DeleteWebhookResponse {
    pub success: Option<bool>,
    pub message: Option<String>,
}

/// Response of `POST /webhooks/{webhookId}/secret/rotate`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct RotateSecretResponse {
    pub webhook_id: Option<String>,
    pub signing_secret: Option<String>,
    pub rotated_at: Option<i64>,
    pub rotated_by: Option<String>,
    pub rotation_count: Option<i64>,
}

/// Response of `GET /webhooks/{webhookId}/logs`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ListWebhookLogsResponse {
    pub logs: Vec<WebhookLog>,
    pub pagination: Option<Pagination>,
}

impl Listing for ListWebhookLogsResponse {
    type Item = WebhookLog;
    fn into_page(self) -> Page<Self::Item> {
        Page {
            items: self.logs,
            pagination: self.pagination,
        }
    }
}

/// Response of `POST /webhooks/{webhookId}/logs/{eventId}/replay`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ReplayResponse {
    pub success: Option<bool>,
    pub replay_event_id: Option<String>,
    pub original_event_id: Option<String>,
    pub webhook_id: Option<String>,
    pub webhook_url: Option<String>,
    pub response_status: Option<i64>,
    pub duration_ms: Option<i64>,
    pub response_data: Option<crate::types::Json>,
}

// ---------------------------------------------------------------------------
// Operations (public escape hatch — usable via `client.send(..)`).
// ---------------------------------------------------------------------------

/// `GET /webhooks`
#[derive(Debug, Clone, Default)]
pub struct ListWebhooks;

impl Operation for ListWebhooks {
    type Output = ListWebhooksResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/webhooks".into()
    }
}

/// `POST /webhooks`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct CreateWebhook {
    pub webhook_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<i64>,
}

impl CreateWebhook {
    /// Create a new builder with only the required `webhook_url`.
    pub fn new(webhook_url: impl Into<String>) -> Self {
        CreateWebhook {
            webhook_url: webhook_url.into(),
            webhook_type: None,
            valid_until: None,
        }
    }

    /// Set the webhook type.
    #[must_use]
    pub fn webhook_type(mut self, v: impl Into<String>) -> Self {
        self.webhook_type = Some(v.into());
        self
    }

    /// Set the expiry timestamp.
    #[must_use]
    pub fn valid_until(mut self, v: i64) -> Self {
        self.valid_until = Some(v);
        self
    }
}

impl Operation for CreateWebhook {
    type Output = CreateWebhookResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/webhooks".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

/// `GET /webhooks/{webhookId}`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct GetWebhook {
    pub webhook_id: String,
}

impl Operation for GetWebhook {
    type Output = Webhook;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/webhooks/{}", self.webhook_id)
    }
}

/// `PATCH /webhooks/{webhookId}`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct UpdateWebhook {
    #[serde(skip)]
    pub webhook_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecate: Option<bool>,
}

impl UpdateWebhook {
    /// Create a new builder with only the required `webhook_id`.
    pub fn new(webhook_id: impl Into<String>) -> Self {
        UpdateWebhook {
            webhook_id: webhook_id.into(),
            webhook_type: None,
            valid_until: None,
            deprecate: None,
        }
    }

    /// Set the webhook type.
    #[must_use]
    pub fn webhook_type(mut self, v: impl Into<String>) -> Self {
        self.webhook_type = Some(v.into());
        self
    }

    /// Set the expiry timestamp.
    #[must_use]
    pub fn valid_until(mut self, v: i64) -> Self {
        self.valid_until = Some(v);
        self
    }

    /// Mark the webhook as deprecated.
    #[must_use]
    pub fn deprecate(mut self, v: bool) -> Self {
        self.deprecate = Some(v);
        self
    }
}

impl Operation for UpdateWebhook {
    type Output = Webhook;
    const METHOD: Method = Method::PATCH;
    fn path(&self) -> String {
        format!("/webhooks/{}", self.webhook_id)
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

/// `DELETE /webhooks/{webhookId}`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct DeleteWebhook {
    pub webhook_id: String,
}

impl Operation for DeleteWebhook {
    type Output = DeleteWebhookResponse;
    const METHOD: Method = Method::DELETE;
    fn path(&self) -> String {
        format!("/webhooks/{}", self.webhook_id)
    }
}

/// `POST /webhooks/{webhookId}/secret/rotate`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct RotateWebhookSecret {
    pub webhook_id: String,
}

impl Operation for RotateWebhookSecret {
    type Output = RotateSecretResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!("/webhooks/{}/secret/rotate", self.webhook_id)
    }
}

/// `GET /webhooks/{webhookId}/logs`
#[allow(missing_docs)]
#[derive(Debug, Clone, Default)]
pub struct ListWebhookLogs {
    pub webhook_id: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub sort: Option<String>,
    pub status: Option<i64>,
    pub min_status: Option<i64>,
    pub max_status: Option<i64>,
}

impl Operation for ListWebhookLogs {
    type Output = ListWebhookLogsResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        format!("/webhooks/{}/logs", self.webhook_id)
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        let mut q = Vec::new();
        push_opt(&mut q, "limit", self.limit);
        push_opt(&mut q, "offset", self.offset);
        push_opt(&mut q, "sort", self.sort.as_ref());
        push_opt(&mut q, "status", self.status);
        push_opt(&mut q, "min_status", self.min_status);
        push_opt(&mut q, "max_status", self.max_status);
        q
    }
}

/// `POST /webhooks/{webhookId}/logs/{eventId}/replay`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct ReplayWebhookEvent {
    pub webhook_id: String,
    pub event_id: String,
}

impl Operation for ReplayWebhookEvent {
    type Output = ReplayResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!(
            "/webhooks/{}/logs/{}/replay",
            self.webhook_id, self.event_id
        )
    }
}

// ---------------------------------------------------------------------------
// Resource handles + accessors.
// ---------------------------------------------------------------------------

/// Handle for the `webhooks` resource group. Created via
/// [`Client::webhooks`](crate::Client::webhooks).
#[derive(Debug)]
pub struct Webhooks<'c, C> {
    pub(crate) client: &'c C,
}

/// Handle for the `webhooks/{webhookId}/logs` sub-resource. Created via
/// [`Webhooks::logs`].
#[derive(Debug)]
pub struct WebhookLogs<'c, C> {
    pub(crate) client: &'c C,
    pub(crate) webhook_id: String,
}

#[cfg(feature = "async")]
impl crate::Client {
    /// Access the webhooks resource group.
    pub fn webhooks(&self) -> Webhooks<'_, crate::Client> {
        Webhooks { client: self }
    }
}

#[cfg(feature = "sync")]
impl crate::BlockingClient {
    /// Access the webhooks resource group.
    pub fn webhooks(&self) -> Webhooks<'_, crate::BlockingClient> {
        Webhooks { client: self }
    }
}

#[cfg(feature = "async")]
impl<'c> Webhooks<'c, crate::Client> {
    /// List all webhooks.
    pub async fn list(&self) -> Result<ListWebhooksResponse> {
        self.client.send(ListWebhooks).await
    }

    /// Create a new webhook. Build the request with [`CreateWebhook::new`].
    pub async fn create(&self, op: CreateWebhook) -> Result<CreateWebhookResponse> {
        self.client.send(op).await
    }

    /// Get a webhook by id.
    pub async fn get(&self, webhook_id: impl Into<String>) -> Result<Webhook> {
        self.client
            .send(GetWebhook {
                webhook_id: webhook_id.into(),
            })
            .await
    }

    /// Update a webhook.
    pub async fn update(&self, op: UpdateWebhook) -> Result<Webhook> {
        self.client.send(op).await
    }

    /// Delete a webhook.
    pub async fn delete(&self, webhook_id: impl Into<String>) -> Result<DeleteWebhookResponse> {
        self.client
            .send(DeleteWebhook {
                webhook_id: webhook_id.into(),
            })
            .await
    }

    /// Rotate the signing secret for a webhook.
    pub async fn rotate_secret(
        &self,
        webhook_id: impl Into<String>,
    ) -> Result<RotateSecretResponse> {
        self.client
            .send(RotateWebhookSecret {
                webhook_id: webhook_id.into(),
            })
            .await
    }

    /// Access the logs sub-resource for a specific webhook.
    pub fn logs(&self, webhook_id: impl Into<String>) -> WebhookLogs<'c, crate::Client> {
        WebhookLogs {
            client: self.client,
            webhook_id: webhook_id.into(),
        }
    }
}

#[cfg(feature = "sync")]
impl<'c> Webhooks<'c, crate::BlockingClient> {
    /// List all webhooks.
    pub fn list(&self) -> Result<ListWebhooksResponse> {
        self.client.send(ListWebhooks)
    }

    /// Create a new webhook. Build the request with [`CreateWebhook::new`].
    pub fn create(&self, op: CreateWebhook) -> Result<CreateWebhookResponse> {
        self.client.send(op)
    }

    /// Get a webhook by id.
    pub fn get(&self, webhook_id: impl Into<String>) -> Result<Webhook> {
        self.client.send(GetWebhook {
            webhook_id: webhook_id.into(),
        })
    }

    /// Update a webhook.
    pub fn update(&self, op: UpdateWebhook) -> Result<Webhook> {
        self.client.send(op)
    }

    /// Delete a webhook.
    pub fn delete(&self, webhook_id: impl Into<String>) -> Result<DeleteWebhookResponse> {
        self.client.send(DeleteWebhook {
            webhook_id: webhook_id.into(),
        })
    }

    /// Rotate the signing secret for a webhook.
    pub fn rotate_secret(&self, webhook_id: impl Into<String>) -> Result<RotateSecretResponse> {
        self.client.send(RotateWebhookSecret {
            webhook_id: webhook_id.into(),
        })
    }

    /// Access the logs sub-resource for a specific webhook.
    pub fn logs(&self, webhook_id: impl Into<String>) -> WebhookLogs<'c, crate::BlockingClient> {
        WebhookLogs {
            client: self.client,
            webhook_id: webhook_id.into(),
        }
    }
}

#[cfg(feature = "async")]
impl<'c> WebhookLogs<'c, crate::Client> {
    /// List logs for this webhook (first page, no filters).
    pub async fn list(&self) -> Result<ListWebhookLogsResponse> {
        self.client
            .send(ListWebhookLogs {
                webhook_id: self.webhook_id.clone(),
                ..Default::default()
            })
            .await
    }

    /// List logs with explicit filters/pagination.
    pub async fn list_with(&self, op: ListWebhookLogs) -> Result<ListWebhookLogsResponse> {
        self.client.send(op).await
    }

    /// A paginator over all logs for this webhook.
    pub fn list_all(
        &self,
    ) -> Paginator<'c, crate::Client, impl Fn(u32, u32) -> ListWebhookLogs, ListWebhookLogs> {
        let webhook_id = self.webhook_id.clone();
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, move |offset, limit| {
            ListWebhookLogs {
                webhook_id: webhook_id.clone(),
                offset: Some(offset),
                limit: Some(limit),
                ..Default::default()
            }
        })
    }

    /// Replay a single webhook event by id.
    pub async fn replay(&self, event_id: impl Into<String>) -> Result<ReplayResponse> {
        self.client
            .send(ReplayWebhookEvent {
                webhook_id: self.webhook_id.clone(),
                event_id: event_id.into(),
            })
            .await
    }
}

#[cfg(feature = "sync")]
impl<'c> WebhookLogs<'c, crate::BlockingClient> {
    /// List logs for this webhook (first page, no filters).
    pub fn list(&self) -> Result<ListWebhookLogsResponse> {
        self.client.send(ListWebhookLogs {
            webhook_id: self.webhook_id.clone(),
            ..Default::default()
        })
    }

    /// List logs with explicit filters/pagination.
    pub fn list_with(&self, op: ListWebhookLogs) -> Result<ListWebhookLogsResponse> {
        self.client.send(op)
    }

    /// A paginator over all logs for this webhook.
    pub fn list_all(
        &self,
    ) -> Paginator<'c, crate::BlockingClient, impl Fn(u32, u32) -> ListWebhookLogs, ListWebhookLogs>
    {
        let webhook_id = self.webhook_id.clone();
        Paginator::new(self.client, DEFAULT_PAGE_SIZE, move |offset, limit| {
            ListWebhookLogs {
                webhook_id: webhook_id.clone(),
                offset: Some(offset),
                limit: Some(limit),
                ..Default::default()
            }
        })
    }

    /// Replay a single webhook event by id.
    pub fn replay(&self, event_id: impl Into<String>) -> Result<ReplayResponse> {
        self.client.send(ReplayWebhookEvent {
            webhook_id: self.webhook_id.clone(),
            event_id: event_id.into(),
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

    // --- ListWebhooks ---

    #[test]
    fn list_webhooks_method_and_path() {
        assert_eq!(ListWebhooks::METHOD, http::Method::GET);
        let op = ListWebhooks;
        assert_eq!(op.path(), "/webhooks");
    }

    // --- CreateWebhook ---

    #[test]
    fn create_webhook_method_and_path() {
        assert_eq!(CreateWebhook::METHOD, http::Method::POST);
        let op = CreateWebhook {
            webhook_url: "https://example.com/hook".into(),
            webhook_type: None,
            valid_until: None,
        };
        assert_eq!(op.path(), "/webhooks");
    }

    #[test]
    fn create_webhook_body_minimal() {
        let op = CreateWebhook {
            webhook_url: "https://example.com/hook".into(),
            webhook_type: None,
            valid_until: None,
        };
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "webhook_url": "https://example.com/hook" })
        );
    }

    #[test]
    fn create_webhook_body_populated() {
        let op = CreateWebhook {
            webhook_url: "https://example.com/hook".into(),
            webhook_type: Some("all".into()),
            valid_until: Some(1893456000),
        };
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "webhook_url": "https://example.com/hook",
                "webhook_type": "all",
                "valid_until": 1893456000_i64
            })
        );
    }

    // --- GetWebhook ---

    #[test]
    fn get_webhook_method_and_path() {
        assert_eq!(GetWebhook::METHOD, http::Method::GET);
        let op = GetWebhook {
            webhook_id: "wh1".into(),
        };
        assert_eq!(op.path(), "/webhooks/wh1");
    }

    // --- UpdateWebhook builder ---

    #[test]
    fn update_webhook_method_and_path() {
        assert_eq!(UpdateWebhook::METHOD, http::Method::PATCH);
        let op = UpdateWebhook::new("wh1");
        assert_eq!(op.path(), "/webhooks/wh1");
    }

    #[test]
    fn update_webhook_body_minimal_all_none() {
        let op = UpdateWebhook::new("wh1");
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // All optional fields absent → empty object; webhook_id is #[serde(skip)]
        assert_eq!(v, serde_json::json!({}));
    }

    #[test]
    fn update_webhook_body_with_webhook_type() {
        let op = UpdateWebhook::new("wh1").webhook_type("message");
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v, serde_json::json!({ "webhook_type": "message" }));
    }

    #[test]
    fn update_webhook_body_with_valid_until() {
        let op = UpdateWebhook::new("wh1").valid_until(1893456000);
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v, serde_json::json!({ "valid_until": 1893456000_i64 }));
    }

    #[test]
    fn update_webhook_body_deprecate() {
        let op = UpdateWebhook::new("wh1").deprecate(true);
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v, serde_json::json!({ "deprecate": true }));
    }

    #[test]
    fn update_webhook_body_fully_populated() {
        let op = UpdateWebhook::new("wh1")
            .webhook_type("all")
            .valid_until(1893456000)
            .deprecate(false);
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            v,
            serde_json::json!({
                "webhook_type": "all",
                "valid_until": 1893456000_i64,
                "deprecate": false
            })
        );
    }

    #[test]
    fn update_webhook_webhook_id_not_in_body() {
        let op = UpdateWebhook::new("wh1").webhook_type("all");
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(v.get("webhook_id").is_none());
    }

    // --- DeleteWebhook ---

    #[test]
    fn delete_webhook_method_and_path() {
        assert_eq!(DeleteWebhook::METHOD, http::Method::DELETE);
        let op = DeleteWebhook {
            webhook_id: "wh1".into(),
        };
        assert_eq!(op.path(), "/webhooks/wh1");
    }

    // --- RotateWebhookSecret ---

    #[test]
    fn rotate_webhook_secret_method_and_path() {
        assert_eq!(RotateWebhookSecret::METHOD, http::Method::POST);
        let op = RotateWebhookSecret {
            webhook_id: "wh1".into(),
        };
        assert_eq!(op.path(), "/webhooks/wh1/secret/rotate");
    }

    // --- ListWebhookLogs ---

    #[test]
    fn list_webhook_logs_method_and_path() {
        assert_eq!(ListWebhookLogs::METHOD, http::Method::GET);
        let op = ListWebhookLogs {
            webhook_id: "wh1".into(),
            limit: None,
            offset: None,
            sort: None,
            status: None,
            min_status: None,
            max_status: None,
        };
        assert_eq!(op.path(), "/webhooks/wh1/logs");
    }

    #[test]
    fn list_webhook_logs_query_empty_when_no_options() {
        let op = ListWebhookLogs {
            webhook_id: "wh1".into(),
            limit: None,
            offset: None,
            sort: None,
            status: None,
            min_status: None,
            max_status: None,
        };
        assert!(op.query().is_empty());
    }

    #[test]
    fn list_webhook_logs_query_with_all_options() {
        let op = ListWebhookLogs {
            webhook_id: "wh1".into(),
            limit: Some(10),
            offset: Some(5),
            sort: Some("desc".into()),
            status: Some(200),
            min_status: Some(200),
            max_status: Some(299),
        };
        let q = op.query();
        assert_eq!(q.len(), 6);
        assert!(q.contains(&("limit", "10".into())));
        assert!(q.contains(&("offset", "5".into())));
        assert!(q.contains(&("sort", "desc".into())));
        assert!(q.contains(&("status", "200".into())));
        assert!(q.contains(&("min_status", "200".into())));
        assert!(q.contains(&("max_status", "299".into())));
    }

    #[test]
    fn list_webhook_logs_query_omits_unset_optionals() {
        let op = ListWebhookLogs {
            webhook_id: "wh1".into(),
            limit: Some(20),
            offset: None,
            sort: None,
            status: None,
            min_status: None,
            max_status: None,
        };
        let q = op.query();
        assert_eq!(q.len(), 1);
        assert!(q.contains(&("limit", "20".into())));
    }

    // --- ReplayWebhookEvent ---

    #[test]
    fn replay_webhook_event_method_and_path() {
        assert_eq!(ReplayWebhookEvent::METHOD, http::Method::POST);
        let op = ReplayWebhookEvent {
            webhook_id: "wh1".into(),
            event_id: "evt1".into(),
        };
        assert_eq!(op.path(), "/webhooks/wh1/logs/evt1/replay");
    }
}
