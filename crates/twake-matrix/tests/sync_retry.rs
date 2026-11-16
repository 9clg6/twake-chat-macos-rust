//! The sync comes back by itself after an error (ARCHITECTURE section 7: "on error it restarts
//! with back-off"; wiring lot, A031 decision 1: today the poll task stops at the first error and
//! only a new `subscribe` would start it again, so a hiccup of the network freezes the list).
//!
//! Public surface used: `MatrixRoomList::new(Client)` and `RoomListPort::subscribe`, as today.
//! The default first retry waits at most 2 s (the spec waits 15 s).
//! Mock homeserver only (wiremock through `MatrixMockServer`), no real account.
#![recursion_limit = "256"]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use std::time::Duration;

use common::seed::{AccountData, RoomSeed, SlidingScript, signed_in, text_event};
use futures::StreamExt;
use futures::stream::BoxStream;
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use twake_app::ports::RoomListPort;
use twake_domain::UserId;
use twake_domain::room_list::{RoomListDiff, RoomSummary, apply_diff};
use twake_matrix::MatrixRoomList;

const WAIT: Duration = Duration::from_secs(15);

fn alice() -> UserId {
    UserId::parse("@alice:example.org").unwrap()
}

fn room(id: &str, name: &str) -> RoomSeed {
    RoomSeed::new(id)
        .named(name)
        .with_message(text_event("$m", "@bob:example.org", 5_000, "hi"))
}

/// Reads the stream until `done` holds for the list built from its batches.
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

#[tokio::test]
async fn the_list_keeps_updating_after_the_server_refused_the_sync_a_few_times() {
    let server = MatrixMockServer::new().await;
    let script = SlidingScript::new(vec![room("!a:example.org", "Alpha")], AccountData::default());
    script.mount(&server).await;
    let dir = tempfile::tempdir().unwrap();
    let port = MatrixRoomList::new(signed_in(&server, dir.path(), &alice()).await);

    let mut stream = port.subscribe().await.unwrap();
    let mut list = vec![];
    assert!(
        until(&mut stream, &mut list, |l| l.len() == 1).await,
        "the first list arrives"
    );

    // The server hiccups, then recovers and has a new room.
    script.fail_next(3, 403);
    script.later(vec![room("!b:example.org", "Beta")]);

    assert!(
        until(&mut stream, &mut list, |l| l.iter().any(|r| r.name == "Beta")).await,
        "a room created after the outage must reach the same stream; got {list:?}"
    );
}
