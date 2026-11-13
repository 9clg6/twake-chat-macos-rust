//! The sliding sync (MSC4186) an adapter starts for itself, at its first use (A031).
//!
//! ponytail: a bare `RoomListService::sync` per adapter, not the `SyncService` of ARCHITECTURE
//! section 7 (it also runs the encryption sync, a second sliding sync connection the mock
//! homeserver of the specs does not script). The app wires one shared `SyncService` later.

use std::sync::Arc;

use futures::StreamExt;
use matrix_sdk::Client;
use matrix_sdk_ui::RoomListService;
use matrix_sdk_ui::room_list_service::Error as ServiceError;
use tokio::sync::{Mutex, oneshot};
use tokio::task::JoinHandle;
use twake_app::ports::RoomListError;

use crate::map;

/// A running sync: the service, and the task that polls it. Dropping it stops the sync.
pub(crate) struct Running {
    pub(crate) service: Arc<RoomListService>,
    task: JoinHandle<()>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// The sync of one client, started once and again if it stopped on an error.
pub(crate) struct Syncer {
    client: Client,
    running: Mutex<Option<Arc<Running>>>,
}

impl Syncer {
    pub(crate) fn new(client: Client) -> Self {
        Self {
            client,
            running: Mutex::new(None),
        }
    }

    pub(crate) fn client(&self) -> &Client {
        &self.client
    }

    /// The running sync, started if need be; returns once the server has answered once.
    pub(crate) async fn ensure(&self) -> Result<Arc<Running>, RoomListError> {
        if self.client.session_meta().is_none() {
            return Err(RoomListError::NotSignedIn);
        }
        let mut slot = self.running.lock().await;
        if let Some(running) = slot.as_ref().filter(|r| !r.task.is_finished()) {
            return Ok(Arc::clone(running));
        }
        let running = Arc::new(start(self.client.clone()).await?);
        *slot = Some(Arc::clone(&running));
        Ok(running)
    }
}

async fn start(client: Client) -> Result<Running, RoomListError> {
    let service = Arc::new(RoomListService::new(client).await.map_err(service_error)?);
    let (first_tx, first_rx) = oneshot::channel();
    let task = spawn_poll(Arc::clone(&service), first_tx);
    let running = Running { service, task };
    match first_rx.await {
        Ok(Ok(())) => Ok(running),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(RoomListError::Unavailable),
    }
}

fn spawn_poll(
    service: Arc<RoomListService>,
    first: oneshot::Sender<Result<(), RoomListError>>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let sync = service.sync();
        futures::pin_mut!(sync);
        let mut first = Some(first);
        while let Some(step) = sync.next().await {
            let step = step.map_err(service_error);
            let failed = step.is_err();
            if let Some(tx) = first.take() {
                let _ = tx.send(step);
            }
            if failed {
                break;
            }
        }
    })
}

/// A failure of the sync, by kind (never by message: it may hold the token).
pub(crate) fn service_error(error: ServiceError) -> RoomListError {
    match error {
        ServiceError::SlidingSync(e) => map::room_list_error(&e),
        _ => RoomListError::Unavailable,
    }
}
