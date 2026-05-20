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

/// Pagination metadata returned inside `ListWebhookLogsResponse`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct WebhookLogPagination {
    pub total: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub returned: Option<i64>,
    pub has_more: Option<bool>,
}

/// Response of `GET /webhooks/{webhookId}/logs`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ListWebhookLogsResponse {
    pub logs: Vec<WebhookLog>,
    pub pagination: Option<WebhookLogPagination>,
}

impl Listing for ListWebhookLogsResponse {
    type Item = WebhookLog;
    fn into_page(self) -> Page<Self::Item> {
        Page {
            items: self.logs,
            pagination: self.pagination.map(|p| Pagination {
                limit: p.limit,
                offset: p.offset,
                total: p.total,
            }),
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
#[derive(Debug, Clone)]
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

    /// Create a new webhook.
    pub async fn create(
        &self,
        webhook_url: impl Into<String>,
        webhook_type: Option<String>,
        valid_until: Option<i64>,
    ) -> Result<CreateWebhookResponse> {
        self.client
            .send(CreateWebhook {
                webhook_url: webhook_url.into(),
                webhook_type,
                valid_until,
            })
            .await
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

    /// Create a new webhook.
    pub fn create(
        &self,
        webhook_url: impl Into<String>,
        webhook_type: Option<String>,
        valid_until: Option<i64>,
    ) -> Result<CreateWebhookResponse> {
        self.client.send(CreateWebhook {
            webhook_url: webhook_url.into(),
            webhook_type,
            valid_until,
        })
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
                limit: None,
                offset: None,
                sort: None,
                status: None,
                min_status: None,
                max_status: None,
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
                sort: None,
                status: None,
                min_status: None,
                max_status: None,
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
            limit: None,
            offset: None,
            sort: None,
            status: None,
            min_status: None,
            max_status: None,
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
                sort: None,
                status: None,
                min_status: None,
                max_status: None,
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
