use super::ChannelType;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// A v4 delivery channel.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Channel {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub channel_type: Option<ChannelType>,
    pub display_address: Option<String>,
    pub sender_key: Option<String>,
    pub status: Option<String>,
    pub capabilities: Option<ChannelCapabilities>,
    pub created_at: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A v4 routing priority.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Priority {
    pub id: Option<String>,
    pub name: Option<String>,
    pub is_default: Option<bool>,
    #[serde(default)]
    pub channels: Vec<PriorityChannel>,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// One channel assignment in a routing priority.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PriorityChannel {
    pub channel_id: Option<String>,
    #[serde(rename = "type")]
    pub channel_type: Option<ChannelType>,
    pub priority: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Channel capability information.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChannelCapabilities {
    #[serde(default)]
    pub protocols: Vec<String>,
    #[serde(default)]
    pub content: Vec<String>,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub interactive: Vec<String>,
    #[serde(default)]
    pub gates: Vec<String>,
    #[serde(flatten)]
    pub capabilities: BTreeMap<String, Value>,
}
