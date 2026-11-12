//! What the room list screen shows: the diffs of the stream, a filter and a search.

use twake_domain::read_state::{UnreadCount, total_unread};
use twake_domain::room_list::{RoomFilter, RoomListDiff, RoomSummary, apply_diff, matches_search};

use crate::ports::RoomListError;

/// What the screen says about the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoomListStatus {
    /// Nothing received yet.
    Loading,
    /// No room, no search.
    Empty,
    /// Rooms exist but none passes the filter and the search.
    NoMatch,
    /// Rooms to show.
    Ready,
    /// The stream failed; the next batch recovers.
    Failed(RoomListError),
}

/// The room list model of a screen.
#[derive(Debug, Default)]
pub struct RoomListState {
    rooms: Vec<RoomSummary>,
    loaded: bool,
    failure: Option<RoomListError>,
    filter: RoomFilter,
    search: String,
}

impl RoomListState {
    /// Loading, filter All, no search.
    pub fn new() -> Self {
        Self::default()
    }

    /// Applies a batch in order; the first one loads the list; clears a failure.
    pub fn apply(&mut self, batch: Vec<RoomListDiff>) {
        for diff in batch {
            self.rooms = apply_diff(&self.rooms, diff);
        }
        self.loaded = true;
        self.failure = None;
    }

    /// Picks a chip.
    pub fn set_filter(&mut self, filter: RoomFilter) {
        self.filter = filter;
    }

    /// The chip.
    pub fn filter(&self) -> RoomFilter {
        self.filter
    }

    /// Sets the search of a room name.
    pub fn set_search(&mut self, query: &str) {
        self.search = query.to_owned();
    }

    /// The search.
    pub fn search(&self) -> &str {
        &self.search
    }

    /// Records a failure; the rooms are kept.
    pub fn fail(&mut self, error: RoomListError) {
        self.failure = Some(error);
    }

    fn shows(&self, room: &RoomSummary) -> bool {
        self.filter.matches(room) && matches_search(&room.name, &self.search)
    }

    /// The rooms to draw: sorted, with the filter and the search applied.
    pub fn view(&self) -> Vec<RoomSummary> {
        self.rooms
            .iter()
            .filter(|r| self.shows(r))
            .cloned()
            .collect()
    }

    /// What the screen says about the list.
    pub fn status(&self) -> RoomListStatus {
        if let Some(error) = &self.failure {
            RoomListStatus::Failed(error.clone())
        } else if !self.loaded {
            RoomListStatus::Loading
        } else if self.rooms.iter().any(|r| self.shows(r)) {
            RoomListStatus::Ready
        } else if self.search.is_empty()
            && (self.filter == RoomFilter::All || self.rooms.is_empty())
        {
            RoomListStatus::Empty
        } else {
            RoomListStatus::NoMatch
        }
    }

    /// The badge: unread of every room, whatever the filter and the search.
    pub fn total_unread(&self) -> UnreadCount {
        let counts: Vec<UnreadCount> = self.rooms.iter().map(|r| r.unread).collect();
        total_unread(&counts)
    }
}
