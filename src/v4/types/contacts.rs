use super::ChannelType;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// A v4 contact, distinct from the v2 contact shape.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Contact {
    pub id: Option<String>,
    pub name: Option<String>,
    pub created_at: Option<i64>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub last_message_time: Option<i64>,
    pub last_direction: Option<String>,
    pub last_text: Option<String>,
    pub last_status: Option<String>,
    #[serde(default)]
    pub identities: Vec<ContactSummaryIdentity>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Identity embedded in a contact response.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ContactSummaryIdentity {
    pub id: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub identifier: Option<String>,
    pub channel_id: Option<String>,
    pub created_at: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A channel-addressable identity attached to a contact.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ContactIdentity {
    pub id: Option<String>,
    pub identifier: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub channel_id: Option<String>,
    pub created_at: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Contact capability record for one identity/channel.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ContactCapability {
    pub id: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub identifier: Option<String>,
    pub channel_id: Option<String>,
    #[serde(default)]
    pub capabilities: BTreeMap<String, Value>,
}

/// Contact timeline entry.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TimelineEntry {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    pub occurred_at: Option<i64>,
    pub chat_id: Option<String>,
    pub channel_id: Option<String>,
    pub channel_type: Option<ChannelType>,
    #[serde(rename = "object", default)]
    pub object: BTreeMap<String, Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
