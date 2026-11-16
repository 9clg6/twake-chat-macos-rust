//! A 401 in the middle of the sync reaches whoever listens to `expiry(&Client)` (A033 "Not done":
//! the retries stop on `NotSignedIn` but nobody was told, the list froze; the session machine
//! needs the `Unauthorized401` event). D396: the server ended the session.
//! Mock homeserver only.
#![recursion_limit = "256"]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use std::time::Duration;

use common::seed::{AccountData, RoomSeed, SlidingScript, signed_in, text_event};
use futures::StreamExt;
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use twake_app::ports::RoomListPort;
use twake_domain::UserId;
use twake_matrix::{Expiry, MatrixRoomList, expiry};

const WAIT: Duration = Duration::from_secs(15);

#[tokio::test]
async fn a_token_refused_while_the_list_is_syncing_is_reported_as_an_expiry() {
    let server = MatrixMockServer::new().await;
    let script = SlidingScript::new(
        vec![
            RoomSeed::new("!a:example.org")
                .named("Alpha")
                .with_message(text_event("$m", "@bob:example.org", 5_000, "hi")),
        ],
        AccountData::default(),
    );
    script.mount(&server).await;
    let dir = tempfile::tempdir().unwrap();
    let client = signed_in(
        &server,
        dir.path(),
        &UserId::parse("@alice:example.org").unwrap(),
    )
    .await;
    let mut lost = Box::pin(expiry(&client));
    let port = MatrixRoomList::new(client);
    let mut stream = port.subscribe().await.unwrap();
    assert!(stream.next().await.is_some(), "the first list arrives");

    script.fail_next(1, 401);

    let told = tokio::time::timeout(WAIT, lost.next()).await;
    assert_eq!(
        told.expect("the expiry must be reported, not left to freeze the list"),
        Some(Expiry::Hard)
    );
}
