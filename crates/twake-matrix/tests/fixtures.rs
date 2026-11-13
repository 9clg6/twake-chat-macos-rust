//! The mock homeserver of the read specs, checked against the SDK alone (no adapter of ours):
//! if these fail, the fixtures lie and no spec of `room_list.rs` / `timeline.rs` means anything.
#![recursion_limit = "256"]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use std::time::Duration;

use common::history::History;
use common::seed::{AccountData, RoomSeed, SlidingScript, text_event};
use futures::{StreamExt, pin_mut};
use matrix_sdk::room::MessagesOptions;
use matrix_sdk::ruma::{room_id, uint};
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use matrix_sdk_ui::RoomListService;

async fn first_sync(server: &MatrixMockServer) -> matrix_sdk::Client {
    let client = server.client_builder().build().await;
    let service = RoomListService::new(client.clone()).await.unwrap();
    let sync = service.sync();
    pin_mut!(sync);
    tokio::time::timeout(Duration::from_secs(10), sync.next())
        .await
        .expect("the first sliding sync response arrives")
        .expect("the sync stream is alive")
        .expect("the first response is accepted");
    client
}

#[tokio::test]
async fn the_seeded_rooms_reach_the_client_with_their_counts_and_tags() {
    let server = MatrixMockServer::new().await;
    let rooms = vec![
        RoomSeed::new("!group:example.org")
            .named("Group")
            .unread(3, 1)
            .with_message(text_event("$m1", "@bob:example.org", 5_000, "hello")),
        RoomSeed::new("!dm:example.org")
            .named("Bob")
            .direct()
            .avatar("mxc://example.org/bob"),
        RoomSeed::new("!fav:example.org").named("Favourite"),
    ];
    let data = AccountData::default()
        .direct("!dm:example.org", "@bob:example.org")
        .muting("!fav:example.org")
        .tagging("!fav:example.org", "m.favourite");
    SlidingScript::new(rooms, data).mount(&server).await;

    let client = first_sync(&server).await;

    let group = client.get_room(room_id!("!group:example.org")).unwrap();
    assert_eq!(
        group.cached_display_name().map(|n| n.to_string()),
        Some("Group".into())
    );
    let counts = group.unread_notification_counts();
    assert_eq!((counts.notification_count, counts.highlight_count), (3, 1));
    let dm = client.get_room(room_id!("!dm:example.org")).unwrap();
    assert!(dm.is_direct().await.unwrap());
    let fav = client.get_room(room_id!("!fav:example.org")).unwrap();
    assert!(fav.is_favourite());
    let mode = client
        .notification_settings()
        .await
        .get_user_defined_room_notification_mode(fav.room_id())
        .await;
    assert_eq!(
        mode,
        Some(matrix_sdk::notification_settings::RoomNotificationMode::Mute)
    );
}

#[tokio::test]
async fn the_history_pages_back_by_token_to_the_start() {
    let server = MatrixMockServer::new().await;
    let events: Vec<_> = (0..12)
        .map(|i| text_event(&format!("$e{i}"), "@bob:example.org", 1_000 + i, "x"))
        .collect();
    let history = History::new("!h:example.org", events);
    SlidingScript::new(vec![history.seed()], AccountData::default())
        .mount(&server)
        .await;
    history.mount(&server).await;

    let client = first_sync(&server).await;
    let room = client.get_room(room_id!("!h:example.org")).unwrap();

    // 12 events, 5 in the sync: `/messages` from `t7` gives $e6..$e0 newest first.
    let mut options = MessagesOptions::backward().from(Some("t7"));
    options.limit = uint!(4);
    let page = room.messages(options).await.unwrap();
    let ids: Vec<String> = page
        .chunk
        .iter()
        .map(|e| e.event_id().unwrap().to_string())
        .collect();
    assert_eq!(ids, ["$e6", "$e5", "$e4", "$e3"]);
    assert_eq!(page.end.as_deref(), Some("t3"));

    let mut options = MessagesOptions::backward().from(page.end.as_deref());
    options.limit = uint!(30);
    let last = room.messages(options).await.unwrap();
    assert_eq!(last.chunk.len(), 3);
    assert_eq!(last.end, None, "no token behind the oldest event");
}
