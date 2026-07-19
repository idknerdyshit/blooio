use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// A v4 phone-number lookup result.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PhoneNumberLookup {
    pub number: Option<String>,
    pub valid: Option<bool>,
    pub formatted: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
