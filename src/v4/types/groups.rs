use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// A v4 group.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Group {
    pub id: Option<String>,
    pub channel_id: Option<String>,
    pub channel_type: Option<super::ChannelType>,
    pub name: Option<String>,
    pub icon_url: Option<String>,
    pub chat_id: Option<String>,
    pub chat_linked: Option<bool>,
    pub created_at: Option<i64>,
    #[serde(default)]
    pub members: Vec<GroupMember>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A member returned by the v4 group-members endpoint.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GroupMember {
    pub identity_id: Option<String>,
    pub contact_id: Option<String>,
    pub role: Option<String>,
    pub channel_type: Option<super::ChannelType>,
    pub identifier: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
