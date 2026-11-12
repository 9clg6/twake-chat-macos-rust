//! Contract suites: the behaviour every implementation of a port must have, run on the
//! fake in `twake-testing` and, later, on the real adapter (ARCHITECTURE.md section 4).
//!
//! Each suite takes a factory that builds the port **seeded** with some content, and answers
//! `Ok(())` or the first broken rule as text (no panic in library code: the caller of a
//! suite does `.expect(..)` in its test). A suite asserts what the screens rely on and
//! nothing about how the port is made. A bug found because fake and adapter disagree adds a
//! case here, and the fake follows the adapter.
//!
//! Specified by `docs/dev/app-core-api-sketch.md` (section 4).

use std::fmt::Debug;
use std::sync::Arc;

use futures::StreamExt;
use futures::executor::block_on;
use futures::stream::BoxStream;
use twake_app::ports::{RoomListPort, TIMELINE_PAGE_SIZE, TimelineError, TimelinePort};
use twake_domain::room_list::{RoomKind, RoomListDiff, RoomSummary};
use twake_domain::timeline::{Message, SendState, TimelineDiff};
use twake_domain::{EventId, RoomId, Timestamp, UserId};

pub use crate::draft_store_contract::draft_store;
pub use crate::session_store_contract::session_store;

/// The outcome of a suite: `Ok(())`, or the first rule that does not hold.
pub type Verdict = Result<(), String>;

pub(crate) fn same<T: PartialEq + Debug>(rule: &str, got: T, want: T) -> Verdict {
    if got == want {
        Ok(())
    } else {
        Err(format!("{rule}: got {got:?}, wanted {want:?}"))
    }
}

pub(crate) fn holds(rule: &str, condition: bool) -> Verdict {
    if condition {
        Ok(())
    } else {
        Err(format!("{rule}: does not hold"))
    }
}

fn first_batch<T>(batches: &mut BoxStream<'static, Vec<T>>) -> Result<Vec<T>, String> {
    block_on(batches.next()).ok_or_else(|| "the stream ended before a batch".to_string())
}

fn room_id(n: usize) -> RoomId {
    // Constant, well-formed ids: a failure here is a bug of the suite, shown by the verdict.
    RoomId::parse(&format!("!contract{n}:example.org")).unwrap_or_else(|_| unreachable!())
}

// ------------------------------------------------------------ room list

fn seed_room(n: usize) -> RoomSummary {
    RoomSummary::new(room_id(n), format!("Contract room {n}"), RoomKind::Group)
        .with_last_activity(Timestamp::from_millis(1_000 + n as u64 * 7))
        .with_last_message_preview(format!("preview {n}"))
}

fn by_id(mut rooms: Vec<RoomSummary>) -> Vec<RoomSummary> {
    rooms.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
    rooms
}

fn reset_rooms(batch: Vec<RoomListDiff>) -> Result<Vec<RoomSummary>, String> {
    match batch.as_slice() {
        [RoomListDiff::Reset(rooms)] => Ok(by_id(rooms.clone())),
        other => Err(format!("the first batch must be one Reset, got {other:?}")),
    }
}

type MakeRoomList<'a> = &'a dyn Fn(Vec<RoomSummary>) -> Arc<dyn RoomListPort>;

/// What every `RoomListPort` must do. `make` builds a port whose account holds `rooms`.
pub fn room_list(make: impl Fn(Vec<RoomSummary>) -> Arc<dyn RoomListPort>) -> Verdict {
    first_batch_is_the_whole_list(&make)?;
    an_account_without_room_starts_with_an_empty_reset(&make)?;
    every_subscription_starts_with_its_own_reset(&make)
}

fn subscribe(
    port: &Arc<dyn RoomListPort>,
) -> Result<BoxStream<'static, Vec<RoomListDiff>>, String> {
    block_on(port.subscribe()).map_err(|e| format!("subscribe failed: {e:?}"))
}

fn first_batch_is_the_whole_list(make: MakeRoomList<'_>) -> Verdict {
    let seed = vec![seed_room(1), seed_room(2), seed_room(3)];
    let mut batches = subscribe(&make(seed.clone()))?;
    let rooms = reset_rooms(first_batch(&mut batches)?)?;
    same("the first Reset holds the seeded rooms", rooms, by_id(seed))
}

fn an_account_without_room_starts_with_an_empty_reset(make: MakeRoomList<'_>) -> Verdict {
    let mut batches = subscribe(&make(vec![]))?;
    let rooms = reset_rooms(first_batch(&mut batches)?)?;
    same("an empty account starts with an empty Reset", rooms, vec![])
}

fn every_subscription_starts_with_its_own_reset(make: MakeRoomList<'_>) -> Verdict {
    let seed = vec![seed_room(4), seed_room(5)];
    let port = make(seed.clone());
    let mut one = subscribe(&port)?;
    let mut two = subscribe(&port)?;
    let rooms_one = reset_rooms(first_batch(&mut one)?)?;
    let rooms_two = reset_rooms(first_batch(&mut two)?)?;
    same(
        "subscription one starts with a Reset",
        rooms_one,
        by_id(seed.clone()),
    )?;
    same(
        "subscription two starts with a Reset",
        rooms_two,
        by_id(seed),
    )
}

// ------------------------------------------------------------ timeline

fn history(count: usize) -> Vec<Message> {
    let sender = UserId::parse("@contract-bob:example.org").unwrap_or_else(|_| unreachable!());
    (0..count)
        .map(|i| {
            let event = EventId::parse(&format!("$contract{i}")).unwrap_or_else(|_| unreachable!());
            Message::remote(
                event,
                sender.clone(),
                Timestamp::from_millis(1_700_000_000_000 + i as u64 * 1_000),
                format!("message {i}"),
            )
        })
        .collect()
}

fn me() -> UserId {
    UserId::parse("@contract-me:example.org").unwrap_or_else(|_| unreachable!())
}

type MakeTimeline<'a> = &'a dyn Fn(UserId, RoomId, Vec<Message>) -> Arc<dyn TimelinePort>;
type Opened = (Arc<dyn TimelinePort>, BoxStream<'static, Vec<TimelineDiff>>);

fn open(
    port: &Arc<dyn TimelinePort>,
    room: usize,
) -> Result<BoxStream<'static, Vec<TimelineDiff>>, String> {
    block_on(port.open(room_id(room))).map_err(|e| format!("open failed: {e:?}"))
}

fn opened(make: MakeTimeline<'_>, count: usize) -> Result<Opened, String> {
    let port = make(me(), room_id(1), history(count));
    let mut batches = open(&port, 1)?;
    first_batch(&mut batches)?;
    Ok((port, batches))
}

fn paginate(port: &Arc<dyn TimelinePort>) -> Verdict {
    block_on(port.paginate_back(room_id(1))).map_err(|e| format!("paginate failed: {e:?}"))
}

fn send(port: &Arc<dyn TimelinePort>, text: &str) -> Verdict {
    block_on(port.send_text(room_id(1), text.to_string()))
        .map_err(|e| format!("send failed: {e:?}"))
}

/// What every `TimelinePort` must do. `make` builds a port for the user `me`, who is in the
/// room `room` whose whole history is `messages` (oldest first, all from other people).
pub fn timeline(make: impl Fn(UserId, RoomId, Vec<Message>) -> Arc<dyn TimelinePort>) -> Verdict {
    opening_gives_the_last_page_in_one_reset(&make)?;
    opening_a_short_history_gives_all_of_it(&make)?;
    opening_an_unknown_room_is_refused(&make)?;
    pagination_goes_back_page_by_page_to_the_start(&make)?;
    pagination_of_exactly_one_page_ends_with_an_empty_page(&make)?;
    pagination_of_an_unknown_room_is_refused(&make)?;
    a_sent_text_comes_back_as_a_local_echo(&make)?;
    two_sends_have_two_transaction_ids(&make)?;
    sending_to_an_unknown_room_is_refused(&make)
}

fn opening_gives_the_last_page_in_one_reset(make: MakeTimeline<'_>) -> Verdict {
    let all = history(TIMELINE_PAGE_SIZE + 45);
    let mut batches = open(&make(me(), room_id(1), all.clone()), 1)?;
    let want = vec![TimelineDiff::Reset(all[45..].to_vec())];
    same(
        "opening shows the last page",
        first_batch(&mut batches)?,
        want,
    )
}

fn opening_a_short_history_gives_all_of_it(make: MakeTimeline<'_>) -> Verdict {
    let all = history(4);
    let mut batches = open(&make(me(), room_id(1), all.clone()), 1)?;
    same(
        "a short history shows whole",
        first_batch(&mut batches)?,
        vec![TimelineDiff::Reset(all)],
    )
}

fn opening_an_unknown_room_is_refused(make: MakeTimeline<'_>) -> Verdict {
    let port = make(me(), room_id(1), history(3));
    let refused = block_on(port.open(room_id(2))).err();
    same(
        "an unknown room is refused",
        refused,
        Some(TimelineError::RoomNotFound),
    )
}

fn pagination_goes_back_page_by_page_to_the_start(make: MakeTimeline<'_>) -> Verdict {
    let all = history(TIMELINE_PAGE_SIZE * 2 + 15);
    let (port, mut batches) = opened(make, all.len())?;
    paginate(&port)?;
    let middle = TimelineDiff::Prepend {
        messages: all[15..45].to_vec(),
        reached_start: false,
    };
    same("the second page", first_batch(&mut batches)?, vec![middle])?;
    paginate(&port)?;
    let oldest = TimelineDiff::Prepend {
        messages: all[..15].to_vec(),
        reached_start: true,
    };
    same(
        "the last page reaches the start",
        first_batch(&mut batches)?,
        vec![oldest],
    )
}

fn pagination_of_exactly_one_page_ends_with_an_empty_page(make: MakeTimeline<'_>) -> Verdict {
    let (port, mut batches) = opened(make, TIMELINE_PAGE_SIZE)?;
    paginate(&port)?;
    let end = TimelineDiff::Prepend {
        messages: vec![],
        reached_start: true,
    };
    same(
        "an empty page reaches the start",
        first_batch(&mut batches)?,
        vec![end],
    )
}

fn pagination_of_an_unknown_room_is_refused(make: MakeTimeline<'_>) -> Verdict {
    let port = make(me(), room_id(1), history(3));
    let refused = block_on(port.paginate_back(room_id(2)));
    same(
        "pagination of an unknown room",
        refused,
        Err(TimelineError::RoomNotFound),
    )
}

fn echo_of(batch: Vec<TimelineDiff>) -> Result<Message, String> {
    match batch.as_slice() {
        [TimelineDiff::Append(list)] if list.len() == 1 => Ok(list[0].clone()),
        other => Err(format!("a send must append one local echo, got {other:?}")),
    }
}

fn a_sent_text_comes_back_as_a_local_echo(make: MakeTimeline<'_>) -> Verdict {
    let (port, mut batches) = opened(make, 3)?;
    send(&port, "on its way")?;
    let echo = echo_of(first_batch(&mut batches)?)?;
    same("echo body", echo.body.as_str(), "on its way")?;
    same("echo sender", &echo.sender, &me())?;
    same("echo state", echo.send_state, SendState::Sending)?;
    holds("echo has no event id yet", echo.event_id().is_none())?;
    holds("echo has a transaction id", echo.txn_id().is_some())
}

fn two_sends_have_two_transaction_ids(make: MakeTimeline<'_>) -> Verdict {
    let (port, mut batches) = opened(make, 3)?;
    send(&port, "one")?;
    send(&port, "two")?;
    let one = echo_of(first_batch(&mut batches)?)?;
    let two = echo_of(first_batch(&mut batches)?)?;
    holds(
        "two sends, two transaction ids",
        one.txn_id() != two.txn_id(),
    )
}

fn sending_to_an_unknown_room_is_refused(make: MakeTimeline<'_>) -> Verdict {
    let port = make(me(), room_id(1), history(3));
    let refused = block_on(port.send_text(room_id(2), "lost".to_string()));
    same(
        "sending to an unknown room",
        refused,
        Err(TimelineError::RoomNotFound),
    )
}
