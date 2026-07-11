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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout
)]
mod tests {
    #[test]
    fn omitted_response_collections_default_to_empty() {
        let contacts: super::contacts::ListContactsResponse = serde_json::from_str("{}").unwrap();
        let tags: super::contacts::ContactTagsResponse = serde_json::from_str("{}").unwrap();
        let groups: super::groups::ListGroupsResponse = serde_json::from_str("{}").unwrap();
        let members: super::groups::ListGroupMembersResponse = serde_json::from_str("{}").unwrap();
        let chats: super::chats::ListChatsResponse = serde_json::from_str("{}").unwrap();
        let messages: super::chats::ListChatMessagesResponse = serde_json::from_str("{}").unwrap();
        let webhooks: super::webhooks::ListWebhooksResponse = serde_json::from_str("{}").unwrap();
        let logs: super::webhooks::ListWebhookLogsResponse = serde_json::from_str("{}").unwrap();
        let locations: super::location::LocationContactsResponse =
            serde_json::from_str("{}").unwrap();
        let lookups: super::phone_numbers::BatchLookupResponse =
            serde_json::from_str("{}").unwrap();
        let numbers: super::numbers::ListNumbersResponse = serde_json::from_str("{}").unwrap();

        assert!(contacts.contacts.is_empty());
        assert!(tags.tags.is_empty());
        assert!(groups.groups.is_empty());
        assert!(members.members.is_empty());
        assert!(chats.chats.is_empty());
        assert!(messages.messages.is_empty());
        assert!(webhooks.webhooks.is_empty());
        assert!(logs.logs.is_empty());
        assert!(locations.friends.is_empty());
        assert!(lookups.results.is_empty());
        assert!(numbers.numbers.is_empty());
    }
}
