//! `FakeRoomList` and `FakeTimeline`: deterministic stand-ins of the ports, with a way for a
//! test to inject what the server would send.
//!
//! Need: ARCHITECTURE.md section 4 ("every port has a fake in `twake-testing` and a contract
//! suite run on both the fake and the real adapter") and section 6 (`--preview` runs the UI
//! on fakes). The fakes are specified here and by `twake_testing::contract`; the real
//! adapter will run the same contract.
//!
//! Public items used are listed in `docs/dev/app-core-api-sketch.md`.

#![cfg(test)]

use std::sync::Arc;

use futures::executor::block_on;
use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use twake_app::ports::{
    RoomListError, RoomListPort, TIMELINE_PAGE_SIZE, TimelineError, TimelinePort,
};
use twake_domain::room_list::{RoomKind, RoomListDiff, RoomSummary};
use twake_domain::timeline::{Message, SendState, TimelineDiff};
use twake_domain::{EventId, RoomId, Timestamp, UserId};
use twake_testing::contract;
use twake_testing::{FakeRoomList, FakeTimeline};

// ------------------------------------------------------------ helpers

const NOW: u64 = 1_800_000_000_000;

fn room_id(n: usize) -> RoomId {
    RoomId::parse(&format!("!fake{n}:example.org")).expect("valid room id")
}

fn room(n: usize, activity: u64) -> RoomSummary {
    RoomSummary::new(room_id(n), format!("Fake {n}"), RoomKind::Group)
        .with_last_activity(Timestamp::from_millis(activity))
}

fn me() -> UserId {
    UserId::parse("@me:example.org").expect("valid user id")
}

fn bob() -> UserId {
    UserId::parse("@bob:example.org").expect("valid user id")
}

fn history(count: usize) -> Vec<Message> {
    (0..count)
        .map(|i| {
            let event = EventId::parse(&format!("$old{i}")).expect("valid event id");
            Message::remote(
                event,
                bob(),
                Timestamp::from_millis(1_000 + i as u64),
                format!("old {i}"),
            )
        })
        .collect()
}

/// The next batch if one is already waiting, never blocking.
fn pending<T>(batches: &mut BoxStream<'static, Vec<T>>) -> Option<Vec<T>> {
    batches.next().now_or_never().flatten()
}

fn ids(rooms: &[RoomSummary]) -> Vec<RoomId> {
    rooms.iter().map(|r| r.id.clone()).collect()
}

fn reset_of(batch: Vec<RoomListDiff>) -> Vec<RoomSummary> {
    match batch.as_slice() {
        [RoomListDiff::Reset(rooms)] => rooms.clone(),
        other => panic!("expected one Reset, got {other:?}"),
    }
}

fn appended(batch: Vec<TimelineDiff>) -> Vec<Message> {
    batch
        .into_iter()
        .flat_map(|d| match d {
            TimelineDiff::Append(list) => list,
            other => panic!("expected only Append, got {other:?}"),
        })
        .collect()
}

fn timeline_with(count: usize) -> Arc<FakeTimeline> {
    Arc::new(
        FakeTimeline::new(me(), Timestamp::from_millis(NOW)).with_room(room_id(1), history(count)),
    )
}

fn open_room(fake: &FakeTimeline, n: usize) -> BoxStream<'static, Vec<TimelineDiff>> {
    let mut batches = block_on(fake.open(room_id(n))).expect("opens");
    assert!(
        pending(&mut batches).is_some(),
        "the first batch is ready at once"
    );
    batches
}

// ------------------------------------------------------------ FakeRoomList

#[test]
fn the_room_list_fake_honours_the_room_list_contract() {
    contract::room_list(|rooms| Arc::new(FakeRoomList::new(rooms))).expect("contract");
}

#[test]
fn the_room_list_fake_starts_with_a_sorted_reset() {
    let fake = FakeRoomList::new(vec![
        room(1, 100),
        room(2, 300).low_priority(),
        room(3, 200),
    ]);
    let mut batches = block_on(fake.subscribe()).expect("subscribes");
    let rooms = reset_of(pending(&mut batches).expect("a first batch"));
    assert_eq!(ids(&rooms), vec![room_id(3), room_id(1), room_id(2)]);
}

#[test]
fn the_room_list_fake_has_nothing_more_until_a_push() {
    let fake = FakeRoomList::new(vec![room(1, 100)]);
    let mut batches = block_on(fake.subscribe()).expect("subscribes");
    assert!(pending(&mut batches).is_some());
    assert!(pending(&mut batches).is_none());
}

#[test]
fn a_push_reaches_the_subscriber_as_one_batch_of_the_same_diffs() {
    let fake = FakeRoomList::new(vec![room(1, 100)]);
    let mut batches = block_on(fake.subscribe()).expect("subscribes");
    let _ = pending(&mut batches);
    let diffs = vec![
        RoomListDiff::Insert(room(2, 200)),
        RoomListDiff::Remove(room_id(1)),
    ];
    fake.push(diffs.clone());
    assert_eq!(pending(&mut batches), Some(diffs));
    assert!(pending(&mut batches).is_none());
}

#[test]
fn a_push_reaches_every_subscriber() {
    let fake = FakeRoomList::new(vec![]);
    let mut one = block_on(fake.subscribe()).expect("subscribes");
    let mut two = block_on(fake.subscribe()).expect("subscribes");
    let _ = (pending(&mut one), pending(&mut two));
    let diffs = vec![RoomListDiff::Insert(room(5, 5))];
    fake.push(diffs.clone());
    assert_eq!(pending(&mut one), Some(diffs.clone()));
    assert_eq!(pending(&mut two), Some(diffs));
}

#[test]
fn a_late_subscriber_starts_from_the_pushed_state() {
    let fake = FakeRoomList::new(vec![room(1, 100), room(2, 200)]);
    fake.push(vec![
        RoomListDiff::Insert(room(3, 300)),
        RoomListDiff::Remove(room_id(2)),
    ]);
    let mut batches = block_on(fake.subscribe()).expect("subscribes");
    let rooms = reset_of(pending(&mut batches).expect("a first batch"));
    assert_eq!(ids(&rooms), vec![room_id(3), room_id(1)]);
}

#[test]
fn a_dropped_subscriber_does_not_stop_the_others() {
    let fake = FakeRoomList::new(vec![]);
    let gone = block_on(fake.subscribe()).expect("subscribes");
    let mut kept = block_on(fake.subscribe()).expect("subscribes");
    let _ = pending(&mut kept);
    drop(gone);
    let diffs = vec![RoomListDiff::Insert(room(1, 1))];
    fake.push(diffs.clone());
    assert_eq!(pending(&mut kept), Some(diffs));
}

#[test]
fn a_failing_room_list_refuses_to_subscribe() {
    let fake = FakeRoomList::new(vec![room(1, 1)]).failing(RoomListError::Unavailable);
    assert_eq!(
        block_on(fake.subscribe()).err(),
        Some(RoomListError::Unavailable)
    );
}

// ------------------------------------------------------------ FakeTimeline

#[test]
fn the_timeline_fake_honours_the_timeline_contract() {
    contract::timeline(|user, room, messages| {
        Arc::new(FakeTimeline::new(user, Timestamp::from_millis(NOW)).with_room(room, messages))
    })
    .expect("contract");
}

#[test]
fn a_page_is_30_messages() {
    let fake = timeline_with(TIMELINE_PAGE_SIZE + 5);
    let mut batches = block_on(fake.open(room_id(1))).expect("opens");
    let first = pending(&mut batches).expect("a first batch");
    match first.as_slice() {
        [TimelineDiff::Reset(list)] => assert_eq!(list.len(), 30),
        other => panic!("expected one Reset, got {other:?}"),
    }
}

#[test]
fn the_echo_carries_the_user_and_the_clock_of_the_fake() {
    let fake = timeline_with(2);
    let mut batches = open_room(&fake, 1);
    block_on(fake.send_text(room_id(1), "hi".into())).expect("sends");
    let echo = appended(pending(&mut batches).expect("an echo")).remove(0);
    assert_eq!(echo.sender, me());
    assert_eq!(echo.timestamp, Timestamp::from_millis(NOW));
}

#[test]
fn every_subscriber_of_the_room_gets_the_echo() {
    let fake = timeline_with(2);
    let mut one = open_room(&fake, 1);
    let mut two = open_room(&fake, 1);
    block_on(fake.send_text(room_id(1), "hi".into())).expect("sends");
    assert!(pending(&mut one).is_some());
    assert!(pending(&mut two).is_some());
}

#[test]
fn confirming_replaces_each_echo_by_a_server_event_with_the_same_transaction() {
    let fake = timeline_with(2);
    let mut batches = open_room(&fake, 1);
    block_on(fake.send_text(room_id(1), "first".into())).expect("sends");
    block_on(fake.send_text(room_id(1), "second".into())).expect("sends");
    let echoes = [
        appended(pending(&mut batches).expect("echo")).remove(0),
        appended(pending(&mut batches).expect("echo")).remove(0),
    ];
    fake.confirm_sends(&room_id(1));
    let confirmed = appended(pending(&mut batches).expect("confirmation"));
    assert_eq!(confirmed.len(), 2);
    for (echo, done) in echoes.iter().zip(&confirmed) {
        assert_eq!(done.txn_id(), echo.txn_id());
        assert_eq!(done.body, echo.body);
        assert_eq!(done.sender, me());
        assert_eq!(done.send_state, SendState::Sent);
        assert!(done.event_id().is_some());
    }
    assert_ne!(confirmed[0].event_id(), confirmed[1].event_id());
}

#[test]
fn confirming_with_nothing_pending_sends_nothing() {
    let fake = timeline_with(2);
    let mut batches = open_room(&fake, 1);
    fake.confirm_sends(&room_id(1));
    assert!(pending(&mut batches).is_none());
}

#[test]
fn a_send_is_confirmed_only_once() {
    let fake = timeline_with(2);
    let mut batches = open_room(&fake, 1);
    block_on(fake.send_text(room_id(1), "once".into())).expect("sends");
    let _ = pending(&mut batches);
    fake.confirm_sends(&room_id(1));
    let _ = pending(&mut batches);
    fake.confirm_sends(&room_id(1));
    assert!(pending(&mut batches).is_none());
}

#[test]
fn failing_the_sends_marks_each_echo_failed_in_place() {
    let fake = timeline_with(2);
    let mut batches = open_room(&fake, 1);
    block_on(fake.send_text(room_id(1), "lost".into())).expect("sends");
    let echo = appended(pending(&mut batches).expect("echo")).remove(0);
    fake.fail_sends(&room_id(1));
    let batch = pending(&mut batches).expect("a failure");
    match batch.as_slice() {
        [TimelineDiff::Update(failed)] => {
            assert_eq!(failed.txn_id(), echo.txn_id());
            assert_eq!(failed.body, "lost");
            assert_eq!(failed.send_state, SendState::Failed);
        }
        other => panic!("expected one Update, got {other:?}"),
    }
}

#[test]
fn a_confirmed_message_is_in_the_history_of_the_next_opening() {
    let fake = timeline_with(3);
    let _keep = open_room(&fake, 1);
    block_on(fake.send_text(room_id(1), "kept".into())).expect("sends");
    fake.confirm_sends(&room_id(1));
    let mut batches = block_on(fake.open(room_id(1))).expect("opens");
    match pending(&mut batches).expect("a first batch").as_slice() {
        [TimelineDiff::Reset(list)] => {
            assert_eq!(list.len(), 4);
            assert_eq!(list[3].body, "kept");
            assert_eq!(list[3].send_state, SendState::Sent);
        }
        other => panic!("expected one Reset, got {other:?}"),
    }
}

#[test]
fn a_failed_message_is_not_in_the_history() {
    let fake = timeline_with(3);
    let _keep = open_room(&fake, 1);
    block_on(fake.send_text(room_id(1), "lost".into())).expect("sends");
    fake.fail_sends(&room_id(1));
    let mut batches = block_on(fake.open(room_id(1))).expect("opens");
    match pending(&mut batches).expect("a first batch").as_slice() {
        [TimelineDiff::Reset(list)] => assert_eq!(list.len(), 3),
        other => panic!("expected one Reset, got {other:?}"),
    }
}

#[test]
fn a_push_reaches_the_subscribers_of_that_room_only() {
    let fake = Arc::new(
        FakeTimeline::new(me(), Timestamp::from_millis(NOW))
            .with_room(room_id(1), history(2))
            .with_room(room_id(2), history(2)),
    );
    let mut one = open_room(&fake, 1);
    let mut two = open_room(&fake, 2);
    let late = Message::remote(
        EventId::parse("$late").expect("valid event id"),
        bob(),
        Timestamp::from_millis(5_000),
        "late",
    );
    let diffs = vec![TimelineDiff::Append(vec![late])];
    fake.push(&room_id(1), diffs.clone());
    assert_eq!(pending(&mut one), Some(diffs));
    assert!(pending(&mut two).is_none());
}

#[test]
fn a_failing_timeline_refuses_every_call() {
    let fake = FakeTimeline::new(me(), Timestamp::from_millis(NOW))
        .with_room(room_id(1), history(2))
        .failing(TimelineError::Unavailable);
    assert_eq!(
        block_on(fake.open(room_id(1))).err(),
        Some(TimelineError::Unavailable)
    );
    assert_eq!(
        block_on(fake.paginate_back(room_id(1))),
        Err(TimelineError::Unavailable)
    );
    assert_eq!(
        block_on(fake.send_text(room_id(1), "x".into())),
        Err(TimelineError::Unavailable)
    );
}
