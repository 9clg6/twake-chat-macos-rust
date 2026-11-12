//! A conversation port with no network: echoes, confirmations and failures are driven by the test.

use std::collections::HashMap;
use std::sync::Mutex;

use futures::FutureExt;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use twake_app::ports::{TIMELINE_PAGE_SIZE, TimelineError, TimelinePort};
use twake_domain::timeline::{Message, TimelineDiff};
use twake_domain::{EventId, RoomId, Timestamp, TxnId, UserId};

use crate::subscribers::{Subscribers, locked};

struct Room {
    history: Vec<Message>,
    /// Index in `history` of the oldest message shown.
    window_start: usize,
    pending: Vec<Message>,
    subscribers: Subscribers<TimelineDiff>,
}

struct Inner {
    rooms: HashMap<RoomId, Room>,
    sent: usize,
}

/// Rooms with a history; the user is `me`, and every echo and confirmation is stamped `now`.
pub struct FakeTimeline {
    me: UserId,
    now: Timestamp,
    inner: Mutex<Inner>,
    failure: Option<TimelineError>,
}

impl FakeTimeline {
    /// No room yet.
    pub fn new(me: UserId, now: Timestamp) -> Self {
        Self {
            me,
            now,
            inner: Mutex::new(Inner {
                rooms: HashMap::new(),
                sent: 0,
            }),
            failure: None,
        }
    }

    /// A room whose whole history is `history`, oldest first.
    pub fn with_room(self, room: RoomId, history: Vec<Message>) -> Self {
        let state = Room {
            history,
            window_start: 0,
            pending: Vec::new(),
            subscribers: Subscribers::new(),
        };
        locked(&self.inner).rooms.insert(room, state);
        self
    }

    /// Makes `open`, `paginate_back` and `send_text` return `error`.
    pub fn failing(mut self, error: TimelineError) -> Self {
        self.failure = Some(error);
        self
    }

    /// One batch to the subscribers of `room` only.
    pub fn push(&self, room: &RoomId, diffs: Vec<TimelineDiff>) {
        if let Some(state) = locked(&self.inner).rooms.get_mut(room) {
            state.subscribers.send(&diffs);
        }
    }

    /// One batch: the server event of each pending send, in send order; they join the history.
    pub fn confirm_sends(&self, room: &RoomId) {
        let mut inner = locked(&self.inner);
        let Inner { rooms, sent } = &mut *inner;
        let Some(state) = rooms.get_mut(room) else {
            return;
        };
        let confirmed: Vec<Message> = std::mem::take(&mut state.pending)
            .into_iter()
            .filter_map(|echo| {
                *sent += 1;
                let event = EventId::parse(&format!("$fake-confirmed-{sent}")).ok()?;
                let txn = echo.txn_id()?.clone();
                Some(
                    Message::remote(event, echo.sender, echo.timestamp, echo.body).with_txn_id(txn),
                )
            })
            .collect();
        if confirmed.is_empty() {
            return;
        }
        state.history.extend(confirmed.iter().cloned());
        state.subscribers.send(&[TimelineDiff::Append(confirmed)]);
    }

    /// One batch: each pending send marked failed; they do not join the history.
    pub fn fail_sends(&self, room: &RoomId) {
        let mut inner = locked(&self.inner);
        let Some(state) = inner.rooms.get_mut(room) else {
            return;
        };
        let failed: Vec<TimelineDiff> = std::mem::take(&mut state.pending)
            .into_iter()
            .map(|echo| TimelineDiff::Update(echo.failed()))
            .collect();
        state.subscribers.send(&failed);
    }

    fn check(&self, room: &RoomId) -> Result<(), TimelineError> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if locked(&self.inner).rooms.contains_key(room) {
            Ok(())
        } else {
            Err(TimelineError::RoomNotFound)
        }
    }

    fn open_now(
        &self,
        room: &RoomId,
    ) -> Result<BoxStream<'static, Vec<TimelineDiff>>, TimelineError> {
        self.check(room)?;
        let mut inner = locked(&self.inner);
        let state = inner
            .rooms
            .get_mut(room)
            .ok_or(TimelineError::RoomNotFound)?;
        state.window_start = state.history.len().saturating_sub(TIMELINE_PAGE_SIZE);
        let first = TimelineDiff::Reset(state.history[state.window_start..].to_vec());
        Ok(state.subscribers.join(vec![first]))
    }

    fn paginate_now(&self, room: &RoomId) -> Result<(), TimelineError> {
        self.check(room)?;
        let mut inner = locked(&self.inner);
        let state = inner
            .rooms
            .get_mut(room)
            .ok_or(TimelineError::RoomNotFound)?;
        let start = state.window_start.saturating_sub(TIMELINE_PAGE_SIZE);
        let page = TimelineDiff::Prepend {
            messages: state.history[start..state.window_start].to_vec(),
            reached_start: start == 0,
        };
        state.window_start = start;
        state.subscribers.send(&[page]);
        Ok(())
    }

    fn send_now(&self, room: &RoomId, body: String) -> Result<(), TimelineError> {
        self.check(room)?;
        let mut inner = locked(&self.inner);
        inner.sent += 1;
        let txn = TxnId::parse(&format!("fake-txn-{}", inner.sent))
            .map_err(|_| TimelineError::Unavailable)?;
        let echo = Message::local(txn, self.me.clone(), self.now, body);
        let state = inner
            .rooms
            .get_mut(room)
            .ok_or(TimelineError::RoomNotFound)?;
        state.pending.push(echo.clone());
        state.subscribers.send(&[TimelineDiff::Append(vec![echo])]);
        Ok(())
    }
}

impl TimelinePort for FakeTimeline {
    fn open(
        &self,
        room: RoomId,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Vec<TimelineDiff>>, TimelineError>> {
        std::future::ready(self.open_now(&room)).boxed()
    }

    fn paginate_back(&self, room: RoomId) -> BoxFuture<'_, Result<(), TimelineError>> {
        std::future::ready(self.paginate_now(&room)).boxed()
    }

    fn send_text(&self, room: RoomId, body: String) -> BoxFuture<'_, Result<(), TimelineError>> {
        std::future::ready(self.send_now(&room, body)).boxed()
    }
}
