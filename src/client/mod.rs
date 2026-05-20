//! The two thin IO executors. Everything else in the crate is sans-IO.

#[cfg(feature = "async")]
mod asynchronous;
#[cfg(feature = "sync")]
mod blocking;

#[cfg(feature = "async")]
pub use asynchronous::Client;
#[cfg(feature = "sync")]
pub use blocking::BlockingClient;
