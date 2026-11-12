# API sketch required by the tests

Written by the test author from the need, before the production code exists. This is
the exact public surface the tests use. The implementer may add more, but every item
here must exist with these names, types and semantics. Anything the implementer thinks
wrong is challenged against the need (the cited source), not against the code.
Each sketch was checked by compiling the tests against a throw-away reference
implementation: all tests pass against it, so the sketch is satisfiable.

Tests: `crates/twake-domain/tests/ids.rs`, `crates/twake-domain/tests/read_state.rs`,
`xtask/tests/check_deps.rs`. Dev-dependency added: `proptest = "1"` in `twake-domain`.

---

## 1. `crates/twake-domain/tests/ids.rs`

Need: Matrix spec, "Common identifier format" (ids are 255 bytes at most, sigil and server
name included; server name = hostname, IPv4 or `[IPv6]`, optional `:port` of 1 to 5 digits).

```rust
// crates/twake-domain/src/lib.rs (or ids.rs, re-exported at the crate root)
pub use ids::{EventId, IdError, RoomAlias, RoomId, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdError {
    Empty,             // "" and sigil-only / empty localpart
    WrongSigil,        // first char is not the sigil of the kind
    MissingServer,     // no ':' where the kind needs a server (user, alias, room)
    InvalidServerName, // empty server, bad port, unclosed [ipv6], space...
    InvalidCharacter,  // whitespace, control chars, NUL, '@' in a user localpart
    TooLong,           // more than 255 BYTES in total
}
// impl std::error::Error + Display via thiserror

// Each of the four: #[derive(Debug, Clone, PartialEq, Eq, Hash)], Display (writes the input
// unchanged), FromStr<Err = IdError> (same as parse).
impl RoomId    { pub fn parse(s: &str) -> Result<Self, IdError>; pub fn as_str(&self) -> &str;
                 pub fn server_name(&self) -> Option<&str>; }            // None for a v12 id
impl EventId   { pub fn parse(s: &str) -> Result<Self, IdError>; pub fn as_str(&self) -> &str;
                 pub fn server_name(&self) -> Option<&str>; }            // None for v3+ hashes
impl UserId    { pub fn parse(s: &str) -> Result<Self, IdError>; pub fn as_str(&self) -> &str;
                 pub fn localpart(&self) -> &str; pub fn server_name(&self) -> &str; }
impl RoomAlias { pub fn parse(s: &str) -> Result<Self, IdError>; pub fn as_str(&self) -> &str;
                 pub fn localpart(&self) -> &str; pub fn server_name(&self) -> &str; }
```

Behaviour pinned by the tests: `parse` then `to_string` returns the input byte for byte
(case kept); sigils `!` `$` `@` `#`; the sigil decides the kind (`WrongSigil`); `Empty` for
`""`; `MissingServer` for `@alice` and `#room`; `TooLong` at 256 bytes (255 accepted),
counted in bytes (an alias of 126 `é` is too long); event ids accept `$hash`, `$hash:server`,
base64 characters `/ + - _`; user localparts accept `a-z 0-9 . _ = - / +` and historical
uppercase; no whitespace, tab, newline or NUL anywhere; port 1 to 5 digits.
Eq + Hash so ids are map keys. Parsing must never panic (property test on arbitrary text).

---

## 2. `crates/twake-domain/tests/read_state.rs`

Need: React `src/domain/readState.ts`, `readState.spec.ts`, and D60, D175, D222, D452, D483,
D510, D539, D558/D575, D615, inventory R21 and TH4. All pure functions over immutable values.
Module path `twake_domain::read_state`; uses `EventId`, `UserId` of section 1.

```rust
pub mod read_state {
    /// An event of a timeline, reduced to what the count needs.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TimelineEvent { pub id: EventId, pub sender: UserId /* + private fields */ }
    impl TimelineEvent {
        pub fn message(id: EventId, sender: UserId) -> Self;  // a shown message (text, media, poll, missed call...)
        pub fn other(id: EventId, sender: UserId) -> Self;    // reaction, edit, redaction, state event: never counted
        pub fn mentioning_me(self) -> Self;                   // the message mentions the user
    }

    /// Who the user is, what they have read, whom they ignore.
    #[derive(Debug, Clone)]
    pub struct ReadContext { /* private */ }
    impl ReadContext {
        pub fn new(me: UserId) -> Self;
        pub fn with_receipts(self, receipts: Vec<EventId>) -> Self; // all own receipts (public + private, any order)
        pub fn ignoring(self, user: UserId) -> Self;                // callable several times
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct TimelineReadState {
        pub unread_count: u32,
        pub mention_count: u32,                 // subset of unread_count
        pub latest_event_id: Option<EventId>,   // last *message* of the slice (own and ignored included)
    }
    /// events: oldest first.
    pub fn timeline_read_state(events: &[TimelineEvent], ctx: &ReadContext) -> TimelineReadState;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ThreadReadState {
        pub root: EventId,
        pub state: TimelineReadState,   // RAW counts (unread kept even when not followed); zeroed when muted
        pub is_participating: bool,     // the user follows / takes part
        pub is_muted: bool,
    }
    /// replies: oldest first, root not included.
    pub fn thread_read_state(root: EventId, replies: &[TimelineEvent],
                             is_participating: bool, is_muted: bool, ctx: &ReadContext) -> ThreadReadState;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RoomReadState { pub main: TimelineReadState, pub threads: Vec<ThreadReadState> }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum NotificationLevel { All, MentionsOnly, Nothing }
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RoomFlags { pub notification_level: NotificationLevel, pub marked_unread: bool }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct UnreadCount { pub unread: u32, pub mentions: u32, pub marked_unread: bool }
    impl UnreadCount { pub fn is_unread(&self) -> bool; } // unread > 0 || mentions > 0 || marked_unread

    pub fn room_unread(state: &RoomReadState, flags: &RoomFlags) -> UnreadCount;
    pub fn total_unread(counts: &[UnreadCount]) -> UnreadCount;      // sums; marked_unread always false in the total
    pub fn silent_unread(state: &RoomReadState, flags: &RoomFlags) -> u32; // MentionsOnly: the unread that would count; else 0

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ThreadId { Main, Thread(EventId /* root */) }
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ReceiptTarget { pub thread: ThreadId, pub event: EventId }
    pub fn receipt_targets(state: &RoomReadState) -> Vec<ReceiptTarget>;

    pub fn choose_receipt_event(ordered_ids: &[EventId], unread_count: u32, shown: Option<&EventId>) -> Option<EventId>;
}
```

Semantics (each pinned by a test):

- `timeline_read_state`: read point = the furthest of (the user's last own **message**) and
  (every receipt that points at an event held in `events`, any kind of event, furthest wins,
  order of the list irrelevant). A receipt on an event not held counts for nothing (so with
  no other read point everything held is unread, D483 + D510). Unread = messages of others,
  not ignored, after the read point. A non-message of the user (state event) is not a read
  (D539); non-messages of others never count. Mentions = unread messages marked
  `mentioning_me`.
- `thread_read_state`: same rule on the replies; muted thread: both counts 0 (D60).
  `latest_event_id` is the last reply message (the target of a thread receipt).
- `room_unread`: `unread = main + sum(followed threads)`; `mentions = main + sum(all threads)`
  (a mention counts in a thread not followed; muted threads already 0). `MentionsOnly`:
  unread 0, mentions kept. `Nothing`: both 0. `marked_unread` flag set in the result only
  when `flags.marked_unread` and no count at all (D615); real counts win.
- `silent_unread`: the `unread` the room would have with `All`, for `MentionsOnly` only.
- `receipt_targets`: `Main` first (if `unread_count > 0 || mention_count > 0` and a latest
  event exists), then each thread in order under the same condition, **followed or not**.
  Marked-unread flag is not a reason for a receipt.
- `choose_receipt_event`: `Some(shown)` iff `unread_count > 0` and `shown` is one of the last
  `min(unread_count, len)` ids (last occurrence); else `None` (never goes back, unheld shown = None).
- Property tests: applying `receipt_targets` as receipts leaves the room read and
  `receipt_targets` empty; a receipt never raises a count; `mention_count <= unread_count <=`
  messages of others; the chosen receipt is always inside the unread tail.

---

## 3. `xtask/tests/check_deps.rs`

Need: ARCHITECTURE.md section 3 (table "May depend on", P2 exclusive systems, hard rules 1-4).
`xtask` needs a library target: add `xtask/src/lib.rs` (`pub mod check_deps;`), keep `main.rs`
as the command (`cargo xtask check-deps` = `load_graph` + `check`, prints violations, exit 1).

```rust
pub mod check_deps {
    /// One workspace crate as read from `cargo metadata` (workspace members only).
    #[derive(Debug, Clone)]
    pub struct CrateInfo { pub name: String, /* private: deps, dev_deps, externals, dev_externals */ }
    impl CrateInfo {
        pub fn new(name: &str) -> Self;
        pub fn dep(self, internal: &str) -> Self;          // normal + build dependency on a workspace crate
        pub fn dev_dep(self, internal: &str) -> Self;      // dev-dependency on a workspace crate
        pub fn external(self, name: &str) -> Self;         // normal + build dependency, non-workspace
        pub fn dev_external(self, name: &str) -> Self;     // dev-dependency, non-workspace
        pub fn has_dep(&self, internal: &str) -> bool;     // normal internal deps only
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum RuleId { InternalEdge, ExclusiveSystem, PureCore, UiAdapter, Leaf, Cycle, UnknownCrate }
    impl RuleId { pub fn as_str(&self) -> &'static str; }
    // "edge", "exclusive", "pure-core", "ui-adapter", "leaf", "cycle", "unknown-crate"

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Violation { pub krate: String, pub dependency: String, pub rule: RuleId }
    // Display: contains krate, dependency and rule.as_str()

    pub struct Rules { /* private */ }
    impl Rules {
        pub fn twake() -> Self;                              // the real table of section 3
        pub fn from_edges(edges: &[(&str, &str)]) -> Self;   // test rules: only an allowed-edge table
                                                             // (crates named in it are known); no hard
                                                             // rule 1/2/3 and no exclusive systems
    }

    pub fn check(rules: &Rules, crates: &[CrateInfo]) -> Vec<Violation>; // sorted (krate, dependency, rule)
    pub fn load_graph(workspace_manifest: &std::path::Path) -> Result<Vec<CrateInfo>, String>; // cargo_metadata
}
```

Semantics pinned by the tests (decisions of the test author where the document is silent):

- **One report per offending dependency**, under the most specific rule, in this order:
  `Leaf` (crate is `twake-tokens`/`twake-icons`, any normal dependency, internal or external)
  > `PureCore` (crate is `twake-domain`/`twake-app` and the dependency is an adapter crate,
  `twake-gpui`, or an external system of the P2 list) > `UiAdapter` (`twake-ui`/`twake-ui-kit` on an
  adapter crate) > `ExclusiveSystem` (external system outside its owner) > `InternalEdge`
  (internal edge absent from the table, `twake-testing` as a normal dependency included).
  Adapter crates = matrix, services, media, store, platform, calls.
- P2 owners by name: `matrix-sdk*` -> twake-matrix, `gpui*` -> twake-gpui, `livekit` -> twake-calls,
  `keyring*` -> twake-store, `reqwest` -> twake-services. Dev-dependencies are checked for P2 too.
- Dev-dependencies on workspace crates follow the same table, except that **any crate other than
  `twake-testing` may dev-depend on `twake-testing`**.
- `twake-desktop` may depend on every workspace crate of the table (not `twake-testing` as a
  normal dependency: see question Q4).
- `Cycle`: over normal edges between known crates, **leaving out edges already reported** (one fault,
  one report). One violation per cycle: `krate` = smallest crate name of the cycle, `dependency` = its
  smallest-named dependency on the cycle. A self-dependency is a cycle.
- `UnknownCrate`: a graph crate absent from the rules: `Violation { krate: name, dependency: "", rule }`.
- `check` does not depend on the order of the input crates.
- Real workspace test (`#[ignore]`, runs `cargo metadata`): `load_graph(<repo>/Cargo.toml)`
  returns all 17 members and `check(&Rules::twake(), ..)` is empty.
  Run with `cargo test -p xtask -- --ignored`.
