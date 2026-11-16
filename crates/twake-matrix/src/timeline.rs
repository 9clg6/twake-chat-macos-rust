//! `TimelinePort` on matrix-sdk-ui `Timeline` (A031).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use futures::future::BoxFuture;
use futures::stream::{self, BoxStream, StreamExt};
use matrix_sdk::ruma::OwnedRoomId;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::{Client, Room};
use tokio::sync::mpsc;
use twake_app::ports::{TimelineError, TimelinePort};
use twake_domain::RoomId;
use twake_domain::timeline::TimelineDiff;

use crate::sync::Syncer;
use crate::view::View;

/// The conversations of the account, read from sliding sync timelines.
pub struct MatrixTimeline {
    sync: Syncer,
    views: Mutex<HashMap<RoomId, Vec<Weak<View>>>>,
}

impl MatrixTimeline {
    /// An adapter over `client`; the sync starts at the first `open`.
    pub fn new(client: Client) -> Self {
        Self {
            sync: Syncer::new(client),
            views: Mutex::new(HashMap::new()),
        }
    }

    /// The room, once the server has answered at least once.
    async fn room(&self, id: &RoomId) -> Result<Room, TimelineError> {
        self.sync
            .ensure()
            .await
            .map_err(|_| TimelineError::Unavailable)?;
        let id = OwnedRoomId::try_from(id.as_str()).map_err(|_| TimelineError::RoomNotFound)?;
        self.sync
            .client()
            .get_room(&id)
            .ok_or(TimelineError::RoomNotFound)
    }

    fn remember(&self, id: RoomId, view: &Arc<View>) {
        if let Ok(mut views) = self.views.lock() {
            let open = views.entry(id).or_default();
            open.retain(|v| v.strong_count() > 0);
            open.push(Arc::downgrade(view));
        }
    }

    fn open_views(&self, id: &RoomId) -> Vec<Arc<View>> {
        let views = self.views.lock().ok();
        let open = views.as_ref().and_then(|v| v.get(id));
        open.map(|v| v.iter().filter_map(Weak::upgrade).collect())
            .unwrap_or_default()
    }
}

impl TimelinePort for MatrixTimeline {
    fn open(
        &self,
        room: RoomId,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Vec<TimelineDiff>>, TimelineError>> {
        Box::pin(async move {
            let found = self.room(&room).await?;
            let running = self
                .sync
                .ensure()
                .await
                .map_err(|_| TimelineError::Unavailable)?;
            let (tx, mut rx) = mpsc::unbounded_channel();
            let view = View::open(found, running, tx).await?;
            self.remember(room, &view);
            Ok(stream::poll_fn(move |cx| rx.poll_recv(cx)).boxed())
        })
    }

    fn paginate_back(&self, room: RoomId) -> BoxFuture<'_, Result<(), TimelineError>> {
        Box::pin(async move {
            self.room(&room).await?;
            for view in self.open_views(&room) {
                view.page_back().await?;
            }
            Ok(())
        })
    }

    fn send_text(&self, room: RoomId, body: String) -> BoxFuture<'_, Result<(), TimelineError>> {
        Box::pin(async move {
            let found = self.room(&room).await?;
            let content = RoomMessageEventContent::text_plain(body);
            found
                .send_queue()
                .send(content.into())
                .await
                .map_err(|_| TimelineError::Unavailable)?;
            Ok(())
        })
    }
}
