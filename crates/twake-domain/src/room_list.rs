//! The room list: summaries, their order, the filters, the search and the diffs of the stream.
//!
//! Implements D190 (order), D191/D109/D286 (chips), D247 (pin = favourite), D392 (search).

use std::cmp::Reverse;

use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

use crate::read_state::UnreadCount;
use crate::{MxcUri, RoomId, Timestamp};

/// What a room is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoomKind {
    /// A conversation with one person.
    Direct,
    /// A group chat.
    Group,
    /// A space, which contains rooms.
    Space,
    /// A room where only moderators post.
    Broadcast,
    /// A voice room.
    Voice,
}

/// One row of the room list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomSummary {
    /// The room.
    pub id: RoomId,
    /// Its name.
    pub name: String,
    /// Its picture.
    pub avatar: Option<MxcUri>,
    /// What it is.
    pub kind: RoomKind,
    /// The last message as readable text.
    pub last_message_preview: Option<String>,
    /// The last message, else the creation (D101).
    pub last_activity: Timestamp,
    /// What it shows as unread.
    pub unread: UnreadCount,
    /// Pinned: the favourite tag (D247).
    pub is_favourite: bool,
    /// Listed after the others (D190).
    pub is_low_priority: bool,
    /// The space it sits in.
    pub space: Option<RoomId>,
}

impl RoomSummary {
    /// A quiet, unflagged room with no picture and no message.
    pub fn new(id: RoomId, name: impl Into<String>, kind: RoomKind) -> Self {
        Self {
            id,
            name: name.into(),
            avatar: None,
            kind,
            last_message_preview: None,
            last_activity: Timestamp::from_millis(0),
            unread: UnreadCount {
                unread: 0,
                mentions: 0,
                marked_unread: false,
            },
            is_favourite: false,
            is_low_priority: false,
            space: None,
        }
    }

    /// Sets the picture.
    pub fn with_avatar(mut self, avatar: MxcUri) -> Self {
        self.avatar = Some(avatar);
        self
    }

    /// Sets the preview of the last message.
    pub fn with_last_message_preview(mut self, text: impl Into<String>) -> Self {
        self.last_message_preview = Some(text.into());
        self
    }

    /// Sets the last activity.
    pub fn with_last_activity(mut self, ts: Timestamp) -> Self {
        self.last_activity = ts;
        self
    }

    /// Sets the unread count.
    pub fn with_unread(mut self, unread: UnreadCount) -> Self {
        self.unread = unread;
        self
    }

    /// Pins the room.
    pub fn favourite(mut self) -> Self {
        self.is_favourite = true;
        self
    }

    /// Lowers the room's priority.
    pub fn low_priority(mut self) -> Self {
        self.is_low_priority = true;
        self
    }

    /// Puts the room in a space.
    pub fn in_space(mut self, space: RoomId) -> Self {
        self.space = Some(space);
        self
    }
}

/// Orders the list (D190): not low priority first, favourites first, newest activity first;
/// stable, so equal rooms keep their given order.
pub fn sort_rooms(rooms: &mut [RoomSummary]) {
    rooms.sort_by_key(|r| (r.is_low_priority, !r.is_favourite, Reverse(r.last_activity)));
}

/// The chips above the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoomFilter {
    /// Every room.
    #[default]
    All,
    /// Rooms with something unread, never a space (D286).
    Unread,
    /// Broadcast rooms.
    Broadcasts,
    /// Direct rooms.
    Directs,
    /// A space and the rooms inside a space.
    Spaces,
    /// Pinned rooms.
    Favourites,
}

fn is_unread_outside_spaces(room: &RoomSummary) -> bool {
    room.unread.is_unread() && room.kind != RoomKind::Space
}

fn is_in_spaces(room: &RoomSummary) -> bool {
    room.kind == RoomKind::Space || room.space.is_some()
}

impl RoomFilter {
    /// Whether `room` shows under this chip.
    pub fn matches(&self, room: &RoomSummary) -> bool {
        match self {
            Self::All => true,
            Self::Unread => is_unread_outside_spaces(room),
            Self::Broadcasts => room.kind == RoomKind::Broadcast,
            Self::Directs => room.kind == RoomKind::Direct,
            Self::Spaces => is_in_spaces(room),
            Self::Favourites => room.is_favourite,
        }
    }
}

/// Lower case, no accent (D392).
fn fold(s: &str) -> String {
    s.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// `word` sits in `name` between two word boundaries.
fn contains_whole_word(name: &str, word: &str) -> bool {
    name.match_indices(word).any(|(start, found)| {
        let before = name.get(..start).and_then(|s| s.chars().next_back());
        let after = name
            .get(start + found.len()..)
            .and_then(|s| s.chars().next());
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

/// Search of a room name (D392): case and accents ignored; a query ending with a space matches
/// as a whole word, otherwise as a substring; an empty query matches.
pub fn matches_search(name: &str, query: &str) -> bool {
    let (name, query) = (fold(name), fold(query));
    let word = query.trim_end();
    if word.len() == query.len() {
        name.contains(&query)
    } else {
        word.is_empty() || contains_whole_word(&name, word)
    }
}

/// A change of the room list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomListDiff {
    /// The whole list; the same id twice keeps the last.
    Reset(Vec<RoomSummary>),
    /// A room; a known id is replaced.
    Insert(RoomSummary),
    /// Drops a room; an unknown id is ignored.
    Remove(RoomId),
    /// Replaces a room by id; an unknown id is ignored.
    Update(RoomSummary),
    /// A new activity for a room; an unknown id is ignored.
    Move {
        /// The room.
        id: RoomId,
        /// Its new last activity.
        last_activity: Timestamp,
    },
}

fn upsert(list: &mut Vec<RoomSummary>, room: RoomSummary) {
    match list.iter_mut().find(|r| r.id == room.id) {
        Some(known) => *known = room,
        None => list.push(room),
    }
}

/// Applies a diff to a list; the result is keyed by id and sorted by [`sort_rooms`].
pub fn apply_diff(list: &[RoomSummary], diff: RoomListDiff) -> Vec<RoomSummary> {
    let mut rooms = list.to_vec();
    match diff {
        RoomListDiff::Reset(all) => {
            rooms.clear();
            all.into_iter().for_each(|r| upsert(&mut rooms, r));
        }
        RoomListDiff::Insert(room) => upsert(&mut rooms, room),
        RoomListDiff::Remove(id) => rooms.retain(|r| r.id != id),
        RoomListDiff::Update(room) => {
            if let Some(known) = rooms.iter_mut().find(|r| r.id == room.id) {
                *known = room;
            }
        }
        RoomListDiff::Move { id, last_activity } => {
            if let Some(known) = rooms.iter_mut().find(|r| r.id == id) {
                known.last_activity = last_activity;
            }
        }
    }
    sort_rooms(&mut rooms);
    rooms
}
