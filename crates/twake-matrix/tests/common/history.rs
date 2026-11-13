//! A room whose whole history is known: the sync carries its last events, `/messages` pages
//! back through the rest by token, exactly as a homeserver does.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    dead_code
)]

use matrix_sdk::test_utils::mocks::MatrixMockServer;
use serde_json::{Value, json};
use std::time::Duration;
use wiremock::{Request, ResponseTemplate};

use super::seed::RoomSeed;

/// Events the sync itself carries, the newest ones; every older one is behind `/messages`.
pub const IN_SYNC: usize = 5;

/// The whole history of one room, oldest first (events as JSON).
#[derive(Clone)]
pub struct History {
    pub room: String,
    pub events: Vec<Value>,
}

impl History {
    pub fn new(room: &str, events: Vec<Value>) -> Self {
        Self {
            room: room.to_string(),
            events,
        }
    }

    /// The room as the sync shows it: the last `IN_SYNC` events and the token behind them.
    pub fn seed(&self) -> RoomSeed {
        let start = self.events.len().saturating_sub(IN_SYNC);
        let token = (start > 0).then(|| format!("t{start}"));
        RoomSeed::new(&self.room)
            .named("History room")
            .with_timeline(self.events[start..].to_vec(), token)
    }

    fn page(&self, request: &Request) -> ResponseTemplate {
        let query = |key: &str| {
            request
                .url
                .query_pairs()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.into_owned())
        };
        let from: usize = query("from")
            .and_then(|t| t.trim_start_matches('t').parse().ok())
            .unwrap_or(self.events.len());
        let limit: usize = query("limit").and_then(|l| l.parse().ok()).unwrap_or(10);
        let start = from.saturating_sub(limit);
        let chunk: Vec<&Value> = self.events[start..from].iter().rev().collect();
        let mut body = json!({"start": format!("t{from}"), "chunk": chunk, "state": []});
        if start > 0 {
            body["end"] = json!(format!("t{start}"));
        }
        ResponseTemplate::new(200).set_body_json(body)
    }

    /// Mounts `/messages` of the room.
    pub async fn mount(&self, server: &MatrixMockServer) {
        let history = self.clone();
        server
            .mock_room_messages()
            .respond_with(move |request: &Request| history.page(request))
            .mount()
            .await;
    }
}

/// `/send` answers `$sent` after `delay`: a long delay keeps the local echo the only news.
pub async fn mount_send(server: &MatrixMockServer, delay: Duration) {
    server
        .mock_room_send()
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"event_id": "$sent"}))
                .set_delay(delay),
        )
        .mount()
        .await;
}
