use super::ChannelType;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// A v4 chat.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Chat {
    pub id: Option<String>,
    pub channel_id: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub contact_id: Option<String>,
    pub identity_id: Option<String>,
    pub group_id: Option<String>,
    pub state: Option<String>,
    #[serde(default)]
    pub capabilities: BTreeMap<String, Value>,
    pub window_expires_at: Option<i64>,
    pub last_message_at: Option<i64>,
    pub created_at: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Result returned when a chat is found or created.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChatCreated {
    pub id: Option<String>,
    pub channel_id: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub contact_id: Option<String>,
    pub identity_id: Option<String>,
    pub state: Option<String>,
    pub created: Option<bool>,
}

/// Poll content returned by the poll-creation endpoint.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PollContent {
    pub title: Option<String>,
    #[serde(default)]
    pub options: Vec<String>,
}

/// Poll returned after creation.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Poll {
    pub id: Option<String>,
    pub chat_id: Option<String>,
    pub channel_id: Option<String>,
    pub channel_type: Option<ChannelType>,
    #[serde(rename = "type")]
    pub poll_type: Option<String>,
    pub status: Option<String>,
    pub poll: Option<PollContent>,
}

/// Results for a v4 poll.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PollResults {
    pub poll_id: Option<String>,
    pub chat_id: Option<String>,
    pub title: Option<String>,
    #[serde(default)]
    pub options: Vec<PollResultOption>,
    pub total_votes: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Vote total for one poll option.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PollResultOption {
    pub text: Option<String>,
    pub votes: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Result returned after changing a poll vote.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PollVoteResult {
    pub id: Option<String>,
    pub poll_id: Option<String>,
    pub voted_option: Option<String>,
    pub toggled: Option<String>,
    #[serde(default)]
    pub active_votes: Vec<String>,
    #[serde(default)]
    pub active_vote_indices: Vec<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Response after setting a chat typing state.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct TypingResult {
    pub chat_id: Option<String>,
    pub state: Option<String>,
    pub at: Option<i64>,
}

/// Response after marking a chat as read.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MarkReadResult {
    pub chat_id: Option<String>,
    pub read: Option<bool>,
    pub at: Option<i64>,
}

/// Response after sharing a contact card in a chat.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ContactCardShareResult {
    pub chat_id: Option<String>,
    pub shared: Option<bool>,
    pub at: Option<i64>,
}
