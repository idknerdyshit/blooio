use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// A v4 messaging channel type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChannelType {
    /// Blooio/iMessage channel.
    Blooio,
    /// Twilio channel.
    Twilio,
    /// `WhatsApp` channel.
    WhatsApp,
    /// `WhatsApp Business` channel.
    WhatsAppBusiness,
    /// RCS Business Messaging channel.
    RcsBusiness,
    /// An unrecognized provider value preserved for forward compatibility.
    ///
    /// Canonical values are serialized unchanged, but deserialize to their
    /// corresponding known variant rather than back to `Unknown`.
    Unknown(String),
}

impl ChannelType {
    pub(crate) fn wire_value(&self) -> &str {
        match self {
            Self::Blooio => "blooio",
            Self::Twilio => "twilio",
            Self::WhatsApp => "whatsapp",
            Self::WhatsAppBusiness => "whatsapp_business",
            Self::RcsBusiness => "rcs_business",
            Self::Unknown(value) => value,
        }
    }
}

impl Serialize for ChannelType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.wire_value())
    }
}

impl<'de> Deserialize<'de> for ChannelType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "blooio" => Self::Blooio,
            "twilio" => Self::Twilio,
            "whatsapp" => Self::WhatsApp,
            "whatsapp_business" => Self::WhatsAppBusiness,
            "rcs_business" => Self::RcsBusiness,
            _ => Self::Unknown(value),
        })
    }
}

/// Item response envelope used by v4.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
pub struct ItemEnvelope<T> {
    pub data: T,
}

/// Offset metadata returned by v4 contact search mode.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct OffsetPagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub total: Option<i64>,
}

/// Cursor-list response envelope used by v4.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
pub struct ListEnvelope<T> {
    #[serde(default)]
    pub data: Vec<T>,
    #[serde(default)]
    pub has_more: bool,
    pub next_cursor: Option<String>,
    pub pagination: Option<OffsetPagination>,
}

impl<T> crate::v4::CursorListing for ListEnvelope<T> {
    type Item = T;
    fn into_cursor_page(self) -> crate::v4::CursorPage<T> {
        crate::v4::CursorPage {
            items: self.data,
            has_more: self.has_more,
            next_cursor: self.next_cursor,
        }
    }
}

#[cfg(any(feature = "async", feature = "sync"))]
impl<T> crate::Listing for ListEnvelope<T> {
    type Item = T;
    fn into_page(self) -> crate::Page<T> {
        crate::Page {
            items: self.data,
            pagination: self.pagination.map(|p| crate::Pagination {
                limit: p.limit,
                offset: p.offset,
                total: p.total,
                ..Default::default()
            }),
        }
    }
}

/// Generic v4 mutation result whose endpoint owns additional fields.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ActionResult {
    pub id: Option<String>,
    pub success: Option<bool>,
    pub deleted: Option<bool>,
    pub removed: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Item envelope containing a generic mutation result.
pub type ActionResponse = ItemEnvelope<ActionResult>;
