//! A draft store with no disk: drafts in memory, one per room.

use std::collections::HashMap;
use std::sync::Mutex;

use futures::FutureExt;
use futures::future::BoxFuture;
use twake_app::composer::Draft;
use twake_app::ports::{DraftStoreError, DraftStorePort};
use twake_domain::RoomId;

use crate::subscribers::locked;

/// Holds the drafts; `unavailable` makes every call fail.
#[derive(Default)]
pub struct FakeDraftStore {
    drafts: Mutex<HashMap<RoomId, Draft>>,
    unreachable: bool,
}

impl FakeDraftStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Every call returns `Unavailable`.
    pub fn unavailable(mut self) -> Self {
        self.unreachable = true;
        self
    }

    fn run<T: Send + 'static>(
        &self,
        action: impl FnOnce(&mut HashMap<RoomId, Draft>) -> T,
    ) -> BoxFuture<'_, Result<T, DraftStoreError>> {
        let result = if self.unreachable {
            Err(DraftStoreError::Unavailable)
        } else {
            Ok(action(&mut locked(&self.drafts)))
        };
        std::future::ready(result).boxed()
    }
}

impl DraftStorePort for FakeDraftStore {
    fn save(&self, room: RoomId, draft: Draft) -> BoxFuture<'_, Result<(), DraftStoreError>> {
        self.run(|drafts| {
            if draft.text.is_empty() {
                drafts.remove(&room);
            } else {
                drafts.insert(room, draft);
            }
        })
    }

    fn load(&self, room: RoomId) -> BoxFuture<'_, Result<Option<Draft>, DraftStoreError>> {
        self.run(|drafts| drafts.get(&room).cloned())
    }

    fn clear(&self, room: RoomId) -> BoxFuture<'_, Result<(), DraftStoreError>> {
        self.run(|drafts| {
            drafts.remove(&room);
        })
    }
}
