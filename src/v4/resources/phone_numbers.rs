//! V4 phone-number lookup operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::v4::types::{ItemEnvelope, ListEnvelope, PhoneNumberLookup};
use crate::{Result, core::operation::json_body};
use http::Method;
use serde::Serialize;

/// Look up a phone number with GET.
#[derive(Debug, Clone)]
pub struct LookupPhoneNumber {
    pub number: String,
}
impl crate::Operation for LookupPhoneNumber {
    type Output = ItemEnvelope<PhoneNumberLookup>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/phone-numbers/lookup".into()
    }
    fn query(&self) -> Vec<(&'static str, String)> {
        vec![("number", self.number.clone())]
    }
}
impl_v4_operation!(LookupPhoneNumber);
/// Look up a phone number with POST.
#[derive(Debug, Clone, Serialize)]
pub struct LookupPhoneNumberPost {
    pub number: String,
}
impl crate::Operation for LookupPhoneNumberPost {
    type Output = ItemEnvelope<PhoneNumberLookup>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/phone-numbers/lookup".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(LookupPhoneNumberPost);
/// Batch phone-number lookup.
#[derive(Debug, Clone, Serialize)]
pub struct BatchLookupPhoneNumbers {
    pub numbers: Vec<String>,
}
impl BatchLookupPhoneNumbers {
    /// Construct a batch.
    #[must_use]
    pub fn new(numbers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            numbers: numbers.into_iter().map(Into::into).collect(),
        }
    }
}
impl crate::Operation for BatchLookupPhoneNumbers {
    type Output = ListEnvelope<PhoneNumberLookup>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/phone-numbers/batch".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}
impl_v4_operation!(BatchLookupPhoneNumbers);

/// Phone-number lookup handle.
#[derive(Debug)]
pub struct PhoneNumbers<C> {
    pub(crate) client: C,
}
#[cfg(feature = "async")]
impl crate::v4::BlooioAccount<'_> {
    /// Access phone-number lookup.
    #[must_use]
    pub fn phone_numbers(self) -> PhoneNumbers<Self> {
        PhoneNumbers { client: self }
    }
}
#[cfg(feature = "sync")]
impl crate::v4::BlockingBlooioAccount<'_> {
    /// Access phone-number lookup.
    #[must_use]
    pub fn phone_numbers(self) -> PhoneNumbers<Self> {
        PhoneNumbers { client: self }
    }
}
#[cfg(feature = "async")]
impl PhoneNumbers<crate::v4::BlooioAccount<'_>> {
    /// Look up a number with GET.
    pub async fn lookup(
        &self,
        number: impl Into<String>,
    ) -> Result<ItemEnvelope<PhoneNumberLookup>> {
        self.client
            .send(LookupPhoneNumber {
                number: number.into(),
            })
            .await
    }
    /// Look up a number with POST.
    pub async fn lookup_post(
        &self,
        number: impl Into<String>,
    ) -> Result<ItemEnvelope<PhoneNumberLookup>> {
        self.client
            .send(LookupPhoneNumberPost {
                number: number.into(),
            })
            .await
    }
    /// Batch lookup.
    pub async fn batch(
        &self,
        operation: BatchLookupPhoneNumbers,
    ) -> Result<ListEnvelope<PhoneNumberLookup>> {
        self.client.send(operation).await
    }
}
#[cfg(feature = "sync")]
impl PhoneNumbers<crate::v4::BlockingBlooioAccount<'_>> {
    /// Look up a number with GET.
    pub fn lookup(&self, number: impl Into<String>) -> Result<ItemEnvelope<PhoneNumberLookup>> {
        self.client.send(LookupPhoneNumber {
            number: number.into(),
        })
    }
    /// Look up a number with POST.
    pub fn lookup_post(
        &self,
        number: impl Into<String>,
    ) -> Result<ItemEnvelope<PhoneNumberLookup>> {
        self.client.send(LookupPhoneNumberPost {
            number: number.into(),
        })
    }
    /// Batch lookup.
    pub fn batch(
        &self,
        operation: BatchLookupPhoneNumbers,
    ) -> Result<ListEnvelope<PhoneNumberLookup>> {
        self.client.send(operation)
    }
}
