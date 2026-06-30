//! `FaceTime`: initiate `FaceTime` calls.

use http::Method;
use serde::{Deserialize, Serialize};

use crate::core::operation::{Operation, json_body};
use crate::error::Result;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response of `POST /facetime/calls`.
#[allow(missing_docs)]
#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct FaceTimeCallResponse {
    pub success: Option<bool>,
    pub link: Option<String>,
    pub handle: Option<String>,
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

/// `POST /facetime/calls`
#[allow(missing_docs)]
#[derive(Debug, Clone, Serialize)]
pub struct CallFaceTime {
    /// Phone number, email address, or handle to call.
    pub handle: String,
}

impl Operation for CallFaceTime {
    type Output = FaceTimeCallResponse;
    const METHOD: Method = Method::POST;
    fn path(&self) -> String {
        "/facetime/calls".into()
    }
    fn body(&self) -> Result<Option<Vec<u8>>> {
        json_body(self)
    }
}

// ---------------------------------------------------------------------------
// Resource handle + accessors.
// ---------------------------------------------------------------------------

/// Handle for the `facetime` resource group. Created via
/// [`Client::facetime`](crate::Client::facetime).
#[derive(Debug)]
pub struct FaceTime<'c, C> {
    pub(crate) client: &'c C,
}

#[cfg(feature = "async")]
impl crate::Client {
    /// Access the facetime resource group.
    pub fn facetime(&self) -> FaceTime<'_, crate::Client> {
        FaceTime { client: self }
    }
}

#[cfg(feature = "sync")]
impl crate::BlockingClient {
    /// Access the facetime resource group.
    pub fn facetime(&self) -> FaceTime<'_, crate::BlockingClient> {
        FaceTime { client: self }
    }
}

#[cfg(feature = "async")]
impl FaceTime<'_, crate::Client> {
    /// Initiate a `FaceTime` call to a handle.
    pub async fn call(&self, handle: impl Into<String>) -> Result<FaceTimeCallResponse> {
        self.client
            .send(CallFaceTime {
                handle: handle.into(),
            })
            .await
    }
}

#[cfg(feature = "sync")]
impl FaceTime<'_, crate::BlockingClient> {
    /// Initiate a `FaceTime` call to a handle.
    pub fn call(&self, handle: impl Into<String>) -> Result<FaceTimeCallResponse> {
        self.client.send(CallFaceTime {
            handle: handle.into(),
        })
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
    fn call_facetime_method_is_post() {
        assert_eq!(CallFaceTime::METHOD, http::Method::POST);
    }

    #[test]
    fn call_facetime_path() {
        let op = CallFaceTime {
            handle: "abc123".into(),
        };
        assert_eq!(op.path(), "/facetime/calls");
    }

    #[test]
    fn call_facetime_body_serializes_handle() {
        let op = CallFaceTime {
            handle: "abc123".into(),
        };
        let body = op.body().unwrap().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v, serde_json::json!({ "handle": "abc123" }));
    }
}
