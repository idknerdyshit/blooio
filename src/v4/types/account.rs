use crate::Secret;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// Authenticated v4 account information.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Account {
    pub id: Option<String>,
    pub organization_id: Option<String>,
    pub name: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A sender number available to the account.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SenderNumber {
    pub number: Option<String>,
    pub phone_number: Option<String>,
    pub active: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A number's contact card.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ContactCard {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub avatar_url: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Webhook or account secret returned once by the service.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
pub struct SecretResult {
    pub id: Option<String>,
    pub signing_secret: Secret<String>,
}
