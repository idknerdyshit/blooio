# Blooio v4 API Sync — Update Plan

## Overview

Update the `blooio` Rust crate's v4 surface (`src/v4/`) to match the remote OpenAPI spec at `https://api.blooio.com/v4/openapi.json`.

**Remote spec version:** 4.0.0-beta  
**Total paths in spec:** 59  
**Currently implemented:** ~48 of 59 paths  
**Schema delta:** 48 local → 59 remote (11 new/updated schemas)

**Breaking change:** The message content model shifts from a tagged enum (`MessageContent` with `#[serde(tag = "type")]`) to a flat struct (`MessageContentFields`) matching the server's untagged request body format.

## Decision Log

| # | Decision | Choice |
|---|----------|--------|
| 1 | Message content ergonomics | **A** — Flat `MessageContentFields` struct with simple constructors |
| 2 | `SenderSelector` enum | **Removed** entirely |
| 3 | Planned endpoints (templates, attachments, chat participants) | **Stubbed** now |
| 4 | Empty schema types (`Template`, `Attachment`) | **Added** as `#[serde(flatten)]` minimal structs |

---

## Phase 1: Message Content Model Rewrite (breaking) ✅ **COMPLETE**

> All steps 1.1–1.6 implemented and verified. `cargo fmt`, `cargo clippy` (both feature sets), and `cargo test` (all 3 feature combinations) pass clean.

### Step 1.1 — Replace `MessageContent` with `MessageContentFields` ✅

**File: `src/v4/types/messages.rs`**

Deleted the tagged enum `MessageContent` and its variants. Replaced with a flat struct:

```rust
#[derive(Debug, Clone, Default, Serialize)]
pub struct MessageContentFields {
    pub text: Option<String>,
    pub attachments: Option<Vec<String>>,
    pub parts: Option<Vec<MultipartPart>>,
    pub rich_link: Option<RichLinkFields>,
    pub poll: Option<super::PollContent>,
    pub interactive: Option<InteractiveFields>,
    pub template: Option<TemplateFields>,
    pub reply_to: Option<String>,
    pub effect: Option<String>,
    pub link_preview: Option<LinkPreview>,
}
```

Added nested types:

```rust
#[derive(Debug, Clone, Default, Serialize)]
pub struct RichLinkFields {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct InteractiveFields {
    pub kind: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TemplateFields {
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
```

Keep existing `MultipartPart` and `LinkPreview` as they are (nested types, not top-level content).

Added ergonomic constructors:

```rust
impl MessageContentFields {
    pub fn text(value: impl Into<String>) -> Self { ... }
    pub fn media(attachments: impl IntoIterator<Item = impl Into<String>>) -> Self { ... }
    pub fn rich_link(url: impl Into<String>) -> Self { ... }
    pub fn poll(title: impl Into<String>, options: impl IntoIterator<Item = impl Into<String>>) -> Self { ... }
    pub fn interactive(kind: impl Into<String>) -> Self { ... }
}
```

**Wire format change:**
| Before | After |
|--------|-------|
| `{"type": "text", "text": "hello"}` | `{"text": "hello"}` |
| `{"type": "media", "attachments": ["url"]}` | `{"attachments": ["url"]}` |
| `{"type": "rich_link", "url": "..."}` | `{"rich_link": {"url": "..."}}` |
| `{"type": "poll", "title": "...", "options": [...]}` | `{"poll": {"title": "...", "options": [...]}}` |

> **Note:** `PollContent` in `MessageContentFields` uses `super::PollContent` from `chats.rs` (existing response type, now also derives `Serialize`) to avoid glob re-export ambiguity. The plan's standalone `PollContent` struct was not created as a duplicate.

### Step 1.2 — Add `Hybrid` enum ✅

**File: `src/v4/types/messages.rs`**

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Hybrid {
    On(bool),
    Number(String),
}
```

Wire: `true` / `false` or a phone number string like `"+18582849901"`.

### Step 1.3 — Rewrite `SendMessage` ✅

**File: `src/v4/resources/messages.rs`**

Replaced the current `SendMessage` struct that used `SenderSelector`:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct SendMessage {
    pub from: Option<String>,
    pub to: Recipient,
    pub priority_id: Option<String>,
    pub channel_type: Option<ChannelType>,
    pub hybrid: Option<Hybrid>,
    pub content: MessageContentFields,
    pub dry_run: Option<bool>,
}
```

Wire format matches spec's `SendMessageRequest` = `MessageContentFields` + routing fields.

Removed `SenderSelector` entirely from `src/v4/types/messages.rs`.

Added unit tests in `src/v4/resources/messages.rs` for `MessageContentFields` constructors and `SendMessage` serialization.

### Step 1.4 — Update `SendMessageToChat` ✅

**File: `src/v4/resources/messages.rs`**

```rust
pub content: MessageContentFields,  // was: MessageContent
```

### Step 1.5 — Update `SendMessageToChannel` ✅

**File: `src/v4/resources/channels.rs`**

Same change: `content: MessageContent` → `content: MessageContentFields`.

### Step 1.6 — Update tests ✅

**Files updated:**
- `tests/v4_integration_async.rs` — updated all `body_json` matchers to flat format (no `type` field), changed `MessageContent::text(...)` → `MessageContentFields::text(...)`
- `tests/v4_integration_sync.rs` — no changes needed (no `MessageContent` usage)
- `src/v4/types/tests.rs` — no changes needed (tests `Message` response DTO, unchanged)
- `src/v4/resources/chats.rs` mod `tests` — no changes needed (tests `SendPoll`, unchanged)
- `src/v4/resources/messages.rs` — added 5 unit tests for `MessageContentFields` constructors and `SendMessage` serialization
- `examples/v4_quickstart.rs` — updated import and usage to `MessageContentFields`

---

## Phase 2: New Fields on Existing Types (additive, safe)

### Step 2.1 — Add `Direction` enum

**File: `src/v4/types/messages.rs`**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Direction { Inbound, Outbound, Unknown(String) }
```

Serialize: `"inbound"`, `"outbound"`. Deserialize: fall through to `Unknown`.

Add to:
- `Message.direction: Option<Direction>`
- `MessageSendDetails.direction: Option<Direction>`

### Step 2.2 — Add `ChannelStatus` enum

**File: `src/v4/types/channels.rs`**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelStatus {
    PendingVerification, Active, Suspended, Banned, Deprecated, Unknown(String)
}
```

Wire values: `"pending_verification"`, `"active"`, `"suspended"`, `"banned"`, `"deprecated"`.

Changes to `Channel`:
- Add: `status: Option<ChannelStatus>`
- Remove: `sender_key: Option<String>` (not in spec)
- Keep: all existing fields (`id`, `channel_type`, `display_address`, `capabilities`, `created_at`)

### Step 2.3 — Add `ChatState` enum

**File: `src/v4/types/chats.rs`**

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatState { Open, Closed, Blocked, Deprecated, Unknown(String) }
```

Wire values: `"open"`, `"closed"`, `"blocked"`, `"deprecated"`.

Changes:
- `Chat.state: Option<String>` → `Option<ChatState>`
- `ChatCreated.state: Option<String>` → `Option<ChatState>`

### Step 2.4 — Add `alias` to `RoutingMetadata`

**File: `src/v4/types/messages.rs`**

Add `alias: Option<String>` to `RoutingMetadata`.

### Step 2.5 — Add `from` to `MessageSendDetails`

**File: `src/v4/types/messages.rs`**

Add `from: Option<String>` — the resolved sender (phone number, alias, or channel id).

### Step 2.6 — Add `contact_id` to `ContactIdentity`

**File: `src/v4/types/contacts.rs`**

Add `contact_id: Option<String>`.

### Step 2.7 — Add `address` and `alias` to `PriorityChannel`

**File: `src/v4/types/channels.rs`**

Add:
- `address: Option<String>` — the channel's real-world address
- `alias: Option<String>` — organization-wide alias

---

## Phase 3: Missing Operations (stubs)

All operations below are structurally correct per the spec but return 501 on the server (`x-blooio-status: planned`). Stubbed for future use.

### Step 3.1 — Chat Participants

**File: `src/v4/resources/chats.rs`**

Three new operations:

```rust
pub struct ListChatParticipants {
    pub chat_id: String,
}
// GET /chats/{chatId}/participants
// Output: ItemEnvelope<Vec<ChatParticipant>>

pub struct AddChatParticipant {
    #[serde(skip)] pub chat_id: String,
    pub identity_id: String,
}
// POST /chats/{chatId}/participants
// Output: ItemEnvelope<ChatParticipant>

pub struct RemoveChatParticipant {
    pub chat_id: String,
    pub identity_id: String,
}
// DELETE /chats/{chatId}/participants/{identityId}
// Output: ActionResponse
```

**File: `src/v4/types/chats.rs`**

```rust
pub struct ChatParticipant {
    pub identity_id: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
```

Add handle methods to `ChatHandle`:
- `list_participants(&self) -> Result<ItemEnvelope<Vec<ChatParticipant>>>`
- `add_participant(&self, identity_id: impl Into<String>) -> Result<ItemEnvelope<ChatParticipant>>`
- `remove_participant(&self, identity_id: impl Into<String>) -> Result<ActionResponse>`

### Step 3.2 — Templates

**File: `src/v4/resources/messages.rs`**

Four new operations:

```rust
pub struct ListTemplates { /* no params */ }
// GET /templates
// Output: ListEnvelope<Template>

pub struct CreateTemplate {
    #[serde(flatten)]
    pub fields: BTreeMap<String, Value>,
}
// POST /templates
// Output: ItemEnvelope<Template>

pub struct GetTemplate {
    pub template_id: String,
}
// GET /templates/{templateId}
// Output: ItemEnvelope<Template>

pub struct DeleteTemplate {
    pub template_id: String,
}
// DELETE /templates/{templateId}
// Output: ActionResponse
```

**File: `src/v4/types/messages.rs`**

```rust
pub struct Template {
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
```

Add handle methods to `Messages`:
- `list(&self) -> Result<ListEnvelope<Template>>`
- `create(&self, fields: BTreeMap<String, Value>) -> Result<ItemEnvelope<Template>>`
- `get(&self, id: impl Into<String>) -> Result<ItemEnvelope<Template>>`
- `delete(&self, id: impl Into<String>) -> Result<ActionResponse>`

### Step 3.3 — Attachments

**File: `src/v4/resources/messages.rs`**

Two new operations:

```rust
pub struct CreateAttachment {
    #[serde(flatten)]
    pub fields: BTreeMap<String, Value>,
}
// POST /attachments
// Output: ItemEnvelope<Attachment>

pub struct GetAttachment {
    pub attachment_id: String,
}
// GET /attachments/{attachmentId}
// Output: ItemEnvelope<Attachment>
```

**File: `src/v4/types/messages.rs`**

```rust
pub struct Attachment {
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
```

Add handle methods to `Messages`:
- `create_attachment(&self, fields: BTreeMap<String, Value>) -> Result<ItemEnvelope<Attachment>>`
- `get_attachment(&self, id: impl Into<String>) -> Result<ItemEnvelope<Attachment>>`

### Step 3.4 — Poll handle method

**File: `src/v4/resources/chats.rs`**

Add `send_poll` convenience method on `ChatHandle`:

```rust
pub async fn send_poll(&self, title: impl Into<String>, options: impl IntoIterator<Item = impl Into<String>>) -> Result<ItemEnvelope<Poll>>
```

---

## Phase 4: Sync, Tests, Docs

### Step 4.1 — Update local API spec

`api-spec/openapi-v4.json` already fetched from remote. Verify `blooio-v4.openapi.json` and `blooio-v4.version` are in sync.

### Step 4.2 — Full test/lint matrix

```sh
cargo fmt --all --check
cargo clippy --all-features --all-targets -- -D warnings
cargo clippy --no-default-features --features sync,webhooks --all-targets -- -D warnings
cargo test --all-features
cargo test
cargo test --no-default-features --features sync,webhooks
RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +nightly doc --all-features --no-deps
```

### Step 4.3 — Update examples

All v4 examples (`examples/v4_quickstart.rs`, `examples/v4_blocking.rs`, `examples/v4_pagination.rs`, etc.) that use `MessageContent` must switch to `MessageContentFields::text(...)`.

### Step 4.4 — Update CHANGELOG.md

Document breaking change: `MessageContent` tagged enum → `MessageContentFields` flat struct; removal of `SenderSelector`.

---

## File Change Summary

### Phase 1 (complete) — 6 files touched

| File | Scope |
|------|-------|
| `src/v4/types/messages.rs` | **Major** — `MessageContent` → `MessageContentFields` + `Hybrid`, `RichLinkFields`, `InteractiveFields`, `TemplateFields`; `SenderSelector` removed; `MessageSendDetails` gained `from` field |
| `src/v4/types/chats.rs` | Added `Serialize` to `PollContent` (resolved re-export ambiguity) |
| `src/v4/resources/messages.rs` | **Major** — Rewrote `SendMessage`, updated `SendMessageToChat`, added 5 unit tests |
| `src/v4/resources/channels.rs` | Updated `SendMessageToChannel` content type |
| `tests/v4_integration_async.rs` | Updated body matchers and types |
| `examples/v4_quickstart.rs` | Updated to `MessageContentFields` |

### Remaining phases (pending)

| File | Scope |
|------|-------|
| `src/v4/types/messages.rs` | Phase 2: Add `Direction` enum, `alias` on `RoutingMetadata` |
| `src/v4/types/channels.rs` | Phase 2: Add `ChannelStatus` enum, update `Channel`, `PriorityChannel` |
| `src/v4/types/chats.rs` | Phase 2: Add `ChatState` enum; Phase 3: Add `ChatParticipant` struct |
| `src/v4/types/contacts.rs` | Phase 2: Add `contact_id` on `ContactIdentity` |
| `src/v4/resources/messages.rs` | Phase 3: Template & attachment stub operations |
| `src/v4/resources/chats.rs` | Phase 3: Chat participant operations, poll handle method |
| `tests/v4_integration_sync.rs` | No changes needed for Phase 1 (no MessageContent usage) |
| `api-spec/openapi-v4.json` | Already updated from remote |
| `CHANGELOG.md` | Document breaking changes |

**Phase 1:** 6 files touched, 14 distinct changes. All verified via `cargo fmt`, `cargo clippy` (both feature sets), and `cargo test` (all 3 feature combinations) — 399 tests pass.
**Remaining:** Phases 2–4 cover 8+ additional files with additive fields, stub operations, doc generation, and CHANGELOG updates.
