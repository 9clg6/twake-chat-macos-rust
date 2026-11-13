//! `TimelinePort` on matrix-sdk-ui `Timeline`, fed by a mock homeserver (sliding sync room,
//! `/messages` pages by token, `/send`): text, replies, edits, redactions, days, history
//! (FR of the conversation, D31, D495). No network, no real account.
//!
//! Expected public surface of `twake_matrix`: `MatrixTimeline::new(Client) -> MatrixTimeline`
//! implementing `twake_app::ports::TimelinePort`; the adapter starts the sync it needs at the
//! first `open`. What the screens get:
//! - a redacted message is not a message: it is left out of `Reset` / `Prepend` and a live
//!   redaction is `Remove(MessageKey::Event(id))` (the domain has no "deleted" flag: open question);
//! - an edit is no message of its own: the edited one carries the new body and `edited`
//!   (live: `Update`); a reply carries `reply_to` and its body has no quote fallback;
//! - the day dividers of the SDK are not messages: the domain draws its own (`Day::of`);
//! - a failed send is an `Update` of the echo (same transaction id) with `SendState::Failed`;
//!   a confirmed one is an `Update` or an `Append` with the same transaction id and the event id.
#![recursion_limit = "256"]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use std::sync::Arc;
use std::time::Duration;

use common::history::{History, mount_send};
use common::seed::{
    AccountData, RoomSeed, SlidingScript, TOKEN, edit_event, redaction_event, reply_event,
    signed_in, text_event,
};
use futures::StreamExt;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use serde_json::{Value, json};
use tempfile::TempDir;
use twake_app::ports::{TimelineError, TimelinePort};
use twake_domain::timeline::{Message, MessageKey, SendState, TimelineDiff};
use twake_domain::{EventId, RoomId, UserId};
use twake_matrix::MatrixTimeline;
use twake_testing::contract;
use wiremock::ResponseTemplate;

const WAIT: Duration = Duration::from_secs(15);
const ROOM: &str = "!room:example.org";
const BOB: &str = "@bob:example.org";

fn alice() -> UserId {
    UserId::parse("@alice:example.org").unwrap()
}

fn room() -> RoomId {
    RoomId::parse(ROOM).unwrap()
}

fn event(id: &str) -> EventId {
    EventId::parse(id).unwrap()
}

struct Served {
    port: MatrixTimeline,
    script: SlidingScript,
    server: MatrixMockServer,
    _dir: TempDir,
}

impl TimelinePort for Served {
    fn open(
        &self,
        room: RoomId,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Vec<TimelineDiff>>, TimelineError>> {
        self.port.open(room)
    }

    fn paginate_back(&self, room: RoomId) -> BoxFuture<'_, Result<(), TimelineError>> {
        self.port.paginate_back(room)
    }

    fn send_text(&self, room: RoomId, body: String) -> BoxFuture<'_, Result<(), TimelineError>> {
        self.port.send_text(room, body)
    }
}

/// `me` is in the room `ROOM` whose history is `events`; `/send` answers after `send_delay`.
async fn serve(me: &UserId, events: Vec<Value>, send_delay: Duration) -> Served {
    let server = MatrixMockServer::new().await;
    let history = History::new(ROOM, events);
    let script = SlidingScript::new(vec![history.seed()], AccountData::default());
    script.mount(&server).await;
    history.mount(&server).await;
    mount_send(&server, send_delay).await;
    let dir = tempfile::tempdir().unwrap();
    let client = signed_in(&server, dir.path(), me).await;
    Served {
        port: MatrixTimeline::new(client),
        script,
        server,
        _dir: dir,
    }
}

fn texts(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| {
            text_event(
                &format!("$m{i}"),
                BOB,
                1_000 + i as u64,
                &format!("message {i}"),
            )
        })
        .collect()
}

async fn next_batch(stream: &mut BoxStream<'static, Vec<TimelineDiff>>) -> Vec<TimelineDiff> {
    tokio::time::timeout(WAIT, stream.next())
        .await
        .expect("a batch arrives")
        .expect("the stream is alive")
}

async fn open(served: &Served) -> (BoxStream<'static, Vec<TimelineDiff>>, Vec<Message>) {
    let mut stream = tokio::time::timeout(WAIT, served.open(room()))
        .await
        .expect("open answers")
        .expect("open succeeds");
    match next_batch(&mut stream).await.as_slice() {
        [TimelineDiff::Reset(messages)] => (stream, messages.clone()),
        other => panic!("the first batch must be one Reset, got {other:?}"),
    }
}

fn ids(messages: &[Message]) -> Vec<String> {
    messages
        .iter()
        .map(|m| {
            m.event_id()
                .map(|e| e.as_str().to_string())
                .unwrap_or_default()
        })
        .collect()
}

fn by_id<'a>(messages: &'a [Message], id: &str) -> &'a Message {
    messages
        .iter()
        .find(|m| m.event_id().map(EventId::as_str) == Some(id))
        .unwrap_or_else(|| panic!("{id} is not in {messages:?}"))
}

// ------------------------------------------------------------ the contract

#[tokio::test(flavor = "multi_thread")]
async fn the_timeline_contract_holds_on_the_adapter() {
    let handle = tokio::runtime::Handle::current();
    let verdict = tokio::task::spawn_blocking(move || {
        contract::timeline(|me, room, history| {
            // The echo must be the only news of a send: the server answers much later.
            let events = history.iter().map(event_of).collect();
            let served = handle.block_on(serve_room(&me, room.as_str(), events));
            Arc::new(served) as Arc<dyn TimelinePort>
        })
    })
    .await
    .unwrap();
    verdict.expect("the timeline contract");
}

fn event_of(message: &Message) -> Value {
    text_event(
        message.event_id().unwrap().as_str(),
        message.sender.as_str(),
        message.timestamp.as_millis(),
        &message.body,
    )
}

async fn serve_room(me: &UserId, room: &str, events: Vec<Value>) -> Served {
    let server = MatrixMockServer::new().await;
    let history = History::new(room, events);
    let script = SlidingScript::new(vec![history.seed()], AccountData::default());
    script.mount(&server).await;
    history.mount(&server).await;
    mount_send(&server, Duration::from_secs(120)).await;
    let dir = tempfile::tempdir().unwrap();
    let client = signed_in(&server, dir.path(), me).await;
    Served {
        port: MatrixTimeline::new(client),
        script,
        server,
        _dir: dir,
    }
}

// ------------------------------------------------------------ what a bubble shows

#[tokio::test]
async fn text_messages_arrive_in_order_with_sender_time_and_body() {
    let served = serve(&alice(), texts(3), Duration::from_secs(120)).await;
    let (_stream, messages) = open(&served).await;
    assert_eq!(ids(&messages), ["$m0", "$m1", "$m2"]);
    let second = by_id(&messages, "$m1");
    assert_eq!(second.body, "message 1");
    assert_eq!(second.sender.as_str(), BOB);
    assert_eq!(second.timestamp.as_millis(), 1_001);
    assert_eq!(second.send_state, SendState::Sent);
    assert!(!second.edited && second.reply_to.is_none());
}

#[tokio::test]
async fn a_reply_names_the_message_it_answers_and_drops_the_quote_fallback() {
    let mut events = texts(1);
    events.push(reply_event(
        "$r",
        BOB,
        2_000,
        "> <@bob:example.org> message 0\n\nthe answer",
        "$m0",
    ));
    let served = serve(&alice(), events, Duration::from_secs(120)).await;
    let (_stream, messages) = open(&served).await;
    let reply = by_id(&messages, "$r");
    assert_eq!(reply.reply_to, Some(event("$m0")));
    assert_eq!(reply.body, "the answer");
}

#[tokio::test]
async fn an_edit_changes_the_body_of_its_message_and_is_no_message_itself() {
    let mut events = texts(2);
    events.push(edit_event("$e", BOB, 3_000, "$m0", "message zero"));
    let served = serve(&alice(), events, Duration::from_secs(120)).await;
    let (_stream, messages) = open(&served).await;
    assert_eq!(ids(&messages), ["$m0", "$m1"]);
    let first = by_id(&messages, "$m0");
    assert_eq!(first.body, "message zero");
    assert!(first.edited);
    assert!(!by_id(&messages, "$m1").edited);
}

#[tokio::test]
async fn a_deleted_message_is_left_out() {
    let mut events = texts(3);
    events.push(redaction_event("$x", BOB, 3_000, "$m1"));
    let served = serve(&alice(), events, Duration::from_secs(120)).await;
    let (_stream, messages) = open(&served).await;
    assert_eq!(ids(&messages), ["$m0", "$m2"]);
}

#[tokio::test]
async fn messages_of_several_days_come_as_messages_only() {
    let day = 86_400_000u64;
    let events: Vec<Value> = [0, 0, 1, 1, 3]
        .iter()
        .enumerate()
        .map(|(i, d)| {
            text_event(
                &format!("$d{i}"),
                BOB,
                1_700_000_000_000 + d * day + i as u64,
                "x",
            )
        })
        .collect();
    let served = serve(&alice(), events, Duration::from_secs(120)).await;
    let (_stream, messages) = open(&served).await;
    assert_eq!(ids(&messages), ["$d0", "$d1", "$d2", "$d3", "$d4"]);
}

#[tokio::test]
async fn paging_back_reaches_messages_deleted_or_edited_in_older_pages() {
    // 40 events: the sync holds the last 5, `/messages` the rest. $m2 is edited, $m3 deleted.
    let mut events = texts(36);
    events.push(edit_event("$e", BOB, 5_000, "$m2", "two"));
    events.push(redaction_event("$x", BOB, 5_001, "$m3"));
    events.extend((36..38).map(|i| {
        text_event(
            &format!("$m{i}"),
            BOB,
            6_000 + i as u64,
            &format!("message {i}"),
        )
    }));
    let served = serve(&alice(), events, Duration::from_secs(120)).await;
    let (mut stream, first) = open(&served).await;
    assert_eq!(first.len(), 30, "the last page");

    served.paginate_back(room()).await.unwrap();
    let TimelineDiff::Prepend {
        messages,
        reached_start,
    } = next_batch(&mut stream).await.remove(0)
    else {
        panic!("a page back is one Prepend");
    };
    assert!(reached_start);
    assert_eq!(by_id(&messages, "$m2").body, "two");
    assert!(by_id(&messages, "$m2").edited);
    assert!(!ids(&messages).contains(&"$m3".to_string()), "{messages:?}");
}

// ------------------------------------------------------------ live

#[tokio::test]
async fn a_new_message_of_someone_else_is_appended() {
    let served = serve(&alice(), texts(2), Duration::from_secs(120)).await;
    let (mut stream, _) = open(&served).await;
    served
        .script
        .push(vec![RoomSeed::new(ROOM).with_message(text_event(
            "$new",
            BOB,
            9_000,
            "just arrived",
        ))]);
    let TimelineDiff::Append(messages) = next_batch(&mut stream).await.remove(0) else {
        panic!("a new message is an Append");
    };
    assert_eq!(ids(&messages), ["$new"]);
    assert_eq!(messages[0].body, "just arrived");
}

#[tokio::test]
async fn a_live_edit_updates_the_message() {
    let served = serve(&alice(), texts(2), Duration::from_secs(120)).await;
    let (mut stream, _) = open(&served).await;
    served.script.push(vec![
        RoomSeed::new(ROOM).with_message(edit_event("$e", BOB, 9_000, "$m1", "better")),
    ]);
    let TimelineDiff::Update(message) = next_batch(&mut stream).await.remove(0) else {
        panic!("an edit is an Update");
    };
    assert_eq!(message.event_id(), Some(&event("$m1")));
    assert_eq!(message.body, "better");
    assert!(message.edited);
}

#[tokio::test]
async fn a_live_deletion_removes_the_message() {
    let served = serve(&alice(), texts(3), Duration::from_secs(120)).await;
    let (mut stream, _) = open(&served).await;
    served.script.push(vec![
        RoomSeed::new(ROOM).with_message(redaction_event("$x", BOB, 9_000, "$m1")),
    ]);
    let batch = next_batch(&mut stream).await;
    assert_eq!(
        batch,
        vec![TimelineDiff::Remove(MessageKey::Event(event("$m1")))]
    );
}

// ------------------------------------------------------------ sending

async fn the_echo(stream: &mut BoxStream<'static, Vec<TimelineDiff>>) -> Message {
    match next_batch(stream).await.as_slice() {
        [TimelineDiff::Append(list)] if list.len() == 1 => list[0].clone(),
        other => panic!("a send must append one local echo, got {other:?}"),
    }
}

#[tokio::test]
async fn a_confirmed_send_is_the_same_bubble_with_the_event_id_of_the_server() {
    let served = serve(&alice(), texts(2), Duration::from_millis(300)).await;
    let (mut stream, _) = open(&served).await;
    served.send_text(room(), "hello".into()).await.unwrap();
    let echo = the_echo(&mut stream).await;
    assert_eq!(echo.send_state, SendState::Sending);

    let confirmed = loop {
        let batch = next_batch(&mut stream).await;
        let found = batch.into_iter().find_map(|diff| match diff {
            TimelineDiff::Update(m) => Some(m),
            TimelineDiff::Append(list) => list.into_iter().next(),
            _ => None,
        });
        if let Some(message) = found {
            break message;
        }
    };
    assert_eq!(confirmed.event_id(), Some(&event("$sent")));
    assert_eq!(
        confirmed.txn_id(),
        echo.txn_id(),
        "one bubble, one transaction"
    );
    assert_eq!(confirmed.send_state, SendState::Sent);
    assert_eq!(confirmed.body, "hello");
}

#[tokio::test]
async fn a_send_the_server_refuses_ends_as_a_failed_echo() {
    let served = serve(&alice(), texts(2), Duration::from_secs(120)).await;
    served
        .server
        .mock_room_send()
        .respond_with(
            ResponseTemplate::new(403)
                .set_body_json(json!({"errcode": "M_FORBIDDEN", "error": "no"})),
        )
        .mount()
        .await;
    let (mut stream, _) = open(&served).await;
    served.send_text(room(), "nope".into()).await.unwrap();
    let echo = the_echo(&mut stream).await;

    let failed = loop {
        let batch = next_batch(&mut stream).await;
        if let Some(TimelineDiff::Update(m)) = batch.into_iter().next() {
            break m;
        }
    };
    assert_eq!(failed.send_state, SendState::Failed);
    assert_eq!(failed.txn_id(), echo.txn_id());
}

// ------------------------------------------------------------ errors

#[tokio::test]
async fn a_page_the_server_cannot_give_is_unavailable_without_leaking_the_token() {
    let served = serve(&alice(), texts(40), Duration::from_secs(120)).await;
    let (_stream, first) = open(&served).await;
    assert_eq!(first.len(), 30);
    served
        .server
        .mock_room_messages()
        .respond_with(ResponseTemplate::new(500))
        .mount()
        .await;
    let error = tokio::time::timeout(WAIT, served.paginate_back(room()))
        .await
        .expect("a failing server must not hang the page")
        .expect_err("the page fails");
    assert_eq!(error, TimelineError::Unavailable);
    assert!(!format!("{error} {error:?}").contains(TOKEN));
}
