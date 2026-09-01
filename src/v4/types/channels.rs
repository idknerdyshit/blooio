use super::ChannelType;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use serde_json::Value;
use std::collections::BTreeMap;

/// A v4 delivery channel.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Channel {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub channel_type: Option<ChannelType>,
    pub address: Option<String>,
    pub alias: Option<String>,
    pub status: Option<String>,
    pub capabilities: Option<ChannelCapabilities>,
    pub profile: Option<ChannelProfile>,
    pub created_at: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Stored profile metadata for a channel.
///
/// `None` omits a field; `Some(None)` clears it when used in an update.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ChannelProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub about: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<Option<String>>,
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
    pub address: Option<String>,
    pub alias: Option<String>,
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

/// Blooio number inventory family.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlooioNumberType {
    /// Shared Blooio line.
    Shared,
    /// Dedicated Blooio line.
    Dedicated,
    /// Inbound-only Blooio line.
    Inbound,
    /// A future inventory family preserved for forward compatibility.
    Unknown(String),
}

impl BlooioNumberType {
    pub(crate) fn wire_value(&self) -> &str {
        match self {
            Self::Shared => "shared",
            Self::Dedicated => "dedicated",
            Self::Inbound => "inbound",
            Self::Unknown(value) => value,
        }
    }
}

impl Serialize for BlooioNumberType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.wire_value())
    }
}

impl<'de> Deserialize<'de> for BlooioNumberType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "shared" => Self::Shared,
            "dedicated" => Self::Dedicated,
            "inbound" => Self::Inbound,
            _ => Self::Unknown(value),
        })
    }
}

/// One available Blooio number inventory row or area-code quote line.
#[derive(Debug, Clone, PartialEq)]
pub enum AvailableBlooioNumber {
    /// An inventory row with a masked number.
    Inventory(AvailableBlooioInventory),
    /// An area-code quote line.
    Quote(AvailableBlooioQuote),
}

/// A masked row from the available Blooio number inventory.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct AvailableBlooioInventory {
    pub masked_national: Option<String>,
    pub area_code: Option<String>,
    pub country_code: Option<String>,
    pub phone_number_country: Option<String>,
    pub location: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// An area-code availability quote line.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct AvailableBlooioQuote {
    pub area_code: Option<String>,
    pub matched: Option<bool>,
    pub custom_order: Option<bool>,
    pub auto_assigned: Option<bool>,
    pub zip_code: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl<'de> Deserialize<'de> for AvailableBlooioNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("available number item must be an object"))?;
        let is_quote = ["matched", "custom_order", "auto_assigned", "zip_code"]
            .iter()
            .any(|key| object.contains_key(*key));

        if is_quote {
            serde_json::from_value(value)
                .map(Self::Quote)
                .map_err(D::Error::custom)
        } else {
            serde_json::from_value(value)
                .map(Self::Inventory)
                .map_err(D::Error::custom)
        }
    }
}

/// Available Blooio number inventory or area-code quote response.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AvailableBlooioNumbers {
    #[serde(default)]
    pub data: Vec<AvailableBlooioNumber>,
    pub matched_count: Option<u32>,
    pub custom_order_count: Option<u32>,
    #[serde(default)]
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

impl crate::v4::CursorListing for AvailableBlooioNumbers {
    type Item = AvailableBlooioNumber;

    fn into_cursor_page(self) -> crate::v4::CursorPage<Self::Item> {
        crate::v4::CursorPage {
            items: self.data,
            has_more: self.has_more,
            next_cursor: self.next_cursor,
        }
    }
}

/// State of an asynchronous Blooio number purchase.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlooioPurchaseStatus {
    /// Accepted but not yet provisioning.
    Pending,
    /// Provisioning is in progress.
    Provisioning,
    /// Cardholder authentication is required.
    ActionRequired,
    /// Provisioning completed.
    Completed,
    /// The purchase failed terminally.
    Failed,
    /// A future purchase state preserved for forward compatibility.
    Unknown(String),
}

impl<'de> Deserialize<'de> for BlooioPurchaseStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "pending" => Self::Pending,
            "provisioning" => Self::Provisioning,
            "action_required" => Self::ActionRequired,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            _ => Self::Unknown(value),
        })
    }
}

/// A provider-defined allocation created by a number purchase.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct BlooioNumberAllocation {
    #[serde(flatten)]
    pub fields: BTreeMap<String, Value>,
}

/// Accepted or current state of a Blooio number purchase.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct BlooioPurchase {
    pub purchase_id: Option<String>,
    pub status: Option<BlooioPurchaseStatus>,
    pub action_url: Option<String>,
    #[serde(default)]
    pub allocations: Vec<BlooioNumberAllocation>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Result of removing an owned Blooio number.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RemovedBlooioNumber {
    pub phone_number: Option<String>,
    pub channel_id: Option<String>,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
