//! Opt-in bindings for the Blooio v4 API.
//!
//! Enable the `api-v4` Cargo feature to use this namespace. The root crate API
//! remains the v2 surface for compatibility.

#[cfg(any(feature = "async", feature = "sync"))]
mod client;
mod pagination;
#[cfg(any(feature = "async", feature = "sync"))]
pub mod resources;
pub mod types;
#[cfg(feature = "webhooks")]
pub mod webhook;

/// Production base URL for Blooio API v4.
pub const DEFAULT_BASE_URL: &str = "https://api.blooio.com/v4";

/// Marks a sans-IO operation as belonging to API v4.
///
/// Implement this in addition to [`crate::Operation`] for custom v4
/// operations. V4 account handles accept only operations carrying this marker.
#[cfg(any(feature = "async", feature = "sync"))]
pub trait Operation: crate::Operation {}

#[cfg(feature = "sync")]
pub use client::{BlockingBlooioAccount, BlockingClient};
#[cfg(feature = "async")]
pub use client::{BlooioAccount, Client};
#[cfg(any(feature = "async", feature = "sync"))]
pub use pagination::CursorPaginator;
pub use pagination::{CursorListing, CursorPage};
