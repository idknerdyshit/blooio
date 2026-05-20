//! Sans-IO core: operation descriptions, request/response plumbing, and
//! pagination. Nothing here performs IO; the [`Client`](crate::Client) and
//! [`BlockingClient`](crate::BlockingClient) executors do.

pub mod operation;
pub mod pagination;
pub mod request;
pub mod response;

pub use operation::{json_body, Operation};
pub use pagination::{Page, Paginator, Pagination, Listing, DEFAULT_PAGE_SIZE};
pub use request::RequestSpec;
