//! The delay between two restarts of the sync grows after each error and is capped
//! (ARCHITECTURE section 7: "restarts with back-off"; wiring lot).
//!
//! Expected public surface of `twake_matrix` (new):
//! - `Backoff::new(initial: Duration, max: Duration) -> Backoff`: the delay before the first
//!   restart is `initial`, each next one is longer (doubling is the reference), never above `max`;
//! - `Backoff::delay(&self, attempt: u32) -> Duration`: attempt 0 is the first restart; pure,
//!   no randomness (so the value can be asserted);
//! - `MatrixRoomList::with_backoff(client: Client, backoff: Backoff) -> MatrixRoomList`.
//!
//! Kept in its own test file: the specs of `sync_retry.rs` need none of these names.
#![recursion_limit = "256"]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use std::time::Duration;

use common::seed::{AccountData, RoomSeed, SlidingScript, signed_in, text_event};
use futures::StreamExt;
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use twake_app::ports::RoomListPort;
use twake_domain::UserId;
use twake_matrix::{Backoff, MatrixRoomList};

const MS: fn(u64) -> Duration = Duration::from_millis;

#[test]
fn the_first_restart_waits_the_initial_delay() {
    assert_eq!(Backoff::new(MS(100), MS(5_000)).delay(0), MS(100));
}

#[test]
fn each_restart_waits_longer_than_the_one_before() {
    let backoff = Backoff::new(MS(100), MS(60_000));
    let delays: Vec<Duration> = (0..6).map(|n| backoff.delay(n)).collect();
    assert!(
        delays.windows(2).all(|w| w[1] > w[0]),
        "not strictly growing: {delays:?}"
    );
    assert_eq!(backoff.delay(3), MS(800), "it doubles");
}

#[test]
fn the_delay_never_goes_above_the_cap() {
    let backoff = Backoff::new(MS(100), MS(1_000));
    assert_eq!(backoff.delay(4), MS(1_000));
    assert_eq!(backoff.delay(50), MS(1_000));
    assert_eq!(backoff.delay(u32::MAX), MS(1_000), "no overflow");
}

#[tokio::test]
async fn the_sync_asks_again_after_the_delays_of_the_backoff() {
    let server = MatrixMockServer::new().await;
    let seed = RoomSeed::new("!a:example.org")
        .named("Alpha")
        .with_message(text_event("$m", "@bob:example.org", 5_000, "hi"));
    let script = SlidingScript::new(vec![seed], AccountData::default());
    script.mount(&server).await;
    let dir = tempfile::tempdir().unwrap();
    let user = UserId::parse("@alice:example.org").unwrap();
    let client = signed_in(&server, dir.path(), &user).await;
    let port = MatrixRoomList::with_backoff(client, Backoff::new(MS(150), MS(5_000)));
    let mut stream = port.subscribe().await.unwrap();
    let _first = stream.next().await.unwrap();

    script.fail_next(4, 403);
    tokio::time::sleep(Duration::from_secs(4)).await;

    let at = script.refused_at();
    assert_eq!(at.len(), 4, "the sync asked again after each refusal");
    let gaps: Vec<Duration> = at.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(
        gaps[0] >= MS(150) && gaps[1] >= gaps[0] + MS(100) && gaps[2] >= gaps[1] + MS(100),
        "the waits must grow (150, 300, 600 ms), got {gaps:?}"
    );
}
