//! A room list port with no network: the test pushes what the server would send.

use std::sync::Mutex;

use futures::FutureExt;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use twake_app::ports::{RoomListError, RoomListPort};
use twake_domain::room_list::{RoomListDiff, RoomSummary, apply_diff};

use crate::subscribers::{Subscribers, locked};

struct Inner {
    rooms: Vec<RoomSummary>,
    subscribers: Subscribers<RoomListDiff>,
}

/// Holds the rooms of an account; every push is applied to them and sent to every subscriber.
pub struct FakeRoomList {
    inner: Mutex<Inner>,
    failure: Option<RoomListError>,
}

impl FakeRoomList {
    /// An account holding `rooms`.
    pub fn new(rooms: Vec<RoomSummary>) -> Self {
        Self {
            inner: Mutex::new(Inner {
                rooms: apply_diff(&[], RoomListDiff::Reset(rooms)),
                subscribers: Subscribers::new(),
            }),
            failure: None,
        }
    }

    /// Makes `subscribe` return `error`.
    pub fn failing(mut self, error: RoomListError) -> Self {
        self.failure = Some(error);
        self
    }

    /// One batch to every live subscriber, applied to the state of later subscribers.
    pub fn push(&self, diffs: Vec<RoomListDiff>) {
        let mut inner = locked(&self.inner);
        for diff in diffs.iter().cloned() {
            inner.rooms = apply_diff(&inner.rooms, diff);
        }
        inner.subscribers.send(&diffs);
    }
}

impl RoomListPort for FakeRoomList {
    fn subscribe(
        &self,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Vec<RoomListDiff>>, RoomListError>> {
        let result = match &self.failure {
            Some(error) => Err(error.clone()),
            None => {
                let mut inner = locked(&self.inner);
                let first = vec![RoomListDiff::Reset(inner.rooms.clone())];
                Ok(inner.subscribers.join(first))
            }
        };
        std::future::ready(result).boxed()
    }
}
