use super::ChannelType;
use crate::Secret;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// A v4 webhook subscription.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Webhook {
    pub id: Option<String>,
    pub url: Option<String>,
    #[serde(default)]
    pub event_types: Vec<String>,
    pub status: Option<String>,
    pub channel_id: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub created_at: Option<i64>,
}

/// Webhook creation response including its one-time secret.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookWithSecret {
    #[serde(flatten)]
    pub webhook: Webhook,
    pub signing_secret: Secret<String>,
}

/// A webhook delivery attempt.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct WebhookDelivery {
    pub id: Option<String>,
    pub webhook_id: Option<String>,
    pub event_id: Option<String>,
    pub event_type: Option<String>,
    pub status: Option<String>,
    pub attempt_count: Option<i64>,
    pub response_status: Option<i64>,
    pub last_attempt_at: Option<i64>,
    pub next_attempt_at: Option<i64>,
    pub created_at: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
