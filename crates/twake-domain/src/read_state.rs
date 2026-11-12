//! Unread counts and read receipts as pure functions over immutable values
//! (port of the React `src/domain/readState.ts`; decisions D60, D175, D222,
//! D483, D510, D539, D558, D615).

use std::collections::HashSet;

use crate::{EventId, UserId};

/// What an event is, as far as the count is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// A shown message (text, media, poll, missed call...).
    Message { mentions_me: bool },
    /// A reaction, an edit, a redaction, a state event: never counted.
    Other,
}

/// An event of a timeline, reduced to what the count needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEvent {
    /// The event id.
    pub id: EventId,
    /// Who sent it.
    pub sender: UserId,
    kind: Kind,
}

impl TimelineEvent {
    /// A shown message.
    pub fn message(id: EventId, sender: UserId) -> Self {
        Self {
            id,
            sender,
            kind: Kind::Message { mentions_me: false },
        }
    }

    /// An event that is not a message: it is never counted.
    pub fn other(id: EventId, sender: UserId) -> Self {
        Self {
            id,
            sender,
            kind: Kind::Other,
        }
    }

    /// Marks a message as mentioning the user; no effect on other events.
    pub fn mentioning_me(mut self) -> Self {
        if let Kind::Message { mentions_me } = &mut self.kind {
            *mentions_me = true;
        }
        self
    }

    fn is_message(&self) -> bool {
        matches!(self.kind, Kind::Message { .. })
    }

    fn mentions_me(&self) -> bool {
        matches!(self.kind, Kind::Message { mentions_me: true })
    }
}

/// Who the user is, what they have read, whom they ignore.
#[derive(Debug, Clone)]
pub struct ReadContext {
    me: UserId,
    receipts: HashSet<EventId>,
    ignored: HashSet<UserId>,
}

impl ReadContext {
    /// A context for `me`, with no receipt and nobody ignored.
    pub fn new(me: UserId) -> Self {
        Self {
            me,
            receipts: HashSet::new(),
            ignored: HashSet::new(),
        }
    }

    /// Adds all own receipts (public and private, in any order).
    pub fn with_receipts(mut self, receipts: Vec<EventId>) -> Self {
        self.receipts.extend(receipts);
        self
    }

    /// Ignores a person; callable several times.
    pub fn ignoring(mut self, user: UserId) -> Self {
        self.ignored.insert(user);
        self
    }

    /// Does this event move the read point: an own message, or one with a receipt.
    fn reads_up_to(&self, event: &TimelineEvent) -> bool {
        (event.is_message() && event.sender == self.me) || self.receipts.contains(&event.id)
    }

    fn counts(&self, event: &TimelineEvent) -> bool {
        event.is_message() && event.sender != self.me && !self.ignored.contains(&event.sender)
    }
}

/// Unread state of one timeline (the main one, or a thread).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineReadState {
    /// Messages of others after the read point.
    pub unread_count: u32,
    /// Those that mention the user (a subset of `unread_count`).
    pub mention_count: u32,
    /// The last message of the slice, own and ignored included: where a receipt goes.
    pub latest_event_id: Option<EventId>,
}

impl TimelineReadState {
    fn has_unread(&self) -> bool {
        self.unread_count > 0 || self.mention_count > 0
    }
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The unread state of `events`, oldest first.
///
/// The read point is the furthest of the user's last own message and every
/// receipt on an event held in `events`; a receipt on an unheld event counts for nothing.
pub fn timeline_read_state(events: &[TimelineEvent], ctx: &ReadContext) -> TimelineReadState {
    let first_unread = events
        .iter()
        .rposition(|e| ctx.reads_up_to(e))
        .map_or(0, |i| i + 1);
    let unread: Vec<&TimelineEvent> = events
        .iter()
        .skip(first_unread)
        .filter(|e| ctx.counts(e))
        .collect();
    TimelineReadState {
        unread_count: to_u32(unread.len()),
        mention_count: to_u32(unread.iter().filter(|e| e.mentions_me()).count()),
        latest_event_id: events
            .iter()
            .rfind(|e| e.is_message())
            .map(|e| e.id.clone()),
    }
}

/// Unread state of one thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadReadState {
    /// The root event of the thread.
    pub root: EventId,
    /// Raw counts (kept when the thread is not followed), zero when muted.
    pub state: TimelineReadState,
    /// The user follows the thread or takes part in it.
    pub is_participating: bool,
    /// The thread is muted.
    pub is_muted: bool,
}

/// The unread state of a thread; `replies` oldest first, root not included.
pub fn thread_read_state(
    root: EventId,
    replies: &[TimelineEvent],
    is_participating: bool,
    is_muted: bool,
    ctx: &ReadContext,
) -> ThreadReadState {
    let mut state = timeline_read_state(replies, ctx);
    if is_muted {
        state.unread_count = 0;
        state.mention_count = 0;
    }
    ThreadReadState {
        root,
        state,
        is_participating,
        is_muted,
    }
}

/// Unread state of a room: its main timeline and its threads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomReadState {
    /// The main timeline.
    pub main: TimelineReadState,
    /// The threads of the room.
    pub threads: Vec<ThreadReadState>,
}

/// How much a room may notify.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    /// Every message.
    All,
    /// Mentions only.
    MentionsOnly,
    /// Nothing.
    Nothing,
}

/// The user's own settings of a room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomFlags {
    /// What the room may notify.
    pub notification_level: NotificationLevel,
    /// The user marked the room unread by hand.
    pub marked_unread: bool,
}

/// What a room row or a badge shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnreadCount {
    /// Unread messages.
    pub unread: u32,
    /// Mentions.
    pub mentions: u32,
    /// A mark without a count (D615).
    pub marked_unread: bool,
}

impl UnreadCount {
    /// Anything to show: a count, a mention or the mark.
    pub fn is_unread(&self) -> bool {
        self.unread > 0 || self.mentions > 0 || self.marked_unread
    }
}

/// Unread messages of the main timeline and of the threads the user follows.
fn all_unread(state: &RoomReadState) -> u32 {
    state
        .threads
        .iter()
        .filter(|t| t.is_participating)
        .fold(state.main.unread_count, |sum, t| {
            sum.saturating_add(t.state.unread_count)
        })
}

/// The count of a room under the user's flags.
///
/// Mentions count in every thread, followed or not. The mark shows only when no count does.
pub fn room_unread(state: &RoomReadState, flags: &RoomFlags) -> UnreadCount {
    let mentions = state
        .threads
        .iter()
        .fold(state.main.mention_count, |sum, t| {
            sum.saturating_add(t.state.mention_count)
        });
    let (unread, mentions) = match flags.notification_level {
        NotificationLevel::All => (all_unread(state), mentions),
        NotificationLevel::MentionsOnly => (0, mentions),
        NotificationLevel::Nothing => (0, 0),
    };
    UnreadCount {
        unread,
        mentions,
        marked_unread: flags.marked_unread && unread == 0 && mentions == 0,
    }
}

/// The sum of rooms for a badge; a mark is no count.
pub fn total_unread(counts: &[UnreadCount]) -> UnreadCount {
    counts.iter().fold(
        UnreadCount {
            unread: 0,
            mentions: 0,
            marked_unread: false,
        },
        |total, c| UnreadCount {
            unread: total.unread.saturating_add(c.unread),
            mentions: total.mentions.saturating_add(c.mentions),
            marked_unread: false,
        },
    )
}

/// For a mentions-only room, the messages shown in grey and counted by no total.
pub fn silent_unread(state: &RoomReadState, flags: &RoomFlags) -> u32 {
    match flags.notification_level {
        NotificationLevel::MentionsOnly => all_unread(state),
        NotificationLevel::All | NotificationLevel::Nothing => 0,
    }
}

/// Which timeline a receipt is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadId {
    /// The main timeline.
    Main,
    /// The thread with this root.
    Thread(EventId),
}

/// A receipt to send.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptTarget {
    /// The timeline.
    pub thread: ThreadId,
    /// The event read up to.
    pub event: EventId,
}

/// The receipts that mark a room read: the main timeline, then every thread with
/// something unread, followed or not.
pub fn receipt_targets(state: &RoomReadState) -> Vec<ReceiptTarget> {
    let main = (ThreadId::Main, &state.main);
    let threads = state
        .threads
        .iter()
        .map(|t| (ThreadId::Thread(t.root.clone()), &t.state));
    std::iter::once(main)
        .chain(threads)
        .filter(|(_, s)| s.has_unread())
        .filter_map(|(thread, s)| {
            s.latest_event_id
                .clone()
                .map(|event| ReceiptTarget { thread, event })
        })
        .collect()
}

/// The event a receipt may point at: `shown` if it is among the last `unread_count`
/// ids held (so a receipt never goes back), else nothing.
pub fn choose_receipt_event(
    ordered_ids: &[EventId],
    unread_count: u32,
    shown: Option<&EventId>,
) -> Option<EventId> {
    let shown = shown.filter(|_| unread_count > 0)?;
    let tail = usize::try_from(unread_count)
        .unwrap_or(usize::MAX)
        .min(ordered_ids.len());
    let position = ordered_ids.iter().rposition(|id| id == shown)?;
    (position >= ordered_ids.len() - tail).then(|| shown.clone())
}
