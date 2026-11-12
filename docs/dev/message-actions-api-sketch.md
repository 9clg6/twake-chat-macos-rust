# Message actions API sketch: menu, reactions, selection, pins, power levels

Written by the test author from the need, before the production code exists. This is the exact
public surface the tests use; the implementer may add more, but every item here must exist with
these names, types and semantics. A disagreement is settled against the need (the cited source),
not against the code.

Slice: "act on a message", pure logic and one port, no UI. Sources: PRD EPIC-07 (FR-075 to FR-080)
and EPIC-04 (FR-077); `docs/spec/features/timeline.md` D77, D90, D109, D118, D130, D168, D174,
D183, D229, D234, D255, D260, D274, D493, D583, D587, D588; `threads.md`; Matrix spec
(`m.annotation` / `m.reaction`, redactions, `m.room.pinned_events`, `m.room.power_levels`, D418);
rules read in the React client (`useMessageActions.tsx`, `MessageSelectionBar.tsx`,
`recentEmojis.ts`, `makeToggleReaction`), read only. ARCHITECTURE.md A009 (boxed `Send` futures),
A011 (domain and app are pure, no tokio).

Tests: `crates/twake-domain/tests/{reaction,power_levels,pinned}.rs`,
`crates/twake-app/tests/{message_actions,reaction_toggle,selection}.rs`,
`crates/twake-testing/tests/message_actions_port.rs` and the contract suite
`crates/twake-testing/src/message_actions_contract.rs` (written, not to be loosened; reachable as
`twake_testing::contract::message_actions`). `twake-testing/src/lib.rs` must declare
`mod message_actions_contract;` (done by the test author) and re-export `FakeMessageActions`.

Out of scope (YAGNI, not in the brief): Forward, Save, Edit history, "N Seen", Download, assistant
entries (D201), the full emoji picker, threads beyond the "Start a thread" entry, UI.

---

## 1. `twake-domain`

### `twake_domain::reaction` (m.annotation of `m.reaction`)

```rust
/// One `m.reaction` event: `sender` put `key` on a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionEvent { pub event_id: EventId, pub sender: UserId, pub key: String }

/// The people who gave one emoji to one message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionGroup { /* private */ }
impl ReactionGroup {
    pub fn key(&self) -> &str;                       // the spelling first seen (D174)
    pub fn senders(&self) -> &[UserId];              // first reaction first, once each
    pub fn count(&self) -> usize;                    // == senders().len()
    pub fn reacted_by(&self, user: &UserId) -> bool;
    pub fn event_of(&self, user: &UserId) -> Option<&EventId>; // that user's reaction event
}

/// The reactions of one message, one group per emoji.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reactions { /* private */ }
impl Reactions {
    pub fn new() -> Self;
    /// Adds a reaction. Same emoji, ignoring `\u{FE0F}` (D174), joins the group of the first
    /// spelling. A second reaction of the same sender on the same emoji is ignored (the first
    /// event stays). A group is created, last, at its first reaction.
    pub fn add(&mut self, event: ReactionEvent);
    /// The redaction of a reaction event. Returns whether something was removed (an unknown
    /// event id, such as an ignored duplicate, removes nothing). A group left empty disappears;
    /// the same emoji later starts a new group, last.
    pub fn redact(&mut self, event_id: &EventId) -> bool;
    pub fn groups(&self) -> &[ReactionGroup];        // order of first appearance, stable
    pub fn is_empty(&self) -> bool;
    /// The group of `emoji`, whichever way it is spelled.
    pub fn group_of(&self, emoji: &str) -> Option<&ReactionGroup>;
}
```

### `twake_domain::power_levels` (`m.room.power_levels`, D418)

```rust
/// Spec defaults when a field is absent: users_default 0, events_default 0, state_default 50,
/// redact 50, ban 50, no per-user and no per-event entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerLevels { /* private */ }
impl PowerLevels {
    pub fn new() -> Self;                                    // the defaults above
    pub fn with_user(self, user: UserId, level: i64) -> Self;
    pub fn with_users_default(self, level: i64) -> Self;
    pub fn with_event(self, event_type: &str, level: i64) -> Self; // `events` map
    pub fn with_events_default(self, level: i64) -> Self;
    pub fn with_state_default(self, level: i64) -> Self;
    pub fn with_redact(self, level: i64) -> Self;
    pub fn with_ban(self, level: i64) -> Self;

    pub fn level_of(&self, user: &UserId) -> i64;            // users[user] or users_default
    /// Sending a message-like event of this type: `events[type]` or `events_default`.
    pub fn can_send_event(&self, user: &UserId, event_type: &str) -> bool;
    /// Sending a state event of this type: `events[type]` or `state_default`.
    pub fn can_send_state(&self, user: &UserId, event_type: &str) -> bool;
    pub fn can_send_message(&self, user: &UserId) -> bool;   // event type "m.room.message"
    pub fn can_send_reaction(&self, user: &UserId) -> bool;  // event type "m.reaction"
    pub fn can_pin(&self, user: &UserId) -> bool;            // state "m.room.pinned_events"
    pub fn can_ban(&self, user: &UserId) -> bool;            // level >= ban
    /// Redacting an event of `sender`: always one's own; another's needs level >= redact.
    pub fn can_redact(&self, me: &UserId, sender: &UserId) -> bool;
}

/// The rule alone: own events always, others' when `level >= required`.
pub fn can_redact(own: bool, level: i64, required: i64) -> bool;
```

### `twake_domain::pinned` (`m.room.pinned_events`)

```rust
/// The pinned events of a room, in the order of the state event (oldest pin first).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PinnedState { /* private */ }
impl PinnedState {
    pub fn new() -> Self;
    pub fn from_ids(ids: Vec<EventId>) -> Self;       // duplicates dropped, first kept
    pub fn ids(&self) -> &[EventId];
    pub fn contains(&self, id: &EventId) -> bool;
    pub fn pin(&mut self, id: EventId);               // idempotent; a new pin goes last
    pub fn unpin(&mut self, id: &EventId);            // unknown id is fine; others kept (D229)
    /// Several at once (D260): `ids` are in timeline order and are appended in that order.
    pub fn pin_all(&mut self, ids: &[EventId]);
    pub fn unpin_all(&mut self, ids: &[EventId]);
    /// The banner shows one pin at a time, the newest pin first; each click shows the next and
    /// wraps (D229, e2e "Pinned message 2 of 3"). `shown` counts the clicks. None when empty.
    pub fn banner(&self, shown: usize) -> Option<PinnedBanner>;
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedBanner { pub event: EventId, pub position: usize /* 1-based */, pub total: usize }
```

---

## 2. `twake-app`

### `twake_app::reactions`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactionIntent { Send(String), Redact(EventId) }

/// What a click on `emoji` does for `me`, given the reactions now: I already reacted with it
/// (whichever spelling) -> `Redact(my reaction event)`; else `Send(key)`, where `key` is the
/// spelling of the existing group if there is one, else `emoji` as given (D174). Pure.
pub fn apply_reaction_toggle(state: &Reactions, me: &UserId, emoji: &str) -> ReactionIntent;

/// The fixed row of the menu and of the hover (D234, D274): ❤️ 👍 👎 😂 😮 😢, in this order.
pub const QUICK_REACTIONS: [&str; 6] = ["❤️", "👍", "👎", "😂", "😮", "😢"];

/// The emojis the user reacted with lately, for the "recent" tab of the picker.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecentEmojis { /* private */ }
impl RecentEmojis {
    pub const KEPT: usize = 8;
    pub fn new() -> Self;
    /// Newest first; the same emoji (ignoring `\u{FE0F}`) is kept once, moved to the front,
    /// with the spelling just used; only `KEPT` remain.
    pub fn remember(&mut self, emoji: &str);
    pub fn list(&self) -> &[String];
}
```

### `twake_app::message_actions`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    React,      // the quick-reaction card at the head of the menu, with its chevron to the picker
    Reply, CopyText, Pin, Unpin, Select,   // section 1
    Thread, Edit,                          // section 2
    Translate,                             // section 3 (D588: beside the assistant)
    CopyLink,                              // section 4
    Report, Delete,                        // section 5, Delete last
    Retry, Discard,                        // a failed message only
}

/// What the message list knows beyond `Message`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MessageFacts {
    pub deleted: bool, pub undecryptable: bool, pub pinned: bool,
    pub in_thread: bool,        // a reply inside a thread
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomCtx { pub power: PowerLevels, pub scribe: bool /* the Scribe is offered here (D215) */ }

/// The entries of the menu of `message`, in menu order. Rules:
/// - `Sending` or deleted message: no entry. `Failed`: `[Retry, Discard]` only.
/// - Order: React, Reply, CopyText, Pin|Unpin, Select, Thread, Edit, Translate, CopyLink, Report, Delete.
/// - React: `power.can_send_reaction(me)`.
/// - Reply: `can_send_message(me)`. Thread: same, and not `in_thread`, and not undecryptable.
/// - CopyText: body not empty and not undecryptable (D130).
/// - Pin / Unpin: `can_pin(me)`; `Unpin` when `facts.pinned`, else `Pin`; never both.
/// - Select: always (sent, not deleted).
/// - Edit: own, not undecryptable, `can_send_message(me)`; no age limit.
/// - Translate: not own, body not empty, not undecryptable, `room.scribe` (D588).
/// - CopyLink: always.
/// - Report: not own.
/// - Delete: own, or `can_redact(me, sender)` (D77).
pub fn message_actions(message: &Message, facts: &MessageFacts, room: &RoomCtx, me: &UserId) -> Vec<Action>;

/// The menu lines (D274): [Reply, CopyText, Pin, Unpin, Select] [Thread, Edit] [Translate]
/// [CopyLink] [Report, Delete] [Retry, Discard]; `React` is the card above, in no section. An
/// empty section is absent (no line drawn). Input order is kept.
pub fn menu_sections(actions: &[Action]) -> Vec<Vec<Action>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkAction { Copy, Pin, Unpin, Delete }

/// The bar of a selection (D183, D260), in order Copy, Pin|Unpin, Delete. Empty selection: none.
/// Pin only if `can_pin`; `Unpin` when every ticked message is pinned, else `Pin`. Delete only
/// when it is permitted for EVERY ticked message (own, or `can_redact`).
pub fn selection_actions(items: &[(&Message, &MessageFacts)], room: &RoomCtx, me: &UserId) -> Vec<BulkAction>;
```

### `twake_app::selection` (dark bar of ticked messages, D183)

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection { /* private */ }
impl Selection {
    pub fn new() -> Self;                             // not active
    pub fn start(&mut self, id: EventId);             // "Select" in the menu: active, only `id`, anchor `id`
    pub fn is_active(&self) -> bool;
    pub fn len(&self) -> usize;                       // "N selected"
    pub fn contains(&self, id: &EventId) -> bool;
    /// A press: ticks, or unticks if ticked. The anchor moves to `id`.
    pub fn toggle(&mut self, id: EventId);
    /// Shift+click: ticks every message of `order` from the anchor to `id` included (either
    /// direction), keeps the ones already ticked; the anchor moves to `id`. No anchor, or an id
    /// missing from `order`: same as `toggle`.
    pub fn extend_to(&mut self, id: EventId, order: &[EventId]);
    /// Escape: leaves the mode, nothing done.
    pub fn clear(&mut self);
    /// The ticked ids in timeline order (`order`), once each; ids missing from `order` are dropped.
    pub fn ids(&self, order: &[EventId]) -> Vec<EventId>;
}
```
Never a duplicate in `ids`; `len` counts distinct ids.

### Port (`twake_app::ports`, A009)

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActionError { RoomNotFound, Forbidden, Unavailable }   // Display texts free

pub trait MessageActionsPort: Send + Sync {
    /// Sends an `m.reaction` on `target`; returns the id of the reaction event.
    fn send_reaction(&self, room: RoomId, target: EventId, key: String) -> BoxFuture<'_, Result<EventId, ActionError>>;
    /// Redacts an event (a reaction, a message). Redacting twice, or an unknown event, is fine.
    fn redact(&self, room: RoomId, event: EventId) -> BoxFuture<'_, Result<(), ActionError>>;
    /// Adds to `m.room.pinned_events`; already pinned is fine; the others stay, order kept.
    fn pin(&self, room: RoomId, event: EventId) -> BoxFuture<'_, Result<(), ActionError>>;
    fn unpin(&self, room: RoomId, event: EventId) -> BoxFuture<'_, Result<(), ActionError>>;
    /// The pinned events now, in state order.
    fn pinned(&self, room: RoomId) -> BoxFuture<'_, Result<Vec<EventId>, ActionError>>;
    /// Reports a message to the server administrators, with an optional reason (D255).
    fn report(&self, room: RoomId, event: EventId, reason: Option<String>) -> BoxFuture<'_, Result<(), ActionError>>;
}
```

---

## 3. `twake-testing`

```rust
pub struct FakeMessageActions { /* private */ }
impl FakeMessageActions {
    pub fn new() -> Self;                 // knows no room
    pub fn with_room(self, room: RoomId) -> Self;
    pub fn forbidding(self) -> Self;      // every call: Err(Forbidden)
    pub fn unavailable(self) -> Self;     // every call: Err(Unavailable)
}
// contract: pub fn message_actions(make: impl Fn() -> Arc<dyn MessageActionsPort>) -> Verdict
// `make` builds a port that knows the room `!actions1:example.org`, with no pin and no reaction.
```
Order of checks in the fake: unavailable, then unknown room, then forbidden.
