//! The fully-resolved request description shared by both executors.

use crate::core::operation::Operation;
use crate::error::Result;

/// A concrete HTTP request, built once from an [`Operation`] and consumed by
/// whichever executor performs the IO.
#[allow(missing_docs)]
#[derive(Debug)]
pub struct RequestSpec {
    pub method: http::Method,
    pub path: String,
    pub query: Vec<(&'static str, String)>,
    pub headers: Vec<(&'static str, String)>,
    pub body: Option<Vec<u8>>,
}

impl RequestSpec {
    /// Resolve an [`Operation`] into a concrete request.
    pub fn build<O: Operation>(op: &O) -> Result<Self> {
        Ok(RequestSpec {
            method: O::METHOD,
            path: op.path(),
            query: op.query(),
            headers: op.headers(),
            body: op.body()?,
        })
    }
}
