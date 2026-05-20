//! Sans-IO core: operation descriptions, request/response plumbing, and
//! pagination. Nothing here performs IO; the [`Client`](crate::Client) and
//! [`BlockingClient`](crate::BlockingClient) executors do.

pub mod operation;
pub mod pagination;
pub mod ratelimit;
pub mod request;
pub mod response;
pub mod retry;

pub use operation::{Operation, json_body};
pub use pagination::{DEFAULT_PAGE_SIZE, Listing, Page, Pagination, Paginator};
pub use ratelimit::{RateLimit, ResponseMeta};
pub use request::RequestSpec;
pub use retry::RetryPolicy;
