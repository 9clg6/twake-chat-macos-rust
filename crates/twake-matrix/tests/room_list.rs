//! `RoomListPort` on matrix-sdk-ui `RoomListService` (sliding sync, MSC4186), fed by a mock
//! homeserver: no network, no real account (FR-001..FR-010 of the room list, D31, D101, D175, D222).
//!
//! Expected public surface of `twake_matrix`: `MatrixRoomList::new(Client) -> MatrixRoomList`
//! implementing `twake_app::ports::RoomListPort`. The adapter starts the sync it needs at the
//! first `subscribe`; the first batch of a subscription is held back until the first response
//! of the server has been read (a `Reset` of an empty list would lie: D518). Also:
//! `build_client` asks for native sliding sync (`Version::Native`).
#![recursion_limit = "256"]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::seed::{
    AccountData, RoomSeed, SlidingScript, TOKEN, signed_in, state_event, text_event,
};
use futures::StreamExt;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use serde_json::json;
use tempfile::TempDir;
use twake_app::ports::{RoomListError, RoomListPort};
use twake_domain::read_state::UnreadCount;
use twake_domain::room_list::{LastSender, RoomKind, RoomListDiff, RoomSummary, apply_diff};
use twake_domain::{RoomId, Timestamp, UserId};
use twake_matrix::MatrixRoomList;
use twake_testing::contract;
use wiremock::ResponseTemplate;

const WAIT: Duration = Duration::from_secs(15);

fn alice() -> UserId {
    UserId::parse("@alice:example.org").unwrap()
}

/// The adapter, and what must outlive it.
struct Served {
    port: MatrixRoomList,
    script: SlidingScript,
    _server: MatrixMockServer,
    _dir: TempDir,
}

impl RoomListPort for Served {
    fn subscribe(
        &self,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Vec<RoomListDiff>>, RoomListError>> {
        self.port.subscribe()
    }
}

async fn serve(rooms: Vec<RoomSeed>, data: AccountData) -> Served {
    let server = MatrixMockServer::new().await;
    let script = SlidingScript::new(rooms, data);
    script.mount(&server).await;
    let dir = tempfile::tempdir().unwrap();
    let client = signed_in(&server, dir.path(), &alice()).await;
    Served {
        port: MatrixRoomList::new(client),
        script,
        _server: server,
        _dir: dir,
    }
}

async fn next_batch(stream: &mut BoxStream<'static, Vec<RoomListDiff>>) -> Vec<RoomListDiff> {
    tokio::time::timeout(WAIT, stream.next())
        .await
        .expect("a batch arrives")
        .expect("the stream is alive")
}

async fn first_list(served: &Served) -> Vec<RoomSummary> {
    let mut stream = tokio::time::timeout(WAIT, served.subscribe())
        .await
        .expect("subscribe answers")
        .expect("subscribe succeeds");
    match next_batch(&mut stream).await.as_slice() {
        [RoomListDiff::Reset(rooms)] => rooms.clone(),
        other => panic!("the first batch must be one Reset, got {other:?}"),
    }
}

fn find<'a>(rooms: &'a [RoomSummary], id: &str) -> &'a RoomSummary {
    rooms
        .iter()
        .find(|r| r.id.as_str() == id)
        .unwrap_or_else(|| panic!("{id} is not in {rooms:?}"))
}

fn account() -> (Vec<RoomSeed>, AccountData) {
    let bob = "@bob:example.org";
    let rooms = vec![
        // A group: 3 unread, one of them names the user; the newest message is Bob's.
        RoomSeed::new("!group:example.org")
            .named("Team")
            .unread(3, 1)
            .bumped(77)
            .with_member(bob, "Bob Martin")
            .with_message(text_event("$g1", bob, 5_000, "see you at 5")),
        // A direct room, with a picture, whose last message is the user's own.
        RoomSeed::new("!dm:example.org")
            .named("Bob")
            .direct()
            .avatar("mxc://example.org/bob")
            .bumped(5)
            .with_message(text_event("$d1", "@alice:example.org", 8_000, "on my way")),
        // A muted group with unread messages.
        RoomSeed::new("!quiet:example.org")
            .named("Quiet")
            .unread(4, 0)
            .bumped(9)
            .with_member(bob, "Bob Martin")
            .with_message(text_event("$q1", bob, 3_000, "shh")),
    ];
    let data = AccountData::default()
        .direct("!dm:example.org", "@bob:example.org")
        .muting("!quiet:example.org")
        .tagging("!group:example.org", "m.favourite")
        .tagging("!dm:example.org", "m.lowpriority");
    (rooms, data)
}

// ------------------------------------------------------------ the client

#[tokio::test]
async fn a_client_built_by_the_adapter_speaks_native_sliding_sync() {
    let server = MatrixMockServer::new().await;
    let dir = tempfile::tempdir().unwrap();
    let client = common::client(&server, dir.path()).await;
    assert!(
        matches!(
            client.sliding_sync_version(),
            matrix_sdk::sliding_sync::Version::Native
        ),
        "got {:?}",
        client.sliding_sync_version()
    );
}

// ------------------------------------------------------------ the contract

#[tokio::test(flavor = "multi_thread")]
async fn the_room_list_contract_holds_on_the_adapter() {
    let handle = tokio::runtime::Handle::current();
    let verdict = tokio::task::spawn_blocking(move || {
        contract::room_list(|summaries| {
            let (rooms, data) = seeds_of(&summaries);
            let served = handle.block_on(serve(rooms, data));
            Arc::new(served) as Arc<dyn RoomListPort>
        })
    })
    .await
    .unwrap();
    verdict.expect("the room list contract");
}

/// What a server would send for `summaries`: only the fields the contract seeds are carried
/// (name, kind, last message and its author, unread count).
fn seeds_of(summaries: &[RoomSummary]) -> (Vec<RoomSeed>, AccountData) {
    let rooms = summaries.iter().map(seed_of).collect();
    let mut data = AccountData::default();
    for room in summaries.iter().filter(|r| r.kind == RoomKind::Direct) {
        data = data.direct(room.id.as_str(), "@contractpeer:example.org");
    }
    (rooms, data)
}

fn seed_of(summary: &RoomSummary) -> RoomSeed {
    let mut seed = RoomSeed::new(summary.id.as_str()).named(&summary.name);
    if summary.kind == RoomKind::Direct {
        seed = seed.direct();
    }
    if summary.kind == RoomKind::Space {
        seed = seed.space();
    }
    seed = seed.unread(
        u64::from(summary.unread.unread),
        u64::from(summary.unread.mentions),
    );
    let (sender, name) = match &summary.last_sender {
        Some(LastSender::Other(name)) => ("@contractsender:example.org", Some(name.as_str())),
        _ => ("@alice:example.org", None),
    };
    if let Some(name) = name {
        seed = seed.with_member(sender, name);
    }
    match &summary.last_message_preview {
        Some(text) => seed.with_message(text_event(
            "$last",
            sender,
            summary.last_activity.as_millis(),
            text,
        )),
        None => seed,
    }
}

// ------------------------------------------------------------ what a row shows

#[tokio::test]
async fn three_rooms_show_their_name_kind_and_last_message() {
    let (rooms, data) = account();
    let list = first_list(&serve(rooms, data).await).await;
    assert_eq!(list.len(), 3);

    let team = find(&list, "!group:example.org");
    assert_eq!(team.name, "Team");
    assert_eq!(team.kind, RoomKind::Group);
    assert_eq!(team.last_message_preview.as_deref(), Some("see you at 5"));
    assert_eq!(
        team.last_sender,
        Some(LastSender::Other("Bob Martin".into()))
    );

    let dm = find(&list, "!dm:example.org");
    assert_eq!(dm.kind, RoomKind::Direct);
    assert_eq!(
        dm.avatar.as_ref().map(|a| a.as_str()),
        Some("mxc://example.org/bob")
    );
    assert_eq!(dm.last_message_preview.as_deref(), Some("on my way"));
    assert_eq!(dm.last_sender, Some(LastSender::Me));
}

#[tokio::test]
async fn a_room_is_dated_by_its_last_message_not_by_the_bump_of_the_server() {
    // D101: "Not MSC4186 bump_stamp for the shown time".
    let (rooms, data) = account();
    let list = first_list(&serve(rooms, data).await).await;
    assert_eq!(
        find(&list, "!group:example.org").last_activity,
        Timestamp::from_millis(5_000)
    );
    assert_eq!(
        find(&list, "!dm:example.org").last_activity,
        Timestamp::from_millis(8_000)
    );
}

#[tokio::test]
async fn a_room_without_a_message_is_dated_by_its_creation() {
    // D101: "else by its creation".
    let mut create = state_event(
        "m.room.create",
        "",
        "@creator:example.org",
        json!({"creator": "@creator:example.org", "room_version": "10"}),
    );
    create["origin_server_ts"] = json!(4_242);
    let rooms = vec![
        RoomSeed::new("!new:example.org")
            .named("New")
            .with_state(create),
    ];
    let list = first_list(&serve(rooms, AccountData::default()).await).await;
    let room = find(&list, "!new:example.org");
    assert_eq!(room.last_activity, Timestamp::from_millis(4_242));
    assert_eq!(room.last_message_preview, None);
    assert_eq!(room.last_sender, None);
}

#[tokio::test]
async fn unread_counts_and_mentions_come_from_the_server_counts() {
    let (rooms, data) = account();
    let list = first_list(&serve(rooms, data).await).await;
    let unread = |id: &str| find(&list, id).unread;
    assert_eq!(
        unread("!group:example.org"),
        UnreadCount {
            unread: 3,
            mentions: 1,
            marked_unread: false
        }
    );
    assert_eq!(
        unread("!dm:example.org"),
        UnreadCount {
            unread: 0,
            mentions: 0,
            marked_unread: false
        }
    );
}

#[tokio::test]
async fn a_muted_room_is_marked_muted_and_keeps_its_count() {
    let (rooms, data) = account();
    let list = first_list(&serve(rooms, data).await).await;
    let quiet = find(&list, "!quiet:example.org");
    assert!(quiet.is_muted);
    assert_eq!(quiet.unread.unread, 4);
    assert!(!find(&list, "!group:example.org").is_muted);
}

#[tokio::test]
async fn the_favourite_and_low_priority_tags_are_read() {
    let (rooms, data) = account();
    let list = first_list(&serve(rooms, data).await).await;
    assert!(find(&list, "!group:example.org").is_favourite);
    assert!(find(&list, "!dm:example.org").is_low_priority);
    let quiet = find(&list, "!quiet:example.org");
    assert!(!quiet.is_favourite && !quiet.is_low_priority);
}

#[tokio::test]
async fn a_space_is_a_space_and_its_rooms_know_it() {
    let rooms = vec![
        RoomSeed::new("!space:example.org").named("Company").space(),
        RoomSeed::new("!chan:example.org")
            .named("General")
            .in_space("!space:example.org"),
    ];
    let list = first_list(&serve(rooms, AccountData::default()).await).await;
    assert_eq!(find(&list, "!space:example.org").kind, RoomKind::Space);
    assert_eq!(
        find(&list, "!chan:example.org").space,
        Some(RoomId::parse("!space:example.org").unwrap())
    );
}

// ------------------------------------------------------------ live

#[tokio::test]
async fn a_new_message_updates_the_row_of_its_room() {
    let (rooms, data) = account();
    let served = serve(rooms, data).await;
    let mut stream = served.subscribe().await.unwrap();
    let mut list = match next_batch(&mut stream).await.as_slice() {
        [RoomListDiff::Reset(rooms)] => rooms.clone(),
        other => panic!("first batch: {other:?}"),
    };

    let bob = "@bob:example.org";
    served.script.push(vec![
        RoomSeed::new("!group:example.org")
            .named("Team")
            .unread(4, 1)
            .bumped(99)
            .with_member(bob, "Bob Martin")
            .with_message(text_event("$g2", bob, 9_000, "running late")),
    ]);

    let deadline = tokio::time::Instant::now() + WAIT;
    loop {
        for diff in next_batch(&mut stream).await {
            list = apply_diff(&list, diff);
        }
        let team = find(&list, "!group:example.org");
        if team.last_message_preview.as_deref() == Some("running late") {
            assert_eq!(team.last_activity, Timestamp::from_millis(9_000));
            assert_eq!(team.unread.unread, 4);
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "the update never came: {list:?}"
        );
    }
    assert_eq!(list.len(), 3, "no room added or lost");
}

#[tokio::test]
async fn a_room_that_appears_in_the_sync_joins_the_list() {
    let (rooms, data) = account();
    let served = serve(rooms, data).await;
    let mut stream = served.subscribe().await.unwrap();
    let mut list = match next_batch(&mut stream).await.as_slice() {
        [RoomListDiff::Reset(rooms)] => rooms.clone(),
        other => panic!("first batch: {other:?}"),
    };

    served
        .script
        .push(vec![RoomSeed::new("!fresh:example.org").named("Fresh")]);

    let deadline = tokio::time::Instant::now() + WAIT;
    while !list.iter().any(|r| r.id.as_str() == "!fresh:example.org") {
        for diff in next_batch(&mut stream).await {
            list = apply_diff(&list, diff);
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "the room never came: {list:?}"
        );
    }
    assert_eq!(find(&list, "!fresh:example.org").name, "Fresh");
}

// ------------------------------------------------------------ errors

#[tokio::test]
async fn a_client_with_no_session_is_not_signed_in() {
    let server = MatrixMockServer::new().await;
    let dir = tempfile::tempdir().unwrap();
    let port = MatrixRoomList::new(common::client(&server, dir.path()).await);
    let refused = tokio::time::timeout(WAIT, port.subscribe())
        .await
        .expect("subscribe answers")
        .err();
    assert_eq!(refused, Some(RoomListError::NotSignedIn));
}

async fn failing_server(template: ResponseTemplate) -> Served {
    let server = MatrixMockServer::new().await;
    server
        .mock_versions()
        .with_simplified_sliding_sync()
        .ok()
        .mount()
        .await;
    server
        .mock_sliding_sync()
        .respond_with(template)
        .mount()
        .await;
    let dir = tempfile::tempdir().unwrap();
    let client = signed_in(&server, dir.path(), &alice()).await;
    Served {
        port: MatrixRoomList::new(client),
        script: SlidingScript::new(vec![], AccountData::default()),
        _server: server,
        _dir: dir,
    }
}

async fn refusal(served: &Served) -> RoomListError {
    tokio::time::timeout(WAIT, served.subscribe())
        .await
        .expect("a failing server must not hang the subscription")
        .err()
        .expect("subscribe fails")
}

#[tokio::test]
async fn a_server_that_fails_makes_the_list_unavailable_without_leaking_the_token() {
    let served = failing_server(ResponseTemplate::new(500)).await;
    let error = refusal(&served).await;
    assert_eq!(error, RoomListError::Unavailable);
    assert!(!format!("{error} {error:?}").contains(TOKEN));
}

#[tokio::test]
async fn a_token_the_server_no_longer_knows_means_not_signed_in() {
    let expired = ResponseTemplate::new(401).set_body_json(json!({
        "errcode": "M_UNKNOWN_TOKEN",
        "error": "Invalid access token",
        "soft_logout": false,
    }));
    let served = failing_server(expired).await;
    let error = refusal(&served).await;
    assert_eq!(error, RoomListError::NotSignedIn);
    assert!(!format!("{error} {error:?}").contains(TOKEN));
}
