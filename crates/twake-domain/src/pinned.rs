//! The pinned messages of a room: `m.room.pinned_events` (D229, D260).

use crate::EventId;

/// The pinned events of a room, in the order of the state event (oldest pin first).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PinnedState {
    ids: Vec<EventId>,
}

/// The pin the banner shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinnedBanner {
    /// The pinned message shown.
    pub event: EventId,
    /// Its place in the banner, from 1.
    pub position: usize,
    /// How many pins the room has.
    pub total: usize,
}

impl PinnedState {
    /// No pin.
    pub fn new() -> Self {
        Self::default()
    }

    /// The pins of a state event; a duplicate is dropped, the first kept.
    pub fn from_ids(ids: Vec<EventId>) -> Self {
        let mut state = Self::new();
        state.pin_all(&ids);
        state
    }

    /// The pins, oldest first.
    pub fn ids(&self) -> &[EventId] {
        &self.ids
    }

    /// Whether `id` is pinned.
    pub fn contains(&self, id: &EventId) -> bool {
        self.ids.contains(id)
    }

    /// Pins `id`, last; nothing if it is already pinned.
    pub fn pin(&mut self, id: EventId) {
        if !self.contains(&id) {
            self.ids.push(id);
        }
    }

    /// Unpins `id`, the others stay (D229); an unknown id is fine.
    pub fn unpin(&mut self, id: &EventId) {
        self.ids.retain(|e| e != id);
    }

    /// Pins several at once, in the order given (D260).
    pub fn pin_all(&mut self, ids: &[EventId]) {
        for id in ids {
            self.pin(id.clone());
        }
    }

    /// Unpins several at once.
    pub fn unpin_all(&mut self, ids: &[EventId]) {
        self.ids.retain(|e| !ids.contains(e));
    }

    /// The banner shows one pin at a time, the newest first; `shown` counts the clicks, each
    /// shows the next pin and the last wraps to the first (D229). `None` when nothing is pinned.
    pub fn banner(&self, shown: usize) -> Option<PinnedBanner> {
        let total = self.ids.len();
        let position = shown.checked_rem(total)?;
        let event = self.ids.get(total - 1 - position)?.clone();
        Some(PinnedBanner {
            event,
            position: position + 1,
            total,
        })
    }
}
