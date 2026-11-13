//! A homeserver that answers sliding sync (MSC4186), `/messages` and `/send` from a script, so
//! that the read adapters see rooms and histories without any network.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    dead_code
)]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use matrix_sdk::Client;
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use serde_json::{Value, json};
use twake_domain::UserId;
use twake_domain::session::{DeviceId, Secret, Session};
use wiremock::{Request, ResponseTemplate};

pub const TOKEN: &str = "syt_seed_access_token";

/// One room of a sliding sync response.
#[derive(Clone)]
pub struct RoomSeed {
    id: String,
    name: Option<String>,
    is_dm: bool,
    notifications: u64,
    highlights: u64,
    state: Vec<Value>,
    timeline: Vec<Value>,
    prev_batch: Option<String>,
    avatar: Option<String>,
    bump: u64,
}

impl RoomSeed {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: None,
            is_dm: false,
            notifications: 0,
            highlights: 0,
            state: vec![],
            timeline: vec![],
            prev_batch: None,
            avatar: None,
            bump: 1,
        }
    }

    pub fn named(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn direct(mut self) -> Self {
        self.is_dm = true;
        self
    }

    /// `m.room.create` of type `m.space`.
    pub fn space(self) -> Self {
        let create = state_event(
            "m.room.create",
            "",
            "@creator:example.org",
            json!({"creator": "@creator:example.org", "type": "m.space", "room_version": "10"}),
        );
        self.with_state(create)
    }

    pub fn unread(mut self, notifications: u64, highlights: u64) -> Self {
        self.notifications = notifications;
        self.highlights = highlights;
        self
    }

    pub fn avatar(mut self, mxc: &str) -> Self {
        self.avatar = Some(mxc.to_string());
        self
    }

    pub fn bumped(mut self, stamp: u64) -> Self {
        self.bump = stamp;
        self
    }

    pub fn in_space(self, space: &str) -> Self {
        let parent = state_event(
            "m.space.parent",
            space,
            "@creator:example.org",
            json!({"via": ["example.org"], "canonical": true}),
        );
        self.with_state(parent)
    }

    pub fn with_state(mut self, event: Value) -> Self {
        self.state.push(event);
        self
    }

    /// The profile of a member, so that its display name is known to the client.
    pub fn with_member(self, user: &str, display_name: &str) -> Self {
        let member = state_event(
            "m.room.member",
            user,
            user,
            json!({"membership": "join", "displayname": display_name}),
        );
        self.with_state(member)
    }

    /// Events of the timeline, oldest first, and the token to page back from the first.
    pub fn with_timeline(mut self, events: Vec<Value>, prev_batch: Option<String>) -> Self {
        self.timeline = events;
        self.prev_batch = prev_batch;
        self
    }

    pub fn with_message(mut self, event: Value) -> Self {
        self.timeline.push(event);
        self
    }

    fn json(&self, initial: bool) -> Value {
        let mut state = self.state.clone();
        if let Some(name) = &self.name {
            state.push(state_event(
                "m.room.name",
                "",
                "@creator:example.org",
                json!({ "name": name }),
            ));
        }
        if let Some(mxc) = &self.avatar {
            state.push(state_event(
                "m.room.avatar",
                "",
                "@creator:example.org",
                json!({ "url": mxc }),
            ));
        }
        let mut room = json!({
            "initial": initial,
            "bump_stamp": self.bump,
            "is_dm": self.is_dm,
            "notification_count": self.notifications,
            "highlight_count": self.highlights,
            "joined_count": 2,
            "invited_count": 0,
            "required_state": state,
            "timeline": self.timeline,
        });
        if let Some(token) = &self.prev_batch {
            room["prev_batch"] = json!(token);
        }
        room
    }
}

pub fn state_event(kind: &str, key: &str, sender: &str, content: Value) -> Value {
    json!({
        "type": kind,
        "state_key": key,
        "sender": sender,
        "content": content,
        "event_id": format!("$state-{kind}-{key}"),
        "origin_server_ts": 1,
    })
}

/// An `m.room.message` of text.
pub fn text_event(event_id: &str, sender: &str, ts: u64, body: &str) -> Value {
    json!({
        "type": "m.room.message",
        "event_id": event_id,
        "sender": sender,
        "origin_server_ts": ts,
        "content": {"msgtype": "m.text", "body": body},
    })
}

/// A reply, with the quote Matrix clients still put in the body (the fallback).
pub fn reply_event(event_id: &str, sender: &str, ts: u64, body: &str, to: &str) -> Value {
    let mut event = text_event(event_id, sender, ts, body);
    event["content"]["m.relates_to"] = json!({"m.in_reply_to": {"event_id": to}});
    event
}

/// An edit (`m.replace`) of `target`.
pub fn edit_event(event_id: &str, sender: &str, ts: u64, target: &str, new_body: &str) -> Value {
    json!({
        "type": "m.room.message",
        "event_id": event_id,
        "sender": sender,
        "origin_server_ts": ts,
        "content": {
            "msgtype": "m.text",
            "body": format!("* {new_body}"),
            "m.new_content": {"msgtype": "m.text", "body": new_body},
            "m.relates_to": {"rel_type": "m.replace", "event_id": target},
        },
    })
}

/// A redaction of `target`.
pub fn redaction_event(event_id: &str, sender: &str, ts: u64, target: &str) -> Value {
    json!({
        "type": "m.room.redaction",
        "event_id": event_id,
        "sender": sender,
        "origin_server_ts": ts,
        "redacts": target,
        "content": {"redacts": target},
    })
}

/// What the account data extension of the sync holds.
#[derive(Clone, Default)]
pub struct AccountData {
    global: Vec<Value>,
    directs: Vec<(String, String)>,
    rooms: Vec<(String, Vec<Value>)>,
}

impl AccountData {
    /// The room is muted: an `override` rule with no action (what clients write for "Mute").
    pub fn muting(mut self, room: &str) -> Self {
        let rules = json!({
            "type": "m.push_rules",
            "content": {"global": {"override": [{
                "rule_id": room,
                "default": false,
                "enabled": true,
                "conditions": [{"kind": "event_match", "key": "room_id", "pattern": room}],
                "actions": [],
            }]}},
        });
        self.global.push(rules);
        self
    }

    /// `m.direct`: `room` is the direct room with `user` (what makes a room direct, D291).
    pub fn direct(mut self, room: &str, user: &str) -> Self {
        self.directs.push((user.to_string(), room.to_string()));
        self
    }

    /// `m.tag`: `m.favourite` or `m.lowpriority`.
    pub fn tagging(mut self, room: &str, tag: &str) -> Self {
        let event = json!({"type": "m.tag", "content": {"tags": {tag: {"order": 0.5}}}});
        self.rooms.push((room.to_string(), vec![event]));
        self
    }

    fn json(&self) -> Value {
        let mut rooms = serde_json::Map::new();
        for (room, events) in &self.rooms {
            rooms.insert(room.clone(), json!(events));
        }
        let mut global = self.global.clone();
        if !self.directs.is_empty() {
            let mut map = serde_json::Map::new();
            for (user, room) in &self.directs {
                let rooms = map.entry(user.clone()).or_insert_with(|| json!([]));
                rooms.as_array_mut().unwrap().push(json!(room));
            }
            global.push(json!({"type": "m.direct", "content": map}));
        }
        json!({"global": global, "rooms": rooms})
    }
}

#[derive(Default)]
struct Script {
    first: Vec<RoomSeed>,
    account_data: AccountData,
    next: VecDeque<Vec<RoomSeed>>,
}

/// The sliding sync endpoint: the first response holds the seeded rooms, `push` queues what
/// the next poll brings, a poll with nothing queued answers empty after a short wait.
#[derive(Clone)]
pub struct SlidingScript(Arc<Mutex<Script>>);

impl SlidingScript {
    pub fn new(rooms: Vec<RoomSeed>, account_data: AccountData) -> Self {
        Self(Arc::new(Mutex::new(Script {
            first: rooms,
            account_data,
            next: VecDeque::new(),
        })))
    }

    /// The next poll brings these rooms (an update of a known room carries `initial: false`).
    pub fn push(&self, rooms: Vec<RoomSeed>) {
        self.0.lock().unwrap().next.push_back(rooms);
    }

    fn respond(&self, request: &Request) -> ResponseTemplate {
        let body: Value = request.body_json().unwrap_or_default();
        let has_pos = request.url.query_pairs().any(|(k, _)| k == "pos");
        let mut script = self.0.lock().unwrap();
        let (rooms, initial) = if !has_pos {
            (script.first.clone(), true)
        } else {
            (script.next.pop_front().unwrap_or_default(), false)
        };
        let total = script.first.len() + rooms.len();
        let mut room_map = serde_json::Map::new();
        for room in &rooms {
            room_map.insert(room.id.clone(), room.json(initial));
        }
        let ext = if has_pos {
            json!({})
        } else {
            json!({"account_data": script.account_data.json()})
        };
        let template = ResponseTemplate::new(200).set_body_json(json!({
            "txn_id": body["txn_id"],
            "pos": if has_pos { "2" } else { "1" },
            "lists": {"all_rooms": {"count": total}},
            "rooms": room_map,
            "extensions": ext,
        }));
        if has_pos && rooms.is_empty() {
            template.set_delay(Duration::from_millis(250))
        } else {
            template
        }
    }

    /// Mounts the endpoint (and `/versions` announcing sliding sync) on `server`.
    pub async fn mount(&self, server: &MatrixMockServer) {
        server
            .mock_versions()
            .with_simplified_sliding_sync()
            .ok()
            .mount()
            .await;
        let script = self.clone();
        server
            .mock_sliding_sync()
            .respond_with(move |request: &Request| script.respond(request))
            .mount()
            .await;
    }
}

/// A client of `server` signed in as `user` on `DEVICE`, through the adapter of lot 1.
pub async fn signed_in(server: &MatrixMockServer, dir: &std::path::Path, user: &UserId) -> Client {
    let client = super::client(server, dir).await;
    let session = Session {
        user_id: user.clone(),
        device_id: DeviceId::parse("SEEDDEVICE").unwrap(),
        access_token: Secret::new(TOKEN),
        homeserver: super::homeserver(server),
    };
    twake_matrix::restore_session(&client, &session)
        .await
        .unwrap();
    client
}
