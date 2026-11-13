//! One open conversation: the SDK timeline, the window of it the screen shows, and the diffs
//! that tell the screen what changed (A031).
//!
//! The SDK loads history in its own steps and counts every event; the port counts messages, a
//! page of `TIMELINE_PAGE_SIZE`. So the view keeps every message the SDK holds (`loaded`) and
//! shows a window at the end of it (from `first`): a page back first shows what is already
//! loaded, and asks the server only when that is less than a page.

use std::sync::Arc;

use futures::StreamExt;
use matrix_sdk::Room;
use matrix_sdk_ui::Timeline;
use matrix_sdk_ui::timeline::RoomExt;
use tokio::sync::{Mutex, mpsc};
use twake_app::ports::{TIMELINE_PAGE_SIZE, TimelineError};
use twake_domain::timeline::{Message, TimelineDiff};

use crate::message::{key_of, to_message};
use crate::sync::Running;

/// How many times a page may ask the server before it gives up (a server that answers with
/// nothing and no end would otherwise loop).
const MAX_ROUNDS: usize = 20;

pub(crate) struct View {
    timeline: Timeline,
    state: Mutex<State>,
    tx: mpsc::UnboundedSender<Vec<TimelineDiff>>,
    _sync: Arc<Running>,
}

#[derive(Default)]
struct State {
    loaded: Vec<Message>,
    first: Option<Message>,
    reached_start: bool,
}

impl State {
    /// Where the window starts in `loaded`.
    fn start(&self) -> usize {
        self.first
            .as_ref()
            .and_then(|f| self.loaded.iter().position(|m| m.is_same_as(f)))
            .unwrap_or(0)
    }

    fn shown(&self) -> &[Message] {
        &self.loaded[self.start()..]
    }
}

fn unavailable<E>(_: E) -> TimelineError {
    TimelineError::Unavailable
}

impl View {
    /// Opens `room`: the first batch (one `Reset` of the last page) is already queued on `tx`.
    pub(crate) async fn open(
        room: Room,
        sync: Arc<Running>,
        tx: mpsc::UnboundedSender<Vec<TimelineDiff>>,
    ) -> Result<Arc<Self>, TimelineError> {
        let timeline = room.timeline().await.map_err(unavailable)?;
        let view = Arc::new(Self {
            timeline,
            state: Mutex::new(State::default()),
            tx,
            _sync: sync,
        });
        view.first_page().await?;
        tokio::spawn(watch(Arc::clone(&view)));
        Ok(view)
    }

    async fn snapshot(&self) -> Vec<Message> {
        let items = self.timeline.items().await;
        let events = items.iter().filter_map(|i| i.as_event());
        events.filter_map(to_message).collect()
    }

    /// Reads the SDK, and asks for older events until `enough` holds or the start is reached.
    async fn load_until(
        &self,
        state: &mut State,
        enough: impl Fn(&State) -> bool,
    ) -> Result<(), TimelineError> {
        state.loaded = self.snapshot().await;
        for _ in 0..MAX_ROUNDS {
            if state.reached_start || enough(state) {
                break;
            }
            let page = u16::try_from(TIMELINE_PAGE_SIZE).unwrap_or(u16::MAX);
            state.reached_start = self
                .timeline
                .paginate_backwards(page)
                .await
                .map_err(unavailable)?;
            let mut grown = self.snapshot().await;
            carry_txns(&mut grown, &state.loaded);
            state.loaded = grown;
        }
        Ok(())
    }

    async fn first_page(&self) -> Result<(), TimelineError> {
        let mut state = self.state.lock().await;
        self.load_until(&mut state, |s| s.loaded.len() >= TIMELINE_PAGE_SIZE)
            .await?;
        let start = state.loaded.len().saturating_sub(TIMELINE_PAGE_SIZE);
        state.first = state.loaded.get(start).cloned();
        let shown = state.loaded[start..].to_vec();
        self.send(vec![TimelineDiff::Reset(shown)]);
        Ok(())
    }

    /// One page older, as a `Prepend`.
    pub(crate) async fn page_back(&self) -> Result<(), TimelineError> {
        let mut state = self.state.lock().await;
        self.load_until(&mut state, |s| s.start() >= TIMELINE_PAGE_SIZE)
            .await?;
        let end = state.start();
        let start = end.saturating_sub(TIMELINE_PAGE_SIZE);
        let page = state.loaded[start..end].to_vec();
        state.first = state.loaded.get(start).cloned();
        let reached_start = start == 0 && state.reached_start;
        self.send(vec![TimelineDiff::Prepend {
            messages: page,
            reached_start,
        }]);
        Ok(())
    }

    /// What changed in the window since the last time; false once nobody listens.
    async fn refresh(&self) -> bool {
        let mut state = self.state.lock().await;
        let mut new = self.snapshot().await;
        carry_txns(&mut new, &state.loaded);
        let from = window_start(&state, &new);
        let diffs = diff(state.shown(), &new[from..]);
        state.first = new.get(from).cloned().or_else(|| state.first.clone());
        state.loaded = new;
        diffs.is_empty() || self.send(diffs)
    }

    fn send(&self, diffs: Vec<TimelineDiff>) -> bool {
        self.tx.send(diffs).is_ok()
    }
}

/// Wakes at every change of the SDK timeline.
async fn watch(view: Arc<View>) {
    let (_items, changes) = view.timeline.subscribe().await;
    futures::pin_mut!(changes);
    // Whatever came between the first page and the subscription.
    if !view.refresh().await {
        return;
    }
    while changes.next().await.is_some() {
        if !view.refresh().await {
            return;
        }
    }
}

/// A message that was a local echo keeps its transaction id once the server's event replaces it.
fn carry_txns(new: &mut [Message], old: &[Message]) {
    for message in new.iter_mut().filter(|m| m.txn_id().is_none()) {
        let txn = old
            .iter()
            .find(|o| o.is_same_as(message))
            .and_then(|o| o.txn_id().cloned());
        if let Some(txn) = txn {
            *message = message.clone().with_txn_id(txn);
        }
    }
}

/// Where the window starts in `new`: at the first message it showed that is still there, else
/// at the first message the screen has never seen.
fn window_start(state: &State, new: &[Message]) -> usize {
    let still_there = |m: &Message| new.iter().position(|n| n.is_same_as(m));
    state
        .shown()
        .iter()
        .find_map(still_there)
        .or_else(|| {
            new.iter()
                .position(|n| !state.loaded.iter().any(|o| o.is_same_as(n)))
        })
        .unwrap_or(new.len())
}

/// Removals, then updates, then what is new at the end.
fn diff(old: &[Message], new: &[Message]) -> Vec<TimelineDiff> {
    let mut out: Vec<TimelineDiff> = old
        .iter()
        .filter(|o| !new.iter().any(|n| n.is_same_as(o)))
        .filter_map(key_of)
        .map(TimelineDiff::Remove)
        .collect();
    let mut added = Vec::new();
    for message in new {
        match old.iter().find(|o| o.is_same_as(message)) {
            Some(known) if known != message => out.push(TimelineDiff::Update(message.clone())),
            Some(_) => {}
            None => added.push(message.clone()),
        }
    }
    if !added.is_empty() {
        out.push(TimelineDiff::Append(added));
    }
    out
}
