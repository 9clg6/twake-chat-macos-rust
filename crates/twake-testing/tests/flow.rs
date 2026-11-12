//! The vertical slice without a network: the fakes feed the use-case states, as the screens
//! will (ARCHITECTURE.md section 5: a stream of batches applied to one model).
//!
//! Need: `docs/spec/features/roomlist.md` D190 (order), D192/D109 (chips) and
//! `timeline.md` D495 (echo then sent), D532 (older pages by themselves), e2e "day-divider".
//!
//! Public items used are listed in `docs/dev/app-core-api-sketch.md`.

#![cfg(test)]

use futures::executor::block_on;
use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use twake_app::ports::{RoomListPort, TimelinePort};
use twake_app::room_list::{RoomListState, RoomListStatus};
use twake_app::timeline::{TimelineState, TimelineStatus};
use twake_domain::room_list::{RoomFilter, RoomKind, RoomListDiff, RoomSummary};
use twake_domain::timeline::{Message, MessageKey, SendState, TimelineDiff, TimelineItem};
use twake_domain::{EventId, RoomId, Timestamp, UserId};
use twake_testing::{FakeRoomList, FakeTimeline};

fn room_id(n: usize) -> RoomId {
    RoomId::parse(&format!("!flow{n}:example.org")).expect("valid room id")
}

fn me() -> UserId {
    UserId::parse("@me:example.org").expect("valid user id")
}

/// Everything already waiting on the stream, applied to `apply`.
fn drain<T>(batches: &mut BoxStream<'static, Vec<T>>, mut apply: impl FnMut(Vec<T>)) {
    while let Some(Some(batch)) = batches.next().now_or_never() {
        apply(batch);
    }
}

fn messages_of(state: &TimelineState) -> Vec<Message> {
    state
        .items()
        .into_iter()
        .filter_map(|i| match i {
            TimelineItem::Message(m) => Some(m),
            _ => None,
        })
        .collect()
}

#[test]
fn the_room_list_screen_follows_the_fake() {
    let fake = FakeRoomList::new(vec![
        RoomSummary::new(room_id(1), "Alpha", RoomKind::Group)
            .with_last_activity(Timestamp::from_millis(100)),
        RoomSummary::new(room_id(2), "Beta", RoomKind::Direct)
            .with_last_activity(Timestamp::from_millis(200)),
    ]);
    let mut state = RoomListState::new();
    assert_eq!(state.status(), RoomListStatus::Loading);
    let mut batches = block_on(fake.subscribe()).expect("subscribes");
    drain(&mut batches, |b| state.apply(b));
    assert_eq!(state.status(), RoomListStatus::Ready);
    let names: Vec<String> = state.view().into_iter().map(|r| r.name).collect();
    assert_eq!(names, vec!["Beta", "Alpha"]);

    fake.push(vec![RoomListDiff::Move {
        id: room_id(1),
        last_activity: Timestamp::from_millis(900),
    }]);
    drain(&mut batches, |b| state.apply(b));
    let names: Vec<String> = state.view().into_iter().map(|r| r.name).collect();
    assert_eq!(names, vec!["Alpha", "Beta"]);

    state.set_filter(RoomFilter::Directs);
    let names: Vec<String> = state.view().into_iter().map(|r| r.name).collect();
    assert_eq!(names, vec!["Beta"]);
}

/// A conversation screen on a fake: the stream, the state, and a way to bring them up to date.
struct Screen {
    fake: FakeTimeline,
    batches: BoxStream<'static, Vec<TimelineDiff>>,
    state: TimelineState,
}

impl Screen {
    fn open(history_len: i64) -> Self {
        let bob = UserId::parse("@bob:example.org").expect("valid user id");
        let history: Vec<Message> = (0..history_len)
            .map(|i| {
                Message::remote(
                    EventId::parse(&format!("$flow{i}")).expect("valid event id"),
                    bob.clone(),
                    Timestamp::from_millis(1_700_000_000_000 + i as u64 * 1_000),
                    format!("hello {i}"),
                )
            })
            .collect();
        let fake = FakeTimeline::new(me(), Timestamp::from_millis(1_700_000_100_000))
            .with_room(room_id(1), history);
        let batches = block_on(fake.open(room_id(1))).expect("opens");
        let mut screen = Self {
            fake,
            batches,
            state: TimelineState::new(0),
        };
        screen.sync();
        screen
    }

    fn sync(&mut self) {
        let state = &mut self.state;
        drain(&mut self.batches, |b| {
            b.into_iter().for_each(|d| state.apply(d))
        });
    }

    fn bodies(&self) -> Vec<Message> {
        messages_of(&self.state)
    }
}

#[test]
fn opening_a_long_conversation_shows_its_last_page() {
    let screen = Screen::open(45);
    assert_eq!(screen.state.status(), TimelineStatus::Ready);
    assert_eq!(screen.bodies().len(), 30);
    assert_eq!(screen.bodies()[0].body, "hello 15");
    assert!(!screen.state.reached_start());
}

#[test]
fn scrolling_back_shows_the_older_messages_and_the_start() {
    let mut screen = Screen::open(45);
    block_on(screen.fake.paginate_back(room_id(1))).expect("paginates");
    screen.sync();
    assert_eq!(screen.bodies().len(), 45);
    assert_eq!(screen.bodies()[0].body, "hello 0");
    assert!(screen.state.reached_start());
}

#[test]
fn a_sent_message_shows_at_once_then_keeps_its_row_when_confirmed() {
    let mut screen = Screen::open(3);
    block_on(screen.fake.send_text(room_id(1), "my answer".into())).expect("sends");
    screen.sync();
    let sending = screen.bodies();
    assert_eq!(sending.len(), 4);
    assert_eq!(sending[3].send_state, SendState::Sending);

    screen.fake.confirm_sends(&room_id(1));
    screen.sync();
    let sent = screen.bodies();
    assert_eq!(sent.len(), 4, "the echo was replaced, not doubled");
    assert_eq!(sent[3].body, "my answer");
    assert_eq!(sent[3].send_state, SendState::Sent);
    assert!(sent[3].event_id().is_some());
}

#[test]
fn a_failed_send_can_be_discarded_from_the_screen() {
    let mut screen = Screen::open(0);
    assert_eq!(screen.state.status(), TimelineStatus::Empty);

    block_on(screen.fake.send_text(room_id(1), "oops".into())).expect("sends");
    screen.fake.fail_sends(&room_id(1));
    screen.sync();
    let list = screen.bodies();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].send_state, SendState::Failed);

    let txn = list[0]
        .txn_id()
        .expect("a local message has a transaction")
        .clone();
    screen
        .state
        .apply(TimelineDiff::Remove(MessageKey::Txn(txn)));
    assert_eq!(screen.state.status(), TimelineStatus::Empty);
}
