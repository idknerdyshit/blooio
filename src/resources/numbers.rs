//! Numbers: list phone numbers attached to the account and manage number
//! requests.

use http::Method;
use serde::{Deserialize, Serialize};

use crate::core::operation::{Operation, encode_path_segment, json_body};
use crate::error::Result;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// A single phone number entry.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct NumberInfo {
    pub phone_number: Option<String>,
    pub is_active: Option<bool>,
    pub last_active: Option<String>,
    pub plan_kind: Option<String>,
}

/// Response of `GET /me/numbers`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ListNumbersResponse {
    pub numbers: Vec<NumberInfo>,
}

/// Response of `POST /me/numbers/{number}/call-forwarding`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct RequestCallForwardingResponse {
    pub success: Option<bool>,
    pub ticket_id: Option<String>,
    pub status: Option<String>,
    pub number: Option<String>,
    pub forward_to: Option<String>,
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// `GET /me/numbers`
#[derive(Debug, Clone, Default)]
pub struct ListNumbers;

impl Operation for ListNumbers {
    type Output = ListNumbersResponse;
    const METHOD: Method = Method::GET;
    fn path(&self) -> String {
        "/me/numbers".into()
    }
}

/// `POST /me/numbers/{number}/call-forwarding`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct RequestCallForwarding {
    #[serde(skip)]
    /// Account phone number whose calls should be forwarded.
    pub number: String,
    /// Destination phone number to forward calls to.
    pub forward_to: String,
}

impl RequestCallForwarding {
    /// Create a call-forwarding support request.
    pub fn new(number: impl Into<String>, forward_to: impl Into<String>) -> Self {
        Self {
            number: number.into(),
            forward_to: forward_to.into(),
        }
    }
}

impl Operation for RequestCallForwarding {
    type Output = RequestCallForwardingResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        format!(
            "/me/numbers/{}/call-forwarding",
            encode_path_segment(&self.number)
        )
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

// ---------------------------------------------------------------------------
// Resource handle + accessors.
// ---------------------------------------------------------------------------

/// Handle for the `numbers` resource group. Created via
/// [`Client::numbers`](crate::Client::numbers).
#[derive(Debug)]
pub struct Numbers<'c, C> {
    pub(crate) client: &'c C,
}

#[cfg(feature = "async")]
impl crate::Client {
    /// Access the numbers resource group.
    pub fn numbers(&self) -> Numbers<'_, crate::Client> {
        Numbers { client: self }
    }
}

#[cfg(feature = "sync")]
impl crate::BlockingClient {
    /// Access the numbers resource group.
    pub fn numbers(&self) -> Numbers<'_, crate::BlockingClient> {
        Numbers { client: self }
    }
}

#[cfg(feature = "async")]
impl Numbers<'_, crate::Client> {
    /// List all phone numbers on the account.
    pub async fn list(&self) -> Result<ListNumbersResponse> {
        self.client.send(ListNumbers).await
    }

    /// Request that calls to `number` be forwarded to `forward_to`.
    pub async fn request_call_forwarding(
        &self,
        number: impl Into<String>,
        forward_to: impl Into<String>,
    ) -> Result<RequestCallForwardingResponse> {
        self.client
            .send(RequestCallForwarding::new(number, forward_to))
            .await
    }

    /// Send a fully-built call-forwarding request operation.
    pub async fn request_call_forwarding_with(
        &self,
        op: RequestCallForwarding,
    ) -> Result<RequestCallForwardingResponse> {
        self.client.send(op).await
    }
}

#[cfg(feature = "sync")]
impl Numbers<'_, crate::BlockingClient> {
    /// List all phone numbers on the account.
    pub fn list(&self) -> Result<ListNumbersResponse> {
        self.client.send(ListNumbers)
    }

    /// Request that calls to `number` be forwarded to `forward_to`.
    pub fn request_call_forwarding(
        &self,
        number: impl Into<String>,
        forward_to: impl Into<String>,
    ) -> Result<RequestCallForwardingResponse> {
        self.client
            .send(RequestCallForwarding::new(number, forward_to))
    }

    /// Send a fully-built call-forwarding request operation.
    pub fn request_call_forwarding_with(
        &self,
        op: RequestCallForwarding,
    ) -> Result<RequestCallForwardingResponse> {
        self.client.send(op)
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

    #[test]
    fn list_numbers_method_is_get() {
        assert_eq!(ListNumbers::METHOD, http::Method::GET);
    }

    #[test]
    fn list_numbers_path() {
        assert_eq!(ListNumbers.path(), "/me/numbers");
    }

    #[test]
    fn request_call_forwarding_method_is_post() {
        assert_eq!(RequestCallForwarding::METHOD, http::Method::POST);
    }

    #[test]
    fn request_call_forwarding_path_encodes_number() {
        let op = RequestCallForwarding::new("+15551234567", "+15559876543");
        assert_eq!(op.path(), "/me/numbers/%2B15551234567/call-forwarding");
    }

    #[test]
    fn request_call_forwarding_body_uses_forward_to() {
        let op = RequestCallForwarding::new("+15551234567", "+15559876543");
        let body = op.body().unwrap().unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
            serde_json::json!({ "forward_to": "+15559876543" })
        );
    }
}
