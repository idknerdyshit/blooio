//! Hand-written v4 operation and resource namespaces.

pub mod account;
pub mod channels;
pub mod chats;
pub mod contact_card;
pub mod contacts;
pub mod events;
pub mod groups;
pub mod location;
pub mod messages;
pub mod numbers;
pub mod phone_numbers;
pub mod priorities;
pub mod webhooks;

macro_rules! impl_v4_operation {
    ($type:ty) => {
        impl crate::v4::Operation for $type {}
    };
}
pub(crate) use impl_v4_operation;
