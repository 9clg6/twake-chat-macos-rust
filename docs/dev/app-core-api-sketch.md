# App core API sketch: room list and timeline, without a network

Written by the test author from the need, before the production code exists. This is the
exact public surface the tests use. The implementer may add more, but every item here must
exist with these names, types and semantics. Anything the implementer thinks wrong is
challenged against the need (the cited source), not against the code.

Slice: "see the room list and a conversation", on fakes. Sources: `docs/spec/features/roomlist.md`
(D190, D191, D109, D286, D222, D247, D392), `timeline.md` (D495, D523, D532, D605, e2e
day-divider / message-failed / send-echo), ARCHITECTURE.md sections 3, 4, 5, A009, A011.

Tests: `crates/twake-domain/tests/{mxc_txn,room_list,timeline}.rs`,
`crates/twake-app/tests/{ports,room_list_state,timeline_state}.rs`,
`crates/twake-testing/tests/{fakes,flow}.rs`, and the contract suites
`crates/twake-testing/src/contract.rs` (written, not to be loosened).
Manifests already edited by the test author: `twake-app` gets dev-dependency `futures = "0.3"`
(the implementer moves it to `[dependencies]`: the ports use `futures::future::BoxFuture` and
`futures::stream::BoxStream`), `twake-testing` gets dependency `futures = "0.3"`.
The implementer adds `thiserror` to `twake-app`. `twake-testing/src/lib.rs` already has
`pub mod contract;`; it must also re-export `FakeRoomList` and `FakeTimeline`.
No tokio anywhere in `twake-domain` / `twake-app` (A011 Q6).

---

## 1. `twake-domain` (crate root re-exports)

```rust
pub use ids::MxcUri;     // mxc://<server>/<media id>
pub use ids::TxnId;      // client transaction id of a send
pub use time::{Timestamp, Day};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MxcUri { /* private */ }
impl MxcUri {
    pub fn parse(s: &str) -> Result<Self, IdError>; // "" -> Empty; other scheme, no server, no media id,
                                                    // media id with '/', blank or control char -> Err
    pub fn as_str(&self) -> &str;
    pub fn server_name(&self) -> &str;              // "localhost:8008" keeps the port
    pub fn media_id(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TxnId { /* private */ }
impl TxnId {
    pub fn parse(s: &str) -> Result<Self, IdError>; // "" -> Empty; any whitespace -> Err
    pub fn as_str(&self) -> &str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(/* private u64 */);            // milliseconds since the Unix epoch
impl Timestamp {
    pub fn from_millis(ms: u64) -> Self;
    pub fn as_millis(self) -> u64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day { pub year: i32, pub month: u8, pub day: u8 }  // derive order = calendar order
impl Day {
    /// Calendar day of `ts` for a reader at `utc_offset_secs` east of UTC (proleptic Gregorian,
    /// leap years right: 2024-02-29 exists, 2100-02-29 does not).
    pub fn of(ts: Timestamp, utc_offset_secs: i32) -> Self;
}
```

### `twake_domain::room_list`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoomKind { Direct, Group, Space, Broadcast, Voice }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomSummary {
    pub id: RoomId,
    pub name: String,
    pub avatar: Option<MxcUri>,
    pub kind: RoomKind,
    pub last_message_preview: Option<String>,   // already readable text ("Image", "Message deleted"...)
    pub last_activity: Timestamp,               // last message, else creation (D101)
    pub unread: UnreadCount,                    // twake_domain::read_state::UnreadCount
    pub is_favourite: bool,                     // "Pin" = favourite tag (D247)
    pub is_low_priority: bool,                  // D190
    pub space: Option<RoomId>,                  // the space it sits in, for the Spaces chip
}
impl RoomSummary {
    pub fn new(id: RoomId, name: impl Into<String>, kind: RoomKind) -> Self;
        // defaults: no avatar, no preview, last_activity 0, unread all zero + not marked, flags false, no space
    pub fn with_avatar(self, avatar: MxcUri) -> Self;
    pub fn with_last_message_preview(self, text: impl Into<String>) -> Self;
    pub fn with_last_activity(self, ts: Timestamp) -> Self;
    pub fn with_unread(self, unread: UnreadCount) -> Self;
    pub fn favourite(self) -> Self;
    pub fn low_priority(self) -> Self;
    pub fn in_space(self, space: RoomId) -> Self;
}

/// Order of the list (D190): not low priority before low priority; favourites before others;
/// newest `last_activity` first; **stable** (equal rooms keep their given order).
pub fn sort_rooms(rooms: &mut [RoomSummary]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoomFilter { #[default] All, Unread, Broadcasts, Directs, Spaces, Favourites }
impl RoomFilter {
    pub fn matches(&self, room: &RoomSummary) -> bool;
    // All: everything. Unread: unread.is_unread() and kind != Space (D286). Broadcasts: kind Broadcast.
    // Directs: kind Direct. Spaces: kind Space or space.is_some(). Favourites: is_favourite.
}

/// Search of a room name (D392): empty query matches; case and accents ignored on both sides
/// (the accent folding may need a dependency: a decision A0nn first, P8); a query that ends with
/// a space matches its last word as a whole word ("ann " finds "Ann Lee" and "Lee, Ann",
/// not "Anna" nor "Joann"); otherwise a substring.
pub fn matches_search(name: &str, query: &str) -> bool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomListDiff {
    Reset(Vec<RoomSummary>),                              // whole list; same id twice: the last wins
    Insert(RoomSummary),                                  // a known id: replaces
    Remove(RoomId),                                       // unknown id: no-op
    Update(RoomSummary),                                  // replaces by id; unknown id: ignored (not inserted)
    Move { id: RoomId, last_activity: Timestamp },        // new activity only; unknown id: no-op
}

/// Pure. The list is keyed by id and always sorted by `sort_rooms`, whatever the diff.
pub fn apply_diff(list: &[RoomSummary], diff: RoomListDiff) -> Vec<RoomSummary>;
```

### `twake_domain::timeline`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendState { Sending, Sent, Failed }    // also Copy

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub sender: UserId,
    pub timestamp: Timestamp,
    pub body: String,
    pub reply_to: Option<EventId>,
    pub edited: bool,
    pub send_state: SendState,
    /* private: event id and/or transaction id, at least one */
}
impl Message {
    pub fn remote(event: EventId, sender: UserId, ts: Timestamp, body: impl Into<String>) -> Self; // Sent
    pub fn local(txn: TxnId, sender: UserId, ts: Timestamp, body: impl Into<String>) -> Self;      // Sending
    pub fn with_txn_id(self, txn: TxnId) -> Self;     // the server event that came from this send
    pub fn with_reply_to(self, event: EventId) -> Self;
    pub fn edited(self) -> Self;
    pub fn failed(self) -> Self;
    pub fn event_id(&self) -> Option<&EventId>;
    pub fn txn_id(&self) -> Option<&TxnId>;
    pub fn matches(&self, key: &MessageKey) -> bool;  // Event(e): event_id == e; Txn(t): txn_id == t
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageKey { Event(EventId), Txn(TxnId) }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineItem { DayDivider(Day), NewMessages, Message(Message) }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineDiff {
    Reset(Vec<Message>),                                        // room opened / switched
    Append(Vec<Message>),                                       // newer messages and echoes
    Prepend { messages: Vec<Message>, reached_start: bool },    // an older page, oldest first
    Update(Message),                                            // by event id or txn; unknown: ignored
    Remove(MessageKey),                                         // unknown: ignored
}
```

---

## 2. `twake-app`

### `twake_app::ports` (A009: boxed `Send` futures, `Arc<dyn Port>`-safe)

```rust
use futures::{future::BoxFuture, stream::BoxStream};

pub const TIMELINE_PAGE_SIZE: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoomListError { Unavailable, NotSignedIn }   // distinct non-empty Display texts

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TimelineError { RoomNotFound, Unavailable }  // distinct non-empty Display texts

pub trait RoomListPort: Send + Sync {
    /// The first batch of every subscription is exactly `[Reset(all rooms)]`; later batches are diffs.
    fn subscribe(&self) -> BoxFuture<'_, Result<BoxStream<'static, Vec<RoomListDiff>>, RoomListError>>;
}

pub trait TimelinePort: Send + Sync {
    /// First batch: `[Reset(last TIMELINE_PAGE_SIZE messages, oldest first)]`. Unknown room: `RoomNotFound`.
    fn open(&self, room: RoomId) -> BoxFuture<'_, Result<BoxStream<'static, Vec<TimelineDiff>>, TimelineError>>;
    /// Returns once asked; the page arrives on the open streams as `[Prepend { messages, reached_start }]`
    /// (the 30 messages before the ones held; `reached_start` when none remain before them).
    fn paginate_back(&self, room: RoomId) -> BoxFuture<'_, Result<(), TimelineError>>;
    /// Returns once accepted; the local echo arrives on the open streams as `[Append([Message::local(..)])]`
    /// (sender = the user, state Sending, a fresh `TxnId`). The server event comes later, as an
    /// `Append` of a message carrying the same `TxnId` (`with_txn_id`).
    fn send_text(&self, room: RoomId, body: String) -> BoxFuture<'_, Result<(), TimelineError>>;
}
```

### `twake_app::room_list`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomListStatus { Loading, Empty, NoMatch, Ready, Failed(RoomListError) }

pub struct RoomListState { /* private */ }
impl RoomListState {
    pub fn new() -> Self;                                // Loading, filter All, search ""
    pub fn apply(&mut self, batch: Vec<RoomListDiff>);   // diffs in order; the first batch loads the list; clears Failed
    pub fn set_filter(&mut self, filter: RoomFilter);
    pub fn filter(&self) -> RoomFilter;
    pub fn set_search(&mut self, query: &str);
    pub fn search(&self) -> &str;
    pub fn fail(&mut self, error: RoomListError);        // status Failed; rooms kept; the next batch recovers
    pub fn view(&self) -> Vec<RoomSummary>;              // sorted, filter AND search applied
    pub fn status(&self) -> RoomListStatus;
    pub fn total_unread(&self) -> UnreadCount;           // read_state::total_unread over ALL rooms (badge)
}
// status: Failed after fail(); Loading before the first batch; NoMatch when the view is empty and
// (search not empty, or filter != All with at least one room); Empty when the view is empty with no
// room at all (and no search); Ready otherwise.
```

### `twake_app::timeline`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineStatus { Loading, Empty, Ready, Failed(TimelineError) }

pub struct TimelineState { /* private */ }
impl TimelineState {
    pub fn new(utc_offset_secs: i32) -> Self;                // day dividers follow this offset
    pub fn apply(&mut self, diff: TimelineDiff);
    pub fn mark_new_messages_from(&mut self, first_unread: EventId);
    pub fn items(&self) -> Vec<TimelineItem>;
    pub fn reached_start(&self) -> bool;
    pub fn fail(&mut self, error: TimelineError);
    pub fn status(&self) -> TimelineStatus;
}
```

Rules of `apply`:
- `Reset`: replaces all messages, forgets `reached_start` and the marker; status Empty or Ready; an
  event id repeated inside the batch is kept once, with the **last** content (like the room list).
- `Append`: each message goes last, except: same event id already held -> replaced in place (never
  twice); carries a `TxnId` equal to a held local message's -> replaces that echo **in the same
  row**.
- `Prepend`: messages go first, in the given order, skipping event ids already held and repeats
  inside the page (the first is kept);
  `reached_start == true` sets the flag (never cleared except by `Reset`).
- `Update`: replaces in place the message with the same event id or txn id; unknown: ignored.
- `Remove`: drops the message matching the key; unknown: ignored; no message left: status Empty.
- `items()` is derived, never stored: a `DayDivider(Day::of(ts, offset))` before the first message and
  before each message whose day differs from the previous message's; `NewMessages` right before the
  message set by `mark_new_messages_from`, **after** that message's day divider; no marker while that
  event is not held (it appears when it is).
- `fail(e)`: status `Failed(e)`, messages kept; the next `Reset` recovers.

---

## 3. `twake-testing`

```rust
pub struct FakeRoomList { /* Mutex inside; shareable as Arc */ }
impl FakeRoomList {
    pub fn new(rooms: Vec<RoomSummary>) -> Self;
    pub fn failing(self, error: RoomListError) -> Self;       // subscribe returns Err(error)
    pub fn push(&self, diffs: Vec<RoomListDiff>);             // one batch to every live subscriber
}
impl RoomListPort for FakeRoomList { /* first batch: [Reset(rooms)] sorted, state = every push applied (apply_diff) */ }

pub struct FakeTimeline { /* Mutex inside */ }
impl FakeTimeline {
    pub fn new(me: UserId, now: Timestamp) -> Self;           // `now` stamps every echo and confirmation
    pub fn with_room(self, room: RoomId, history: Vec<Message>) -> Self;   // oldest first
    pub fn failing(self, error: TimelineError) -> Self;       // open / paginate_back / send_text return Err(error)
    pub fn push(&self, room: &RoomId, diffs: Vec<TimelineDiff>);   // one batch to the subscribers of that room only
    pub fn confirm_sends(&self, room: &RoomId);   // one batch: an Append([remote.with_txn_id]) per pending send, in send order,
                                                  // fresh distinct event ids, state Sent; the messages join the history; nothing pending: no batch
    pub fn fail_sends(&self, room: &RoomId);      // one batch: an Update(echo.failed()) per pending send; they do not join the history
}
impl TimelinePort for FakeTimeline { /* paging as the port doc: windows of TIMELINE_PAGE_SIZE */ }
```

Contract suites (`twake_testing::contract`, already written, run by the fakes' tests and later by
the Matrix adapter):

```rust
pub type Verdict = Result<(), String>;
pub fn room_list(make: impl Fn(Vec<RoomSummary>) -> Arc<dyn RoomListPort>) -> Verdict;
pub fn timeline(make: impl Fn(UserId, RoomId, Vec<Message>) -> Arc<dyn TimelinePort>) -> Verdict;
```

## 4. Open questions (need, not code)

1. `Move` in an id-keyed, self-sorted list is only an activity bump. If the SDK adapter wants
   index-based `Move`, the model changes; decide before `twake-matrix`.
2. Sort ties: stable (given order). A canonical tie-break by id would make the order independent
   of arrival; the need says nothing.
3. A favourite that is also low priority: D190 is silent; the tests do not pin it.
4. During a search D238 says pins and low priority do not apply; not specified here (the order of
   `view()` does not change with the search).
5. Space tree order (a space followed by its rooms, D190) is not in `RoomListState` yet.
6. Echo confirmed in place (row kept) rather than moved to its server position; unspecified by the
   need, chosen so a bubble never jumps.
7. Accent folding needs `unicode-normalization` or a table: a decision (P8) before the code.
8. `items()` and `view()` return owned `Vec`s; fine for 30 messages, to measure on the 10k fixture
   (P7) before moving to `imbl`.
