//! A room the user read without answering goes back to no unread (A033 "Not done": the read point
//! was the last own message only). Need: the React `AGENTS.md` "Unread" and `readState.ts`: the
//! read point is the furthest of the last own message and the user's own `m.read` receipt (public
//! or private); the receipt of someone else, or on an event the room does not hold, moves nothing.
//! Mock homeserver only: the receipts extension of the scripted sliding sync (`seed.rs`).
#![recursion_limit = "256"]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use std::time::Duration;

use common::seed::{AccountData, RoomSeed, SlidingScript, signed_in, text_event};
use futures::StreamExt;
use futures::stream::BoxStream;
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use tempfile::TempDir;
use twake_app::ports::RoomListPort;
use twake_domain::UserId;
use twake_domain::read_state::UnreadCount;
use twake_domain::room_list::{RoomListDiff, RoomSummary, apply_diff};
use twake_matrix::MatrixRoomList;

const WAIT: Duration = Duration::from_secs(15);
const ROOM: &str = "!group:example.org";
const ALICE: &str = "@alice:example.org";
const BOB: &str = "@bob:example.org";

struct Served {
    port: MatrixRoomList,
    script: SlidingScript,
    _server: MatrixMockServer,
    _dir: TempDir,
}

async fn serve(rooms: Vec<RoomSeed>) -> Served {
    let server = MatrixMockServer::new().await;
    let script = SlidingScript::new(rooms, AccountData::default());
    script.mount(&server).await;
    let dir = tempfile::tempdir().unwrap();
    let alice = UserId::parse(ALICE).unwrap();
    let client = signed_in(&server, dir.path(), &alice).await;
    Served {
        port: MatrixRoomList::new(client),
        script,
        _server: server,
        _dir: dir,
    }
}

fn counts(unread: u32) -> UnreadCount {
    UnreadCount {
        unread,
        mentions: 0,
        marked_unread: false,
    }
}

/// Three messages of bob, `$1` to `$3`.
fn three_from_bob() -> RoomSeed {
    RoomSeed::new(ROOM)
        .named("Team")
        .with_message(text_event("$1", BOB, 1_000, "one"))
        .with_message(text_event("$2", BOB, 2_000, "two"))
        .with_message(text_event("$3", BOB, 3_000, "three"))
}

async fn first_unread(room: RoomSeed) -> UnreadCount {
    let served = serve(vec![room]).await;
    let mut stream = tokio::time::timeout(WAIT, served.port.subscribe())
        .await
        .expect("subscribe answers")
        .unwrap();
    let batch = tokio::time::timeout(WAIT, stream.next())
        .await
        .expect("a first batch")
        .unwrap();
    let [RoomListDiff::Reset(rooms)] = batch.as_slice() else {
        panic!("the first batch must be one Reset, got {batch:?}");
    };
    rooms.iter().find(|r| r.id.as_str() == ROOM).unwrap().unread
}

#[tokio::test]
async fn a_room_read_without_answering_has_no_unread() {
    let room = three_from_bob().read_by(ALICE, "$3");
    assert_eq!(first_unread(room).await, counts(0));
}

#[tokio::test]
async fn only_what_came_after_the_receipt_is_unread() {
    let room = three_from_bob().read_by(ALICE, "$2");
    assert_eq!(first_unread(room).await, counts(1));
}

#[tokio::test]
async fn a_private_receipt_reads_too() {
    let room = three_from_bob().privately_read_by(ALICE, "$3");
    assert_eq!(first_unread(room).await, counts(0));
}

#[tokio::test]
async fn the_receipt_of_someone_else_reads_nothing_for_the_user() {
    let room = three_from_bob().read_by("@carol:example.org", "$3");
    assert_eq!(first_unread(room).await, counts(3));
}

#[tokio::test]
async fn a_receipt_on_an_event_the_room_does_not_hold_reads_nothing() {
    let room = three_from_bob().read_by(ALICE, "$unknown");
    assert_eq!(first_unread(room).await, counts(3));
}

async fn until(
    stream: &mut BoxStream<'static, Vec<RoomListDiff>>,
    list: &mut Vec<RoomSummary>,
    done: impl Fn(&[RoomSummary]) -> bool,
) -> bool {
    let reading = async {
        while !done(list) {
            for diff in stream.next().await? {
                *list = apply_diff(list, diff);
            }
        }
        Some(())
    };
    matches!(tokio::time::timeout(WAIT, reading).await, Ok(Some(())))
}

fn unread_of(list: &[RoomSummary]) -> Option<UnreadCount> {
    list.iter()
        .find(|r| r.id.as_str() == ROOM)
        .map(|r| r.unread)
}

#[tokio::test]
async fn reading_the_room_on_another_device_brings_it_back_to_zero_live() {
    let served = serve(vec![three_from_bob()]).await;
    let mut stream = served.port.subscribe().await.unwrap();
    let mut list = vec![];
    assert!(until(&mut stream, &mut list, |l| unread_of(l) == Some(counts(3))).await);

    // The receipt of alice arrives in a later sync, with no new message.
    served
        .script
        .push(vec![RoomSeed::new(ROOM).read_by(ALICE, "$3")]);

    assert!(
        until(&mut stream, &mut list, |l| unread_of(l) == Some(counts(0))).await,
        "the row must go back to 0 once the receipt arrives; got {:?}",
        unread_of(&list)
    );
}
