//! Hand-written resource namespaces. Each module defines its public
//! [`Operation`](crate::Operation) types plus a resource handle whose methods
//! delegate to the client's `send`.

pub mod account;
pub mod chats;
pub mod contact_card;
pub mod contacts;
pub mod facetime;
pub mod groups;
pub mod location;
pub mod numbers;
pub mod phone_numbers;
pub mod webhooks;
