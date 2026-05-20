//! Phone numbers: lookup and batch-lookup phone number information.

use http::Method;
use serde::{Deserialize, Serialize};

use crate::core::operation::{Operation, json_body};
use crate::error::Result;

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
    pub numbers: Vec<String>,
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
    pub async fn batch(&self, numbers: Vec<String>) -> Result<BatchLookupResponse> {
        self.client.send(BatchLookupPhoneNumbers { numbers }).await
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
    pub fn batch(&self, numbers: Vec<String>) -> Result<BatchLookupResponse> {
        self.client.send(BatchLookupPhoneNumbers { numbers })
    }
}
