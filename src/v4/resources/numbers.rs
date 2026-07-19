//! V4 account sender-number operations.
#![allow(missing_docs)]

use super::impl_v4_operation;
use crate::v4::types::{ActionResponse, ItemEnvelope, ListEnvelope, SenderNumber};
use crate::{
    Result,
    core::operation::{encode_path_segment, json_body},
};
use http::Method;
use serde::Serialize;

/// List sender numbers.
#[derive(Debug, Clone, Copy)]
pub struct ListNumbers;
impl crate::Operation for ListNumbers {
    type Output = ListEnvelope<SenderNumber>;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/me/numbers".into()
    }
}
impl_v4_operation!(ListNumbers);

#[derive(Debug, Serialize)]
struct CallForwardingBody {
    forward_to: String,
}

/// Request call forwarding for a sender number.
#[derive(Debug, Clone)]
pub struct RequestCallForwarding {
    number: String,
    forward_to: String,
}
impl RequestCallForwarding {
    /// Construct the request.
    #[must_use]
    pub fn new(number: impl Into<String>, forward_to: impl Into<String>) -> Self {
        Self {
            number: number.into(),
            forward_to: forward_to.into(),
        }
    }
}
impl crate::Operation for RequestCallForwarding {
    type Output = ItemEnvelope<crate::v4::types::ActionResult>;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!(
            "/me/numbers/{}/call-forwarding",
            encode_path_segment(&self.number)
        )
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(&CallForwardingBody {
            forward_to: self.forward_to.clone(),
        })
    }
}
impl_v4_operation!(RequestCallForwarding);

/// Sender-number resource handle.
#[derive(Debug)]
pub struct Numbers<C> {
    pub(crate) client: C,
}
#[cfg(feature = "async")]
impl crate::v4::BlooioAccount<'_> {
    /// Access sender numbers.
    #[must_use]
    pub fn numbers(self) -> Numbers<Self> {
        Numbers { client: self }
    }
}
#[cfg(feature = "sync")]
impl crate::v4::BlockingBlooioAccount<'_> {
    /// Access sender numbers.
    #[must_use]
    pub fn numbers(self) -> Numbers<Self> {
        Numbers { client: self }
    }
}
#[cfg(feature = "async")]
impl Numbers<crate::v4::BlooioAccount<'_>> {
    /// List sender numbers.
    pub async fn list(&self) -> Result<ListEnvelope<SenderNumber>> {
        self.client.send(ListNumbers).await
    }
    /// Request call forwarding.
    pub async fn request_call_forwarding(
        &self,
        number: impl Into<String>,
        forward_to: impl Into<String>,
    ) -> Result<ActionResponse> {
        self.client
            .send(RequestCallForwarding::new(number, forward_to))
            .await
    }
}
#[cfg(feature = "sync")]
impl Numbers<crate::v4::BlockingBlooioAccount<'_>> {
    /// List sender numbers.
    pub fn list(&self) -> Result<ListEnvelope<SenderNumber>> {
        self.client.send(ListNumbers)
    }
    /// Request call forwarding.
    pub fn request_call_forwarding(
        &self,
        number: impl Into<String>,
        forward_to: impl Into<String>,
    ) -> Result<ActionResponse> {
        self.client
            .send(RequestCallForwarding::new(number, forward_to))
    }
}
