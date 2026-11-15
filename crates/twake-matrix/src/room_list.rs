//! `RoomListPort` on matrix-sdk-ui `RoomListService` (sliding sync, MSC4186) (A031).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use futures::stream::{self, BoxStream, Stream, StreamExt};
use matrix_sdk::event_cache::RoomEventCacheGenericUpdate;
use matrix_sdk::{Client, Room};
use matrix_sdk_ui::eyeball_im::{Vector, VectorDiff};
use matrix_sdk_ui::room_list_service::filters::new_filter_non_left;
use matrix_sdk_ui::room_list_service::{RoomList, RoomListItem};
use tokio::sync::{broadcast, mpsc};
use twake_app::ports::{RoomListError, RoomListPort};
use twake_domain::RoomId;
use twake_domain::room_list::{RoomListDiff, RoomSummary};

use crate::summary::{Ctx, has_message, summarise};
use crate::sync::{Running, Syncer, service_error};

/// The rooms of the account, read from the sliding sync room list.
pub struct MatrixRoomList {
    sync: Syncer,
}

impl MatrixRoomList {
    /// An adapter over `client`; the sync starts at the first `subscribe`.
    pub fn new(client: Client) -> Self {
        Self {
            sync: Syncer::new(client),
        }
    }
}

impl RoomListPort for MatrixRoomList {
    fn subscribe(
        &self,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Vec<RoomListDiff>>, RoomListError>> {
        Box::pin(async move {
            let running = self.sync.ensure().await?;
            let list = running.service.all_rooms().await.map_err(service_error)?;
            let (tx, mut rx) = mpsc::unbounded_channel();
            tokio::spawn(feed(running, list, self.sync.client().clone(), tx));
            let batches = stream::poll_fn(move |cx| rx.poll_recv(cx));
            Ok(batches.boxed())
        })
    }
}

/// What wakes the feed.
enum Event {
    Diffs(Vec<VectorDiff<RoomListItem>>),
    Touched(String),
    Everything,
}

async fn feed(
    _running: Arc<Running>,
    list: RoomList,
    client: Client,
    tx: mpsc::UnboundedSender<Vec<RoomListDiff>>,
) {
    // ponytail: no cap on the rooms shown, the sort and the paging belong to the domain.
    let (diffs, controller) = list.entries_with_dynamic_adapters(usize::MAX);
    controller.set_filter(Box::new(new_filter_non_left()));
    let events = stream::select(diffs.map(Event::Diffs), cache_updates(&client));
    futures::pin_mut!(events);
    let mut mirror = Mirror::default();
    let Some(()) = settle(&mut events, &mut mirror).await else {
        return;
    };
    let ctx = Ctx::new(&client).await;
    let all = summaries(&ctx, mirror.rooms.iter().map(|r| &**r)).await;
    mirror.sent = all.iter().map(|s| (s.id.clone(), s.clone())).collect();
    if tx.send(vec![RoomListDiff::Reset(all)]).is_err() {
        return;
    }
    while let Some(event) = events.next().await {
        let batch = mirror.apply(&client, event).await;
        if !batch.is_empty() && tx.send(batch).is_err() {
            break;
        }
    }
}

/// Waits for the first list of rooms, then until every room shows a last message in its event
/// cache, or the caches went quiet for `QUIET` after some news (rooms with no message never
/// show one), or `SETTLE_MAX` went by: the SDK fills the caches in the background after a
/// response, and a first `Reset` without the last messages would show empty rows. None if
/// the streams end.
/// ponytail: polls every `TICK`, the SDK gives no signal; a slow machine shows the last
/// messages a moment later, as `Update`s.
async fn settle(
    events: &mut (impl Stream<Item = Event> + Unpin),
    mirror: &mut Mirror,
) -> Option<()> {
    loop {
        if let Event::Diffs(diffs) = events.next().await? {
            mirror.absorb(diffs);
            break;
        }
    }
    let start = tokio::time::Instant::now();
    let mut last_news: Option<tokio::time::Instant> = None;
    while start.elapsed() < SETTLE_MAX && !quiet(last_news) && !mirror.all_have_a_message().await {
        if listen(events, mirror).await? {
            last_news = Some(tokio::time::Instant::now());
        }
    }
    Some(())
}

fn quiet(last_news: Option<tokio::time::Instant>) -> bool {
    last_news.is_some_and(|at| at.elapsed() >= QUIET)
}

/// Waits `TICK` for an event: Some(true) if one came, Some(false) if none, None if the streams ended.
async fn listen(
    events: &mut (impl Stream<Item = Event> + Unpin),
    mirror: &mut Mirror,
) -> Option<bool> {
    match tokio::time::timeout(TICK, events.next()).await {
        Ok(Some(Event::Diffs(diffs))) => {
            mirror.absorb(diffs);
            Some(true)
        }
        Ok(Some(_)) => Some(true),
        Ok(None) => None,
        Err(_) => Some(false),
    }
}

const SETTLE_MAX: Duration = Duration::from_secs(2);
const TICK: Duration = Duration::from_millis(25);
const QUIET: Duration = Duration::from_millis(400);
const NEW_ROOM_PAUSE: Duration = Duration::from_millis(150);

/// A room whose event cache changed: its last message may have.
fn cache_updates(client: &Client) -> impl Stream<Item = Event> + use<> {
    let receiver = client.event_cache().subscribe_to_room_generic_updates();
    stream::unfold(receiver, |mut receiver| async move {
        match receiver.recv().await {
            Ok(RoomEventCacheGenericUpdate { room_id }) => {
                Some((Event::Touched(room_id.to_string()), receiver))
            }
            Err(broadcast::error::RecvError::Lagged(_)) => Some((Event::Everything, receiver)),
            Err(broadcast::error::RecvError::Closed) => None,
        }
    })
}

/// The rooms of the SDK and the summaries already sent: what turns a change into diffs.
#[derive(Default)]
struct Mirror {
    rooms: Vector<RoomListItem>,
    sent: HashMap<RoomId, RoomSummary>,
}

impl Mirror {
    /// Applies diffs of the SDK; the ids they touch, and whether the whole list changed.
    fn absorb(&mut self, diffs: Vec<VectorDiff<RoomListItem>>) -> (HashSet<String>, bool) {
        let mut touched = HashSet::new();
        let mut all = false;
        for diff in diffs {
            all |= matches!(diff, VectorDiff::Reset { .. } | VectorDiff::Clear);
            touched.extend(rooms_of(&diff));
            diff.apply(&mut self.rooms);
        }
        (touched, all)
    }

    /// The SDK announces a room before it has applied its state: a pause lets the name come.
    /// ponytail: a pause, not a signal; a late name arrives as an `Update`.
    async fn let_new_rooms_fill(&self, touched: &HashSet<String>) {
        let is_new = |id: &String| RoomId::parse(id).is_ok_and(|id| !self.sent.contains_key(&id));
        if touched.iter().any(is_new) {
            tokio::time::sleep(NEW_ROOM_PAUSE).await;
        }
    }

    async fn all_have_a_message(&self) -> bool {
        for room in &self.rooms {
            if !has_message(room).await {
                return false;
            }
        }
        true
    }

    async fn apply(&mut self, client: &Client, event: Event) -> Vec<RoomListDiff> {
        let ctx = Ctx::new(client).await;
        match event {
            Event::Diffs(diffs) => {
                let (touched, all) = self.absorb(diffs);
                self.let_new_rooms_fill(&touched).await;
                self.refresh(&ctx, &touched, all).await
            }
            Event::Touched(id) => self.refresh(&ctx, &HashSet::from([id]), false).await,
            Event::Everything => self.refresh(&ctx, &HashSet::new(), true).await,
        }
    }

    /// Diffs from what was sent to what the rooms say now: `all` rooms, else the `touched` and
    /// the new ones; rooms gone from the list are removed.
    async fn refresh(
        &mut self,
        ctx: &Ctx,
        touched: &HashSet<String>,
        all: bool,
    ) -> Vec<RoomListDiff> {
        let sent = &mut self.sent;
        let ids: HashSet<&str> = self.rooms.iter().map(|r| r.room_id().as_str()).collect();
        let mut diffs: Vec<RoomListDiff> = sent
            .keys()
            .filter(|id| !ids.contains(id.as_str()))
            .cloned()
            .map(RoomListDiff::Remove)
            .collect();
        sent.retain(|id, _| ids.contains(id.as_str()));
        let wanted = self.rooms.iter().filter(|r| {
            all || touched.contains(r.room_id().as_str())
                || RoomId::parse(r.room_id().as_str()).is_ok_and(|id| !sent.contains_key(&id))
        });
        for summary in summaries(ctx, wanted.map(|r| &**r)).await {
            match sent.insert(summary.id.clone(), summary.clone()) {
                None => diffs.push(RoomListDiff::Insert(summary)),
                Some(old) if old != summary => diffs.push(RoomListDiff::Update(summary)),
                Some(_) => {}
            }
        }
        diffs
    }
}

async fn summaries<'a>(ctx: &Ctx, rooms: impl Iterator<Item = &'a Room>) -> Vec<RoomSummary> {
    let mut out = Vec::new();
    for room in rooms {
        out.extend(summarise(ctx, room).await);
    }
    out
}

/// The ids of the rooms a diff brings.
fn rooms_of(diff: &VectorDiff<RoomListItem>) -> Vec<String> {
    let id = |r: &RoomListItem| r.room_id().to_string();
    match diff {
        VectorDiff::Append { values } | VectorDiff::Reset { values } => {
            values.iter().map(id).collect()
        }
        VectorDiff::PushFront { value }
        | VectorDiff::PushBack { value }
        | VectorDiff::Insert { value, .. }
        | VectorDiff::Set { value, .. } => vec![id(value)],
        _ => vec![],
    }
}
