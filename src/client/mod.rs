//! The two thin IO executors. Everything else in the crate is sans-IO.

#[cfg(feature = "async")]
mod asynchronous;
#[cfg(feature = "sync")]
mod blocking;
#[cfg(feature = "sensitive-diagnostics")]
mod sensitive;
#[cfg(feature = "tracing")]
mod trace;

use crate::core::options::RequestOptions;
use crate::core::request::RequestSpec;
use crate::credentials::BlooioCreds;

#[derive(Clone, Copy)]
pub(crate) struct AttemptContext<'a> {
    pub(crate) creds: &'a BlooioCreds,
    pub(crate) spec: &'a RequestSpec,
    pub(crate) url: &'a str,
    pub(crate) options: &'a RequestOptions,
    pub(crate) operation_type: &'static str,
    pub(crate) attempt: u32,
    pub(crate) max_retries: u32,
}

#[cfg(feature = "async")]
pub use asynchronous::{BlooioAccount, Client};
#[cfg(feature = "sync")]
pub use blocking::{BlockingBlooioAccount, BlockingClient};
