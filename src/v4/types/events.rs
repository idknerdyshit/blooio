use super::ChannelType;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// A unified v4 event.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Event {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub message_id: Option<String>,
    pub chat_id: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub occurred_at: Option<i64>,
    #[serde(default)]
    pub data: BTreeMap<String, Value>,
}
