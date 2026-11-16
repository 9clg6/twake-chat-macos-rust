//! The room list and timeline ports: their exact shape (A009) and their errors.
//!
//! Need: ARCHITECTURE.md section 4 (ports are traits whose methods return boxed `Send`
//! futures, held by the UI as `Arc<dyn Port + Send + Sync>`; streams carry batches of
//! diffs; one error enum per port, `thiserror`), section 5 (pages of 30 messages) and A009,
//! A011 Q6 (no tokio in `twake-app`). The stubs below implement the traits the way an
//! adapter must; if the trait changes shape they stop compiling.
//!
//! Public items used are listed in `docs/dev/app-core-api-sketch.md`.

#![cfg(test)]

use std::sync::Arc;

use futures::executor::block_on;
use futures::future::BoxFuture;
use futures::stream::{self, BoxStream, StreamExt};
use twake_app::ports::{
    RoomListError, RoomListPort, TIMELINE_PAGE_SIZE, TimelineError, TimelinePort,
};
use twake_domain::room_list::{RoomKind, RoomListDiff, RoomSummary};
use twake_domain::timeline::{Message, TimelineDiff};
use twake_domain::{EventId, RoomId, Timestamp, UserId};

// ------------------------------------------------------------ stubs

fn room_id(n: usize) -> RoomId {
    RoomId::parse(&format!("!r{n}:example.org")).expect("valid room id")
}

fn summary() -> RoomSummary {
    RoomSummary::new(room_id(1), "General", RoomKind::Group)
}

fn message() -> Message {
    let sender = UserId::parse("@alice:example.org").expect("valid user id");
    let event = EventId::parse("$e1").expect("valid event id");
    Message::remote(event, sender, Timestamp::from_millis(1), "hi")
}

struct StubRoomList;

impl RoomListPort for StubRoomList {
    fn subscribe(
        &self,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Vec<RoomListDiff>>, RoomListError>> {
        Box::pin(async {
            let first = vec![RoomListDiff::Reset(vec![summary()])];
            Ok(stream::iter(vec![first]).boxed())
        })
    }
}

struct StubTimeline;

impl TimelinePort for StubTimeline {
    fn open(
        &self,
        room: RoomId,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Vec<TimelineDiff>>, TimelineError>> {
        Box::pin(async move {
            if room != room_id(1) {
                return Err(TimelineError::RoomNotFound);
            }
            let first = vec![TimelineDiff::Reset(vec![message()])];
            Ok(stream::iter(vec![first]).boxed())
        })
    }

    fn paginate_back(&self, _room: RoomId) -> BoxFuture<'_, Result<(), TimelineError>> {
        Box::pin(async { Ok(()) })
    }

    fn send_text(&self, _room: RoomId, _body: String) -> BoxFuture<'_, Result<(), TimelineError>> {
        Box::pin(async { Err(TimelineError::Unavailable) })
    }
}

fn assert_send<T: Send>(_: &T) {}

fn assert_send_sync<T: Send + Sync + ?Sized>() {}

// ------------------------------------------------------------ dyn compatibility (A009)

#[test]
fn the_room_list_port_is_held_as_an_arc_dyn() {
    let port: Arc<dyn RoomListPort> = Arc::new(StubRoomList);
    let mut batches = block_on(port.subscribe()).expect("subscribes");
    let first = block_on(batches.next()).expect("a first batch");
    assert_eq!(first, vec![RoomListDiff::Reset(vec![summary()])]);
}

#[test]
fn the_timeline_port_is_held_as_an_arc_dyn() {
    let port: Arc<dyn TimelinePort> = Arc::new(StubTimeline);
    let mut batches = block_on(port.open(room_id(1))).expect("opens");
    let first = block_on(batches.next()).expect("a first batch");
    assert_eq!(first, vec![TimelineDiff::Reset(vec![message()])]);
    assert_eq!(block_on(port.paginate_back(room_id(1))), Ok(()));
    assert_eq!(
        block_on(port.send_text(room_id(1), "hi".into())),
        Err(TimelineError::Unavailable)
    );
}

#[test]
fn a_port_can_be_shared_between_threads() {
    assert_send_sync::<dyn RoomListPort>();
    assert_send_sync::<dyn TimelinePort>();
}

#[test]
fn the_futures_of_a_port_are_send() {
    let list = StubRoomList;
    let timeline = StubTimeline;
    assert_send(&list.subscribe());
    assert_send(&timeline.open(room_id(1)));
    assert_send(&timeline.paginate_back(room_id(1)));
    assert_send(&timeline.send_text(room_id(1), String::new()));
}

#[test]
fn the_streams_of_a_port_are_send() {
    let port: Arc<dyn TimelinePort> = Arc::new(StubTimeline);
    let batches = block_on(port.open(room_id(1))).expect("opens");
    assert_send(&batches);
}

#[test]
fn a_port_answers_from_another_thread() {
    let port: Arc<dyn TimelinePort> = Arc::new(StubTimeline);
    let worker = std::thread::spawn(move || block_on(port.open(room_id(1))).is_ok());
    assert!(worker.join().expect("thread ends"));
}

// ------------------------------------------------------------ errors

#[test]
fn the_page_of_the_timeline_is_30_messages() {
    assert_eq!(TIMELINE_PAGE_SIZE, 30);
}

#[test]
fn room_list_errors_are_told_apart_and_have_a_message() {
    let (a, b) = (RoomListError::Unavailable, RoomListError::NotSignedIn);
    assert_ne!(a, b);
    assert_eq!(a.clone(), a);
    assert!(!a.to_string().is_empty());
    assert!(!b.to_string().is_empty());
    assert_ne!(a.to_string(), b.to_string());
}

#[test]
fn timeline_errors_are_told_apart_and_have_a_message() {
    let (a, b) = (TimelineError::RoomNotFound, TimelineError::Unavailable);
    assert_ne!(a, b);
    assert_eq!(b.clone(), b);
    assert!(!a.to_string().is_empty());
    assert!(!b.to_string().is_empty());
    assert_ne!(a.to_string(), b.to_string());
}
