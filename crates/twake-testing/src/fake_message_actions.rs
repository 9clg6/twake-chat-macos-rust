//! A server for reactions, pins and reports with no network: pins in memory, one list per room.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::FutureExt;
use futures::future::BoxFuture;
use twake_app::ports::{ActionError, MessageActionsPort};
use twake_domain::pinned::PinnedState;
use twake_domain::{EventId, RoomId};

use crate::subscribers::locked;

/// Holds the pins of the rooms it knows; `forbidding` and `unavailable` make the calls fail.
#[derive(Default)]
pub struct FakeMessageActions {
    rooms: Mutex<HashMap<RoomId, PinnedState>>,
    reactions: AtomicUsize,
    forbidden: bool,
    unreachable: bool,
}

impl FakeMessageActions {
    /// A server that knows no room.
    pub fn new() -> Self {
        Self::default()
    }

    /// Knows `room`, with no pin.
    pub fn with_room(self, room: RoomId) -> Self {
        locked(&self.rooms).insert(room, PinnedState::new());
        self
    }

    /// Every change is refused with `Forbidden`; reading the pins still works.
    pub fn forbidding(mut self) -> Self {
        self.forbidden = true;
        self
    }

    /// Every call returns `Unavailable`.
    pub fn unavailable(mut self) -> Self {
        self.unreachable = true;
        self
    }

    /// The checks come in this order: unreachable, unknown room, forbidden.
    fn check<T>(
        &self,
        room: &RoomId,
        change: bool,
        action: impl FnOnce(&mut PinnedState) -> Result<T, ActionError>,
    ) -> Result<T, ActionError> {
        if self.unreachable {
            return Err(ActionError::Unavailable);
        }
        let mut rooms = locked(&self.rooms);
        let pins = rooms.get_mut(room).ok_or(ActionError::RoomNotFound)?;
        if change && self.forbidden {
            return Err(ActionError::Forbidden);
        }
        action(pins)
    }

    fn run<T: Send + 'static>(
        &self,
        room: &RoomId,
        change: bool,
        action: impl FnOnce(&mut PinnedState) -> T,
    ) -> BoxFuture<'_, Result<T, ActionError>> {
        let result = self.check(room, change, |pins| Ok(action(pins)));
        std::future::ready(result).boxed()
    }
}

impl MessageActionsPort for FakeMessageActions {
    fn send_reaction(
        &self,
        room: RoomId,
        _target: EventId,
        _key: String,
    ) -> BoxFuture<'_, Result<EventId, ActionError>> {
        let n = self.reactions.fetch_add(1, Ordering::Relaxed);
        let result = self.check(&room, true, |_| {
            EventId::parse(&format!("$reaction{n}")).map_err(|_| ActionError::Unavailable)
        });
        std::future::ready(result).boxed()
    }

    fn redact(&self, room: RoomId, _event: EventId) -> BoxFuture<'_, Result<(), ActionError>> {
        self.run(&room, true, |_| ())
    }

    fn pin(&self, room: RoomId, event: EventId) -> BoxFuture<'_, Result<(), ActionError>> {
        self.run(&room, true, |pins| pins.pin(event))
    }

    fn unpin(&self, room: RoomId, event: EventId) -> BoxFuture<'_, Result<(), ActionError>> {
        self.run(&room, true, |pins| pins.unpin(&event))
    }

    fn pinned(&self, room: RoomId) -> BoxFuture<'_, Result<Vec<EventId>, ActionError>> {
        self.run(&room, false, |pins| pins.ids().to_vec())
    }

    fn report(
        &self,
        room: RoomId,
        _event: EventId,
        _reason: Option<String>,
    ) -> BoxFuture<'_, Result<(), ActionError>> {
        self.run(&room, true, |_| ())
    }
}
