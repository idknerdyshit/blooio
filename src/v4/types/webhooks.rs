use super::ChannelType;
use crate::Secret;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::BTreeMap;

/// A v4 webhook subscription.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Webhook {
    pub id: Option<String>,
    pub url: Option<String>,
    /// Current event list; new subscriptions receive `["*"]`, while legacy filters are frozen.
    #[serde(default)]
    pub event_types: Vec<String>,
    pub status: Option<String>,
    pub scope: Option<WebhookScope>,
    pub api_key: Option<Secret<String>>,
    pub integration_id: Option<String>,
    pub channel_id: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub created_at: Option<i64>,
}

/// Ownership scope for a webhook subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WebhookScope {
    /// Events from every line in the organization.
    Organization,
    /// Events from lines owned by one API key.
    ApiKey,
    /// Events from lines owned by one integration.
    Integration,
    /// A future scope preserved for forward compatibility.
    Unknown(String),
}

impl<'de> Deserialize<'de> for WebhookScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "organization" => Self::Organization,
            "api_key" => Self::ApiKey,
            "integration" => Self::Integration,
            _ => Self::Unknown(value),
        })
    }
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
