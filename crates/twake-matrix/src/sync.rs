//! The sliding sync (MSC4186) an adapter starts for itself, at its first use (A031).
//!
//! ponytail: a bare `RoomListService::sync` per adapter, not the `SyncService` of ARCHITECTURE
//! section 7 (it also runs the encryption sync, a second sliding sync connection the mock
//! homeserver of the specs does not script). The app wires one shared `SyncService` later.

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use matrix_sdk::Client;
use matrix_sdk_ui::RoomListService;
use matrix_sdk_ui::room_list_service::Error as ServiceError;
use tokio::sync::{Mutex, oneshot};
use tokio::task::JoinHandle;
use twake_app::ports::RoomListError;

use crate::map;

/// The wait before each restart of the sync: `initial`, doubled at each attempt, up to `max`.
/// No randomness: one device, one server connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
    initial: Duration,
    max: Duration,
}

impl Backoff {
    /// A back-off starting at `initial` and never above `max`.
    pub fn new(initial: Duration, max: Duration) -> Self {
        Self { initial, max }
    }

    /// The wait before restart number `attempt`, the first being 0.
    pub fn delay(&self, attempt: u32) -> Duration {
        let factor = 1u32 << attempt.min(31);
        self.initial.saturating_mul(factor).min(self.max)
    }
}

impl Default for Backoff {
    /// 1 s, doubling up to one minute.
    fn default() -> Self {
        Self::new(Duration::from_secs(1), Duration::from_secs(60))
    }
}

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
    backoff: Backoff,
    running: Mutex<Option<Arc<Running>>>,
}

impl Syncer {
    pub(crate) fn new(client: Client) -> Self {
        Self::with_backoff(client, Backoff::default())
    }

    pub(crate) fn with_backoff(client: Client, backoff: Backoff) -> Self {
        Self {
            client,
            backoff,
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
        let running = Arc::new(start(self.client.clone(), self.backoff).await?);
        *slot = Some(Arc::clone(&running));
        Ok(running)
    }
}

async fn start(client: Client, backoff: Backoff) -> Result<Running, RoomListError> {
    let service = Arc::new(RoomListService::new(client).await.map_err(service_error)?);
    let (first_tx, first_rx) = oneshot::channel();
    let task = spawn_poll(Arc::clone(&service), first_tx, backoff);
    let running = Running { service, task };
    match first_rx.await {
        Ok(Ok(())) => Ok(running),
        Ok(Err(error)) => Err(error),
        Err(_) => Err(RoomListError::Unavailable),
    }
}

/// Polls the sync; after an error it waits `backoff` and starts it again, until the session
/// is gone. The first error also goes to `first`, so `start` can fail.
fn spawn_poll(
    service: Arc<RoomListService>,
    first: oneshot::Sender<Result<(), RoomListError>>,
    backoff: Backoff,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut first = Some(first);
        let mut attempt = 0;
        loop {
            let sync = service.sync();
            futures::pin_mut!(sync);
            while let Some(step) = sync.next().await {
                let step = step.map_err(service_error);
                if step.is_ok() {
                    attempt = 0;
                }
                let gone = step == Err(RoomListError::NotSignedIn);
                if let Some(tx) = first.take() {
                    let _ = tx.send(step.clone());
                }
                if gone {
                    return;
                }
                if step.is_err() {
                    break;
                }
            }
            tokio::time::sleep(backoff.delay(attempt)).await;
            attempt = attempt.saturating_add(1);
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
