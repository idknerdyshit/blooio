use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// A Find My location returned by v4.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LocationContact {
    pub handle: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub updated_at: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
