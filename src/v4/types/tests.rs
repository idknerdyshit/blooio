#![allow(clippy::panic, clippy::unwrap_used)]

use super::*;
use serde_json::json;

#[test]
fn channel_types_match_the_schema_and_preserve_unknown_values() {
    let known = [
        (ChannelType::Blooio, "blooio"),
        (ChannelType::Twilio, "twilio"),
        (ChannelType::WhatsApp, "whatsapp"),
        (ChannelType::WhatsAppBusiness, "whatsapp_business"),
        (ChannelType::RcsBusiness, "rcs_business"),
    ];

    for (channel_type, wire_value) in known {
        assert_eq!(serde_json::to_value(&channel_type).unwrap(), wire_value);
        assert_eq!(
            serde_json::from_value::<ChannelType>(json!(wire_value)).unwrap(),
            channel_type
        );
    }

    let unknown: ChannelType = serde_json::from_value(json!("future_provider")).unwrap();
    assert_eq!(unknown, ChannelType::Unknown("future_provider".into()));
    assert_eq!(serde_json::to_value(&unknown).unwrap(), "future_provider");
    assert_eq!(
        serde_json::from_value::<ChannelType>(
            serde_json::to_value(ChannelType::Unknown("blooio".into())).unwrap()
        )
        .unwrap(),
        ChannelType::Blooio
    );
    assert!(serde_json::from_value::<ChannelType>(json!(42)).is_err());
}

#[test]
fn webhook_scope_is_forward_compatible_and_api_key_is_redacted() {
    let webhook: Webhook = serde_json::from_value(json!({
        "id": "wh_1",
        "scope": "api_key",
        "api_key": "bl_live_sensitive"
    }))
    .unwrap();
    assert_eq!(webhook.scope, Some(WebhookScope::ApiKey));
    let rendered = format!("{webhook:?}");
    assert!(!rendered.contains("bl_live_sensitive"));
    assert!(rendered.contains("[REDACTED]"));

    let unknown: WebhookScope = serde_json::from_value(json!("future_scope")).unwrap();
    assert_eq!(unknown, WebhookScope::Unknown("future_scope".into()));
}

#[test]
fn message_fields_are_typed_and_unknown_fields_are_forward_compatible() {
    let message: Message = serde_json::from_value(json!({
        "id": "msg_1", "chat_id": "chat_1", "channel_id": "ch_1",
        "channel_type": "blooio", "protocol": "imessage", "direction": "outbound",
        "type": "text", "text": "hello", "status": "delivered",
        "provider_message_id": "provider_1", "reply_to_message_id": null,
        "error": {"code": "none"},
        "attachments": [{"url": "https://example.com/media", "media_type": "image/png", "size": 12, "caption": "photo"}],
        "interactive": {"role": "reply", "kind": "quick_reply", "chosen": ["Yes"]},
        "created_at": 1, "updated_at": 2,
        "future_field": true
    }))
    .unwrap();

    assert_eq!(message.protocol.as_deref(), Some("imessage"));
    assert_eq!(message.channel_type, Some(ChannelType::Blooio));
    assert_eq!(message.direction.as_deref(), Some("outbound"));
    assert_eq!(message.content_type.as_deref(), Some("text"));
    assert_eq!(message.text.as_deref(), Some("hello"));
    assert_eq!(message.created_at, Some(1));
    assert_eq!(message.updated_at, Some(2));
    assert_eq!(message.error.unwrap()["code"], "none");
    assert_eq!(message.attachments[0].size, Some(12));
    assert_eq!(message.interactive.as_ref().unwrap()["role"], "reply");
    assert_eq!(message.extra["future_field"], true);
    assert!(!message.extra.contains_key("protocol"));
    assert!(!message.extra.contains_key("created_at"));
}

#[test]
fn channel_chat_contact_group_and_priority_fields_are_typed() {
    let channel: Channel = serde_json::from_value(json!({
        "id": "ch_1", "type": "sms", "address": "+15551234567",
        "alias": "sender_1", "status": "active", "created_at": 1,
        "capabilities": {"protocols": ["sms"], "content": ["text"], "actions": ["send"], "interactive": [], "gates": []}
    }))
    .unwrap();
    assert_eq!(channel.address.as_deref(), Some("+15551234567"));
    assert_eq!(
        channel.channel_type,
        Some(ChannelType::Unknown("sms".into()))
    );
    assert_eq!(channel.alias.as_deref(), Some("sender_1"));
    assert!(!channel.extra.contains_key("address"));
    assert!(!channel.extra.contains_key("alias"));
    assert_eq!(channel.capabilities.unwrap().protocols, ["sms"]);

    let chat: Chat = serde_json::from_value(json!({
        "id": "chat_1", "channel_id": "ch_1", "channel_type": "sms", "contact_id": "ct_1",
        "identity_id": "cid_1", "group_id": null, "state": "open", "capabilities": {"read": true},
        "window_expires_at": null, "last_message_at": 3, "created_at": 2, "future": "kept"
    }))
    .unwrap();
    assert_eq!(chat.contact_id.as_deref(), Some("ct_1"));
    assert_eq!(chat.capabilities["read"], true);
    assert_eq!(chat.extra["future"], "kept");
    assert!(!chat.extra.contains_key("name"));

    let contact: Contact = serde_json::from_value(json!({
        "id": "ct_1", "name": "Ada", "created_at": 1, "tags": ["vip"],
        "last_message_time": 2, "last_direction": "inbound", "last_text": "hi", "last_status": "read",
        "identities": [{"id": "cid_1", "channel_type": "sms", "identifier": "+1555", "channel_id": null, "created_at": 3}]
    }))
    .unwrap();
    assert_eq!(contact.last_text.as_deref(), Some("hi"));
    assert_eq!(contact.identities[0].identifier.as_deref(), Some("+1555"));

    let group: Group = serde_json::from_value(json!({
        "id": "grp_1", "channel_id": "ch_1", "channel_type": "blooio", "name": null,
        "icon_url": null, "chat_id": "chat_1", "chat_linked": true, "created_at": 1,
        "members": [{"identity_id": "cid_1", "contact_id": null, "role": "member", "channel_type": "blooio", "identifier": "ada@example.test"}]
    }))
    .unwrap();
    assert!(group.chat_linked.unwrap());
    assert_eq!(group.members[0].role.as_deref(), Some("member"));

    let priority: Priority = serde_json::from_value(json!({
        "id": "priority_1", "name": null, "is_default": true, "created_at": 1, "updated_at": 2,
        "channels": [{
            "channel_id": "ch_1", "type": "sms", "address": "+15551234567",
            "alias": "sender_1", "priority": 1
        }]
    }))
    .unwrap();
    assert!(priority.is_default.unwrap());
    assert_eq!(priority.channels[0].priority, Some(1));
    assert_eq!(
        priority.channels[0].address.as_deref(),
        Some("+15551234567")
    );
    assert_eq!(priority.channels[0].alias.as_deref(), Some("sender_1"));
    assert!(!priority.channels[0].extra.contains_key("address"));
    assert!(!priority.channels[0].extra.contains_key("alias"));
}

#[test]
fn polling_routing_timeline_event_and_webhook_fields_are_typed() {
    let send: MessageSendResult = serde_json::from_value(json!({
        "id": "msg_1", "chat_id": "chat_1", "channel_id": "ch_1", "channel_type": "sms",
        "protocol": "sms", "direction": "outbound", "type": "text", "status": "queued",
        "group_id": "grp_1", "hybrid": {"phase": "one"}, "error": null,
        "fallback": {"recommended": true, "reason": "retry"}, "to": "+1555", "dry_run": false,
        "would_send": true, "preview": {"text": "hello"}, "poll": {"title": "Lunch?", "options": ["Yes", "No"]},
        "routing": {"mode": "priority", "channel_type": "sms", "number": "+1555", "alias": "key", "priority_id": "priority_1", "priority": 1}
    }))
    .unwrap();
    let MessageSendResult::Message(send) = send else {
        panic!("expected single send")
    };
    let routing = send.routing.unwrap();
    assert_eq!(routing.priority, Some(1));
    assert_eq!(routing.alias.as_deref(), Some("key"));
    assert!(!routing.extra.contains_key("alias"));
    assert_eq!(send.poll.unwrap().options, ["Yes", "No"]);

    let results: PollResults = serde_json::from_value(json!({"poll_id": "poll_1", "chat_id": "chat_1", "title": "Lunch?", "options": [{"text": "Yes", "votes": 2}], "total_votes": 2})).unwrap();
    assert_eq!(results.options[0].votes, Some(2));
    let vote: PollVoteResult = serde_json::from_value(json!({"id": "vote_1", "poll_id": "poll_1", "voted_option": "Yes", "toggled": "on", "active_votes": ["Yes"], "active_vote_indices": [0]})).unwrap();
    assert_eq!(vote.active_vote_indices, [0]);

    let timeline: TimelineEntry = serde_json::from_value(json!({"type": "message", "occurred_at": 1, "id": "msg_1", "chat_id": "chat_1", "channel_id": "ch_1", "channel_type": "sms", "object": {"text": "hello"}})).unwrap();
    assert_eq!(timeline.object["text"], "hello");
    let event: Event = serde_json::from_value(json!({"id": "evt_1", "type": "message.delivered", "message_id": "msg_1", "chat_id": null, "channel_type": "sms", "occurred_at": 2, "data": {"receipt": true}})).unwrap();
    assert_eq!(event.data["receipt"], true);
    let delivery: WebhookDelivery = serde_json::from_value(json!({"id": "wdel_1", "webhook_id": "wh_1", "event_id": "evt_1", "event_type": "message.delivered", "status": "delivered", "attempt_count": 1, "response_status": 200, "last_attempt_at": 2, "next_attempt_at": null, "created_at": 1})).unwrap();
    assert_eq!(delivery.response_status, Some(200));
}

#[test]
fn message_timestamp_and_channel_naming_regressions_are_enforced() {
    let message: Message =
        serde_json::from_value(json!({"created_at": 1, "updated_at": 2})).unwrap();
    assert_eq!(message.created_at, Some(1));
    assert_eq!(message.updated_at, Some(2));
    assert!(!message.extra.contains_key("created_at"));

    let channel: Channel =
        serde_json::from_value(json!({"address": "+1555", "alias": "key"})).unwrap();
    assert_eq!(channel.address.as_deref(), Some("+1555"));
    assert_eq!(channel.alias.as_deref(), Some("key"));
    assert!(!channel.extra.contains_key("address"));
    assert!(!channel.extra.contains_key("alias"));
}
