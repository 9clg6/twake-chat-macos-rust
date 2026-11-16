# API sketch required by the room list and bubble view-model tests

Written by the test author from the need, before the production code exists. The need:
`ARCHITECTURE.md` (`twake-ui` = screens, calls use cases only; `twake-i18n` = Fluent, A006; lists are
virtualised, section 8), `docs/spec/features/roomlist.md` (D31, D175, D190, D219, D222, D608, D615),
`docs/spec/features/timeline.md` (D297, D378, D470, D495, D500, D519, D523, D138), and the React
`getInitials` / `nameToColor` of twake-mui. Only the pure half: no window, no GPUI type.

Tests (all red: the modules do not exist, so they fail to compile):
`crates/twake-ui/tests/{room_row,bubble,dividers,virtual_list}.rs` (+ `tests/common/mod.rs`, the
fixture catalogues) and `crates/twake-i18n/tests/locale.rs`. The only dev-dependency added is
`proptest` on `twake-ui`.

## 0. `twake_i18n::Locale` (new, A006)

```rust
pub enum Arg<'a> { Str(&'a str), Num(i64) }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HourCycle { H24, H12 }                       // default H24

pub struct Locale;
impl Locale {
    /// One Fluent catalogue (`ftl` text) for `lang` (BCP 47), a reader `utc_offset_secs` east of UTC.
    pub fn from_ftl(lang: &str, ftl: &str, utc_offset_secs: i32) -> Locale;
    pub fn with_hour_cycle(self, cycle: HourCycle) -> Locale;
    pub fn utc_offset_secs(&self) -> i32;
    pub fn hour_cycle(&self) -> HourCycle;
    /// The message `key` with its arguments; an unknown key gives the key itself.
    /// No Unicode isolation marks around arguments (`use_isolating(false)`).
    pub fn tr(&self, key: &str, args: &[(&str, Arg)]) -> String;
}
```

The catalogue is a value, so the tests use fixture catalogues (`en`, `fr`) and no file on disk. Locale
detection and the bundled real catalogues come later (YAGNI).

## 1. Keys (Fluent)

| Key | Args | en fixture | fr fixture |
|---|---|---|---|
| `room-preview-you` | `text` | `You: { $text }` | `Vous : { $text }` |
| `room-preview-author` | `author`, `text` | `{ $author }: { $text }` | `{ $author } : { $text }` |
| `time-yesterday` | - | `Yesterday` | `Hier` |
| `weekday-short-mon` ... `-sun` | - | `Mon` ... `Sun` | `lun.` ... `dim.` |
| `date-short` | `day`, `month`, `year` (2-digit, zero padded strings) | `{ $month }/{ $day }/{ $year }` | `{ $day }/{ $month }/{ $year }` |
| `time-12h` | `hour` (1 to 12, Num), `minute` (2-digit Str), `period` | `{ $hour }:{ $minute } { $period }` | same |
| `period-am` / `period-pm` | - | `AM` / `PM` | `AM` / `PM` |
| `day-today` | - | `Today` | `Aujourd'hui` |
| `day-yesterday` | - | `Yesterday` | `Hier` |
| `new-messages` | - | `New messages` | `Nouveaux messages` |
| `message-edited` | - | `(edited)` | `(modifié)` |

24 hour times are built by the code (`14:05`, zero padded), they need no key. Day dividers older than
yesterday use `date-short`.

## 2. Domain additions (`twake-domain`, `room_list`)

The summary did not say who wrote the last message nor whether the room is muted; both are needed by the
row (D31, D175, D187).

```rust
pub enum LastSender { Me, Other(String /* display name, D596 */) }
// RoomSummary gains:  pub last_sender: Option<LastSender>,  pub is_muted: bool
impl RoomSummary {
    pub fn with_last_sender(self, sender: LastSender) -> Self;
    pub fn muted(self) -> Self;
}
```
`RoomSummary::new` keeps `None` / `false`.

## 3. `twake_ui::avatar`

```rust
pub const AVATAR_COLOR_COUNT: usize = 11;              // the 11 colours of twake-mui `supportedColors`
pub struct AvatarView { pub initials: String, pub color_index: usize }
impl AvatarView { pub fn of(name: &str, id: &str) -> AvatarView }
```
- `initials`: the first letter of the first and of the last word, upper case; one word gives its first two
  letters; cut by grapheme; a word that starts with neither a letter nor a digit (an emoji) is skipped unless
  nothing else remains; a Matrix id `@john.doe:server` reads as `john doe`; empty name gives `""`.
- `color_index`: `< AVATAR_COLOR_COUNT`, a function of `id` only (stable when the room is renamed).

## 4. `twake_ui::room_row`

```rust
pub struct RoomRowView {
    pub title: String,
    pub preview: Option<String>,       // None: no message, or a space (D170)
    pub time: Option<String>,          // None while the badge shows (D608)
    pub badge: Option<BadgeView>,
    pub avatar: AvatarView,            // of (name, room id)
    pub is_pinned: bool,               // favourite (D247)
    pub is_muted: bool,
}
pub struct BadgeView { pub text: String, pub style: BadgeStyle }   // text "" = a dot
pub enum BadgeStyle { Normal, Mention, Muted }
impl RoomRowView {
    pub fn from(room: &RoomSummary, now: Timestamp, locale: &Locale) -> RoomRowView;
}
```
Rules:
- Preview: a direct room shows the text alone (D175: no "You" in a direct row); any other kind but space
  shows `room-preview-you` for `LastSender::Me`, `room-preview-author` for `Other(name)`, the text alone when
  the sender is unknown.
- Time of `last_activity`, in the reader's offset: same day as `now` the clock (`14:05`, or `time-12h`);
  the day before `time-yesterday`; 2 to 6 days before `weekday-short-*`; 7 days and more `date-short`.
- Badge: none when nothing is unread; mentions > 0 gives `@` ; else the count, `99` at most shown as is,
  100 and more `99+`; the mark alone gives an empty text (dot). Style `Muted` when the room is muted, else
  `Mention` when there is a mention, else `Normal`.

## 5. `twake_ui::timeline_rows`

```rust
pub const GROUP_GAP_MS: u64 = 300_000;       // 5 minutes: a decision, the spec is silent (open question 2)
pub const QUOTE_MAX_CHARS: usize = 120;

pub struct TimelineContext<'a> {
    pub me: &'a UserId,
    pub is_direct: bool,
    pub names: &'a HashMap<UserId, String>,            // display names; unknown: the Matrix id
    pub read_by_others_up_to: Option<&'a EventId>,     // newest event any other member has read
}
pub enum Side { Own, Other }
pub enum Ticks { Clock, Sent, Read, Failed }           // D495, D470
pub struct QuoteView { pub target: EventId, pub author: String, pub text: String }

pub struct BubbleView {
    pub side: Side,
    pub author: String,                // display name
    pub avatar: AvatarView,            // of (author, user id)
    pub shows_name: bool,              // first of a run, from someone else, not a direct room (D500, D297)
    pub shows_avatar: bool,            // last of a run, from someone else (D500; direct too, D519)
    pub body: String,
    pub time: String,                  // clock in the reader's offset
    pub ticks: Option<Ticks>,          // own messages only
    pub edited: Option<String>,        // `message-edited`
    pub quote: Option<QuoteView>,
}
pub struct DayDividerView { pub label: String }
pub struct NewMessagesView { pub label: String }
pub enum RowView { Day(DayDividerView), NewMessages(NewMessagesView), Bubble(BubbleView) }

impl DayDividerView   { pub fn from(day: Day, now: Timestamp, locale: &Locale) -> Self; }
impl NewMessagesView  { pub fn from(locale: &Locale) -> Self; }
pub fn timeline_rows(items: &[TimelineItem], ctx: &TimelineContext, now: Timestamp, locale: &Locale)
    -> Vec<RowView>;       // one row per item, same order
```
Rules:
- A run: consecutive messages of one sender, each at most `GROUP_GAP_MS` after the one before; a divider or
  the new-messages marker between them ends it.
- Ticks (own messages): `Sending` clock, `Sent` one tick, `Sent` and at or before `read_by_others_up_to`
  in the list two ticks `Read`, `Failed` `Failed`. Someone else's message: none.
- A quote is the replied message when it is in `items`: its author, its text cut to `QUOTE_MAX_CHARS`
  characters (the last one `…` when cut).
- Day divider: today `day-today`, the day before `day-yesterday` (across month, year and leap days),
  else `date-short`.

## 6. `twake_ui::virtual_list`

```rust
pub struct HeightIndex;                          // prefix sums, built once per change of the list
impl HeightIndex {
    pub fn new(item_heights: &[f32]) -> HeightIndex;
    pub fn len(&self) -> usize;
    pub fn total_height(&self) -> f32;
    pub fn offset(&self, index: usize) -> f32;   // top of item `index`; `total_height` for `len`
    /// Items that meet [scroll_top, scroll_top + viewport), plus `overscan` items on each side,
    /// clamped to 0..len. A negative `scroll_top` (rubber band) counts as 0.
    pub fn visible_range(&self, scroll_top: f32, viewport: f32, overscan: usize) -> Range<usize>;
}
```
Deviation from the request (free function `visible_range(scroll_top, viewport, item_heights, overscan)`):
a free function over the slice sums 10 000 heights on every frame; the index answers in O(log n).
Item `i` meets the window when `top_i < end` and `bottom_i > start`. Heights are positive.

## Open questions (the need is silent; the tests pin the proposal above)

1. The summary has no last sender nor muted flag (section 2). If the domain refuses, the same two inputs
   come as arguments of `RoomRowView::from`; the tests change in one helper.
2. Grouping delay of bubbles: the spec gives 1 minute for image albums only (D89). 5 minutes proposed.
3. Avatar colour: requested "from the id"; twake-mui `nameToColor` hashes the name (sum of upper-case
   char codes modulo 11). A renamed room would change colour. The tests accept any function of the id.
4. "This week": rolling 6 days proposed, not the calendar week.
5. Run avatar: the request said author and avatar at the head of the run; D500/D519 put the name on the
   first bubble and the avatar on the last. The spec is followed (two flags).
6. A receipt on an event not in `items`: not tested (no reader assumed).
7. A reply whose target is not loaded: not tested.
8. Accessible names (`unread-badge`, D378 message name) are not part of this slice.
