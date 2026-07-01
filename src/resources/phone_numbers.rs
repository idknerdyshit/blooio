//! Phone numbers: lookup and batch-lookup phone number information.

use http::Method;
use serde::{Deserialize, Serialize};

use crate::core::operation::{Operation, json_body};
use crate::error::Result;
use crate::types::IntoStringList;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response of `POST /phone-numbers/batch`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct BatchLookupResponse {
    pub results: Vec<crate::types::PhoneNumberLookupResult>,
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// `GET /phone-numbers/lookup?number=...`
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct LookupPhoneNumber {
    /// Phone number to look up, sent as a query parameter.
    pub number: String,
}

impl Operation for LookupPhoneNumber {
    type Output = crate::types::PhoneNumberLookupResult;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/phone-numbers/lookup".into()
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        vec![("number", self.number.clone())]
    }
}

/// `POST /phone-numbers/lookup`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct LookupPhoneNumberPost {
    /// Phone number to look up, sent in the JSON body.
    pub number: String,
}

impl Operation for LookupPhoneNumberPost {
    type Output = crate::types::PhoneNumberLookupResult;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/phone-numbers/lookup".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

/// `POST /phone-numbers/batch`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct BatchLookupPhoneNumbers {
    /// Phone numbers to look up in one request.
    pub numbers: Vec<String>,
}

impl BatchLookupPhoneNumbers {
    /// Create a batch lookup request from a string collection of numbers.
    pub fn new(numbers: impl IntoStringList) -> Self {
        Self {
            numbers: numbers.into_string_vec(),
        }
    }
}

impl Operation for BatchLookupPhoneNumbers {
    type Output = BatchLookupResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/phone-numbers/batch".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

// ---------------------------------------------------------------------------
// Resource handle + accessors.
// ---------------------------------------------------------------------------

/// Handle for the `phone_numbers` resource group. Created via
/// [`Client::phone_numbers`](crate::Client::phone_numbers).
#[derive(Debug)]
pub struct PhoneNumbers<'c, C> {
    pub(crate) client: &'c C,
}

#[cfg(feature = "async")]
impl crate::Client {
    /// Access the `phone_numbers` resource group.
    pub fn phone_numbers(&self) -> PhoneNumbers<'_, crate::Client> {
        PhoneNumbers { client: self }
    }
}

#[cfg(feature = "sync")]
impl crate::BlockingClient {
    /// Access the `phone_numbers` resource group.
    pub fn phone_numbers(&self) -> PhoneNumbers<'_, crate::BlockingClient> {
        PhoneNumbers { client: self }
    }
}

#[cfg(feature = "async")]
impl PhoneNumbers<'_, crate::Client> {
    /// Lookup a phone number via GET query parameter.
    pub async fn lookup(
        &self,
        number: impl Into<String>,
    ) -> Result<crate::types::PhoneNumberLookupResult> {
        self.client
            .send(LookupPhoneNumber {
                number: number.into(),
            })
            .await
    }

    /// Lookup a phone number via POST body.
    pub async fn lookup_post(
        &self,
        number: impl Into<String>,
    ) -> Result<crate::types::PhoneNumberLookupResult> {
        self.client
            .send(LookupPhoneNumberPost {
                number: number.into(),
            })
            .await
    }

    /// Batch lookup multiple phone numbers.
    pub async fn batch(&self, numbers: impl IntoStringList) -> Result<BatchLookupResponse> {
        self.client
            .send(BatchLookupPhoneNumbers::new(numbers))
            .await
    }
}

#[cfg(feature = "sync")]
impl PhoneNumbers<'_, crate::BlockingClient> {
    /// Lookup a phone number via GET query parameter.
    pub fn lookup(
        &self,
        number: impl Into<String>,
    ) -> Result<crate::types::PhoneNumberLookupResult> {
        self.client.send(LookupPhoneNumber {
            number: number.into(),
        })
    }

    /// Lookup a phone number via POST body.
    pub fn lookup_post(
        &self,
        number: impl Into<String>,
    ) -> Result<crate::types::PhoneNumberLookupResult> {
        self.client.send(LookupPhoneNumberPost {
            number: number.into(),
        })
    }

    /// Batch lookup multiple phone numbers.
    pub fn batch(&self, numbers: impl IntoStringList) -> Result<BatchLookupResponse> {
        self.client.send(BatchLookupPhoneNumbers::new(numbers))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::unreadable_literal
)]
mod tests {
    use super::*;
    use crate::core::operation::Operation;

    // --- LookupPhoneNumber (GET) ---

    #[test]
    fn lookup_method_is_get() {
        assert_eq!(LookupPhoneNumber::METHOD, http::Method::GET);
    }

    #[test]
    fn lookup_path() {
        let op = LookupPhoneNumber {
            number: "+15550001111".into(),
        };
        assert_eq!(op.path(), "/phone-numbers/lookup");
    }

    #[test]
    fn lookup_query_contains_number() {
        let op = LookupPhoneNumber {
            number: "+15550001111".into(),
        };
        let q = op.query();
        assert_eq!(q.len(), 1);
        assert_eq!(q[0], ("number", "+15550001111".to_string()));
    }

    // --- LookupPhoneNumberPost (POST) ---

    #[test]
    fn lookup_post_method_is_post() {
        assert_eq!(LookupPhoneNumberPost::METHOD, http::Method::POST);
    }

    #[test]
    fn lookup_post_path() {
        let op = LookupPhoneNumberPost {
            number: "+15550001111".into(),
        };
        assert_eq!(op.path(), "/phone-numbers/lookup");
    }

    #[test]
    fn lookup_post_body_contains_number() {
        let op = LookupPhoneNumberPost {
            number: "+15550001111".into(),
        };
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v, serde_json::json!({ "number": "+15550001111" }));
    }

    // --- BatchLookupPhoneNumbers (POST /phone-numbers/batch) ---

    #[test]
    fn batch_method_is_post() {
        assert_eq!(BatchLookupPhoneNumbers::METHOD, http::Method::POST);
    }

    #[test]
    fn batch_path() {
        let op = BatchLookupPhoneNumbers {
            numbers: vec!["+15550001111".into()],
        };
        assert_eq!(op.path(), "/phone-numbers/batch");
    }

    #[test]
    fn batch_body_single_number() {
        let op = BatchLookupPhoneNumbers::new(["+15550001111"]);
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v, serde_json::json!({ "numbers": ["+15550001111"] }));
    }

    #[test]
    fn batch_body_multiple_numbers() {
        let op = BatchLookupPhoneNumbers::new(["+15550001111", "+15550002222"]);
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "numbers": ["+15550001111", "+15550002222"] })
        );
    }

    #[test]
    fn batch_body_preserves_vec_string_literal_inference() {
        let op = BatchLookupPhoneNumbers::new(vec!["+15550001111".into(), "+15550002222".into()]);
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            v,
            serde_json::json!({ "numbers": ["+15550001111", "+15550002222"] })
        );
    }

    #[test]
    fn batch_body_empty_list() {
        let op = BatchLookupPhoneNumbers { numbers: vec![] };
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v, serde_json::json!({ "numbers": [] }));
    }
}
