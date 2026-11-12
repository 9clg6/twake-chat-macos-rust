# Composer core API sketch: pure rules, no UI, no network

Written by the test author from the need, before the production code exists. This is the exact
public surface the tests use. The implementer may add more, but every item here must exist with
these names, types and semantics. A disagreement is argued against the need (the cited source),
not against the code.

Slice: FR-050 to FR-057 and FR-061 (`docs/prd-twake-chat-macos-rust-2026-11-12.md`), behaviour per
`docs/spec/features/composer.md` (D35, D97, D181, D382, D401, D431, D487, D502, D512, D621, e2e
"Composer", "Formatting", "Drafts"). Reference for the exact rules, read-only: `twake-space-chat`
`src/adapters/matrix/markdown.ts`, `mentionContent.ts`, `src/domain/botCommand.ts`, `mention.ts`,
`emojiShortcode.ts`, `application/useCases/typing.ts`. Matrix spec (D418): `m.room.message`,
`m.relates_to` (`m.in_reply_to`, `m.replace`), `org.matrix.custom.html`, `m.mentions`.

Tests: `crates/twake-domain/tests/{markdown,commands,mention}.rs`,
`crates/twake-app/tests/{composer_state,composer_suggest,composer_access,typing}.rs`,
`crates/twake-testing/tests/{draft_store,composer_flow}.rs` and the contract suite
`crates/twake-testing/src/draft_store_contract.rs` (written, not to be loosened; the implementer
adds `mod draft_store_contract;` to `lib.rs`, `pub use crate::draft_store_contract::draft_store;`
to `contract.rs`, and re-exports `FakeDraftStore`).
No manifest change: every dependency needed is already there. No tokio (A011 Q6).
Offsets (`cursor`, `start`) are **byte offsets on a char boundary**; one that is past the end is
clamped to the end, one inside a char is rounded down.

---

## 1. `twake-domain`

### `twake_domain::search`

```rust
/// A015: NFD, combining marks removed, lower case. "Élodie" -> "elodie". (`room_list` reuses it.)
pub fn fold(s: &str) -> String;
```

### `twake_domain::markdown`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingBody {
    pub body: String,                    // the text exactly as typed (Matrix: the plain fallback)
    pub formatted_body: Option<String>,  // org.matrix.custom.html; None when the text holds no Markdown
}

/// Chat Markdown (a subset, line by line) as Matrix HTML. The text is escaped first (& < > " '),
/// so no tag the user typed ever comes out: only the tags below, built here.
///   **b** -> <strong>   *i* and _i_ -> <em>   ~~s~~ -> <del>   <u>u</u> -> <u>   `c` -> <code>
///   [t](http(s)://.. or mailto:..) -> <a href="..">t</a>   (any other scheme: stays text)
///   lines starting "> " -> <blockquote> (lines joined by <br>)
///   lines "- a" / "* a" -> <ul><li>; "1. a" / "1) a" -> <ol><li>
///   fenced ``` -> <pre><code>..</code></pre> (the code escaped; an unclosed fence stays text)
///   other lines joined by <br>
/// A mark needs a non-space inside both edges ("~~ x ~~", "2 * 3 * 4" are text); `*i*` and `_i_`
/// are not read inside a word ("a*b*c", "snake_case_name" are text). A bare `www.x.org` is not
/// linked (display linkifies). When the HTML equals the escaped text with <br> for newlines the
/// text had no Markdown: `formatted_body` is None.
pub fn to_message(text: &str) -> OutgoingBody;

/// D97: the Markdown the link dialog writes. `address` is trimmed; http:// or https:// is kept,
/// `www.` gets `https://`, anything else (other scheme, spaces inside, empty) is None.
/// An empty `label` becomes the address. -> "[docs](https://www.x.org)"
pub fn link_markdown(label: &str, address: &str) -> Option<String>;
```

### `twake_domain::commands`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotCommand {
    pub bot: UserId,
    pub name: String,        // first word of the syntax: "leave"
    pub syntax: String,      // "leave {days} {reason...}"
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalCommand { DiscardSession }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prepared {
    Message { body: String },                             // "//x" -> "/x"; "/shrug" -> "¯\_(ツ)_/¯"
    Emote { body: String },                               // "/me waves" -> "waves"
    Bot { command: BotCommand, body: String },            // "/leave 2" -> "!leave 2"
    Local(LocalCommand),                                  // "/discardsession" (+ anything after)
    Unknown(String),                                      // "/foo bar" -> "foo" (lower case); not sent
}

/// The rules of `prepareCommandText` + `findLocalCommand`, on text already trimmed. A command name is
/// `[^\s/]+` followed by whitespace or the end; case does not matter. Order: `//` first, then
/// the name: a bot command of the room wins over the client's own, then `me` (needs text after it:
/// "/me" alone is Unknown("me")), `shrug` (text before the shrug, one space), `discardsession`,
/// otherwise Unknown. Text that does not start with "/", or "/ x", "/usr/bin": a plain Message.
pub fn prepare(text: &str, commands: &[BotCommand]) -> Prepared;

/// D27: the commands whose name starts with `query` (case ignored), in the order announced.
pub fn match_commands<'a>(commands: &'a [BotCommand], query: &str) -> Vec<&'a BotCommand>;
```

### `twake_domain::mention`

```rust
pub const MAX_MENTIONS: usize = 25;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MentionTarget { User(UserId), Room(RoomAlias), Everyone }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mention { pub target: MentionTarget, pub name: String }   // name as written in the text
impl Mention {
    pub fn user(id: UserId, name: &str) -> Self;
    pub fn room(alias: RoomAlias, name: &str) -> Self;
    pub fn everyone() -> Self;                                         // name "@room"
}

/// The `m.mentions` of a message.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mentions { pub user_ids: Vec<UserId>, pub room: bool }

/// D487/D401: the picked mentions the text still names (`text.contains(name)`), each user once, in
/// the order picked, at most MAX_MENTIONS users. Everyone -> `room: true`. A room link (`Room`)
/// notifies nobody (`m.mentions` stays empty for it).
pub fn extract(text: &str, picked: &[Mention]) -> Mentions;

/// Each present name of a `User` or `Room` mention (all its occurrences, not glued to a letter,
/// digit or `_`, not inside <a>, <code>, <pre>) becomes a matrix.to link:
/// `<a href="https://matrix.to/#/%40bob%3Alocalhost">bob</a>` (user), `.../%23general%3Alocalhost`
/// (room alias): percent-encoding of the whole id as `encodeURIComponent` does. Everyone stays text.
/// The base is `formatted_body`, else the escaped text with <br>; with nothing to link, the body
/// comes back unchanged (`formatted_body` still None when it was).
pub fn link_names(body: OutgoingBody, picked: &[Mention]) -> OutgoingBody;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member { pub user_id: UserId, pub name: String, pub matrix_name: Option<String> }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MentionCandidate { Person(Member), Everyone }

/// The list after "@<query>" (the query may start with a second "@" for a full id). Fold both sides
/// (accents, case). The own user is never offered. A member is found when a word of "name
/// matrix_name localpart" (split on whitespace . _ : -) or the user id (with or without "@") starts
/// with the query (best), or contains it; best first, then by folded name; at most MAX_MENTIONS
/// people. `@room` (only when `can_mention_room`): first when the query is a start of "room",
/// "all" or "everyone" (non-empty); last when the query is empty and at least one person is listed;
/// otherwise absent (D401, D431).
pub fn match_members(members: &[Member], query: &str, me: &UserId, can_mention_room: bool) -> Vec<MentionCandidate>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkableRoom { pub alias: RoomAlias, pub name: String }

/// The rooms after "#<query>" (D502: only rooms with an alias are given): a word of the name or of
/// the alias starting with the query first, then containing it; folded; by folded name; at most
/// MAX_MENTIONS.
pub fn match_rooms<'a>(rooms: &'a [LinkableRoom], query: &str) -> Vec<&'a LinkableRoom>;
```

---

## 2. `twake-app`

### `twake_app::typing` (FR-057, `makeNotifyTyping`)

```rust
pub const TYPING_RESEND_MS: u64 = 4_000;   // one notice per this delay while typing
pub const TYPING_TIMEOUT_MS: u64 = 6_000;  // how long the server shows the user as typing

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypingNotice { Started, Stopped }

#[derive(Debug, Default)]
pub struct TypingThrottle { /* private */ }
impl TypingThrottle {
    pub fn new() -> Self;
    /// A key stroke: Some(Started) at the first one, then at most one per TYPING_RESEND_MS since
    /// the last notice (exactly 4 000 ms later is allowed); else None.
    pub fn on_input(&mut self, now: Timestamp) -> Option<TypingNotice>;
    /// Message sent or field emptied: Some(Stopped) only if a Started went out and no Stopped since.
    pub fn stop(&mut self) -> Option<TypingNotice>;
}
```

### `twake_app::ports` (A009)

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DraftStoreError { #[error("the draft store is unavailable")] Unavailable }

/// Drafts per room, kept across restarts (FR-055).
pub trait DraftStorePort: Send + Sync {
    /// Replaces the draft of `room`. A draft whose text is "" is a `clear`.
    fn save(&self, room: RoomId, draft: Draft) -> BoxFuture<'_, Result<(), DraftStoreError>>;
    fn load(&self, room: RoomId) -> BoxFuture<'_, Result<Option<Draft>, DraftStoreError>>;
    /// An unknown room is fine.
    fn clear(&self, room: RoomId) -> BoxFuture<'_, Result<(), DraftStoreError>>;
}
```

### `twake_app::composer`

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Draft { pub text: String, pub mentions: Vec<Mention> }   // D487: picked saved with it

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryAction { Mic, Send }                                // FR-050

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisabledReason { OnlyModeratorsPost }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access { Enabled, Disabled(DisabledReason) }
/// FR-056: posting needs `required` (events_default); a member below it cannot.
pub fn can_post(my_power: i64, required: i64) -> Access;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyTarget { pub event_id: EventId, pub sender: UserId, pub preview: String }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditTarget { pub event_id: EventId, pub body: String, pub mentions: Vec<Mention> }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposerMode { Compose, Reply(ReplyTarget), Edit(EditTarget) }   // the banner

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind { Text, Emote }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Relation { None, Reply { to: EventId }, Replace { target: EventId } }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingMessage {
    pub kind: MessageKind,
    pub body: String,                    // trimmed, command prepared ("!leave 2 x", emote text...)
    pub formatted_body: Option<String>,  // markdown + matrix.to links of the mentions
    pub mentions: Mentions,              // all of them (an edit: goes in m.new_content)
    pub notify: Mentions,                // top level m.mentions: = mentions, but for an edit only the newly named
    pub relation: Relation,
    pub bot_command: Option<BotCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SubmitError {
    Empty,                            // nothing, or spaces and newlines only
    UnknownCommand(String),           // "/foo bar": text stays, the screen says "This command does not exist"
    Disabled(DisabledReason),
    LocalCommand(LocalCommand),       // ran on the device, sends nothing; the field is emptied
}                                     // each variant has a non-empty, distinct Display text

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection { pub anchor: usize, pub head: usize }       // head = the cursor

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionKind { Person, Room, Emoji, Command }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trigger { pub kind: SuggestionKind, pub start: usize, pub query: String }
// start = index of the symbol (@ # : /); query = what follows it up to the cursor

/// The suggestion layer asked by the text before `cursor` (rules of findMentionQuery, findRoomQuery,
/// findEmojiQuery, findCommandQuery): "@" and "#" at the start or after whitespace, then no space
/// ("@@bob:x" keeps its second "@" in the query); ":" at the start or after whitespace, then a
/// letter, then letters, digits, _ + -; "/" at the very start of the text, then no space and no "/".
/// "12:30", "http://x", "mail@x", "hi /le" ask nothing.
pub fn find_trigger(text: &str, cursor: usize) -> Option<Trigger>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmojiEntry { pub shortcode: String, pub glyph: String }

#[derive(Debug, Clone, Default)]
pub struct Sources {                      // what the suggestions draw from, given by the screen
    pub members: Vec<Member>, pub rooms: Vec<LinkableRoom>, pub commands: Vec<BotCommand>,
    pub emojis: Vec<EmojiEntry>, pub can_mention_room: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Suggestion {
    Person { user_id: UserId, name: String },  // name = matrix_name, else name: what is written (D302)
    Everyone,
    Room { alias: RoomAlias },
    Emoji { glyph: String },
    Command { name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeOutcome { ClosedSuggestions, CancelledMode, Nothing }

pub struct ComposerState { /* private */ }
impl ComposerState {
    pub fn new(room: RoomId, me: UserId) -> Self;      // empty, cursor 0, Compose, Enabled, no sources
    pub fn text(&self) -> &str;
    pub fn selection(&self) -> Selection;
    pub fn mode(&self) -> &ComposerMode;
    pub fn access(&self) -> Access;
    pub fn action(&self) -> PrimaryAction;             // Mic when the text is empty, else Send

    /// The user typed: the text and the cursor (selection collapsed) are replaced.
    pub fn set_text(&mut self, text: &str, cursor: usize);
    pub fn select(&mut self, anchor: usize, head: usize);
    /// Replaces the selection by `s`, cursor after it.
    pub fn insert(&mut self, s: &str);
    pub fn set_access(&mut self, access: Access);      // Disabled: set_text, insert, accept, start_* are ignored
    pub fn set_sources(&mut self, sources: Sources);

    pub fn start_reply(&mut self, to: ReplyTarget);    // keeps the text; replaces a previous reply
    /// Text := target.body, cursor at its end, picked := target.mentions; the text typed before is
    /// kept aside.
    pub fn start_edit(&mut self, target: EditTarget);
    /// Reply: back to Compose, text kept. Edit: back to Compose, the text typed before comes back.
    pub fn cancel_mode(&mut self);

    /// The draft to keep: text and picked mentions of the compose text (during an edit, the one
    /// kept aside, never the edited text).
    pub fn draft(&self) -> Draft;
    /// Text := draft.text, cursor at its end, picked := draft.mentions.
    pub fn restore(&mut self, draft: Draft);

    pub fn trigger(&self) -> Option<Trigger>;          // None while dismissed by Escape
    /// For the trigger: Person (match_members) / Room (match_rooms) / Emoji / Command (match_commands),
    /// Everyone among the people as `match_members` places it. Emojis: shortcode equal to the query
    /// first, then the shortcodes it starts, then the others, table order; at most 8.
    /// Empty without trigger, while dismissed, or with no match.
    pub fn suggestions(&self) -> Vec<Suggestion>;
    /// Replaces text[start..cursor] of the trigger by the written text + one space (Person: name;
    /// Room: the alias; Everyone: "@room"; Command: "/name"; Emoji: the glyph, no space needed),
    /// cursor after, the mention picked. No trigger: nothing happens.
    pub fn accept(&mut self, suggestion: Suggestion);
    /// Open suggestions (non-empty) -> close them only (text, banner untouched; stays closed until
    /// the trigger is another one); else a reply/edit -> cancel it; else Nothing.
    pub fn escape(&mut self) -> EscapeOutcome;

    /// Enter. Trimmed text -> Empty if nothing; `commands::prepare` with the commands of the sources;
    /// UnknownCommand keeps everything as is. Ok: the field is emptied (text "", cursor 0, no picked
    /// mention), mode back to Compose (an edit gives back the text kept aside). Reply: relation
    /// Reply, and the replied author joins `mentions` and `notify` unless it is `me`. Edit: relation
    /// Replace; `mentions` = original mentions whose name the text still holds + newly picked;
    /// `notify` = the newly picked only (D487). `formatted_body` = markdown + link_names.
    pub fn submit(&mut self) -> Result<OutgoingMessage, SubmitError>;
}
```

---

## 3. `twake-testing`

```rust
pub struct FakeDraftStore { /* private */ }
impl FakeDraftStore { pub fn new() -> Self; pub fn unavailable(self) -> Self; }  // Default; every call fails
impl DraftStorePort for FakeDraftStore {}
// contract::draft_store(make: impl Fn() -> Arc<dyn DraftStorePort>) -> Verdict
```
