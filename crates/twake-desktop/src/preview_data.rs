//! `--preview` fixtures: 50 rooms and a 10 000-message history, made from a fixed seed.
//! Built from domain types alone: `twake-testing` is a dev-dependency only (ARCHITECTURE.md section 3, A011 Q4).

// Fixtures of a preview window, built from literals: `expect` cannot fail.
#![allow(clippy::expect_used)]

use std::collections::HashMap;

use twake_domain::read_state::UnreadCount;
use twake_domain::room_list::{LastSender, RoomKind, RoomSummary};
use twake_domain::timeline::Message;
use twake_domain::{EventId, RoomId, Timestamp, UserId};

/// Messages of the big room.
pub const BIG_ROOM_MESSAGES: usize = 10_000;
const ROOMS: usize = 50;
/// Who wrote the newest messages, newest first: `true` is the user.
const OWN_TAIL: [bool; 9] = [true, true, false, true, false, false, true, true, false];
const MINUTE_MS: u64 = 60_000;
const DAY_MS: u64 = 24 * 60 * MINUTE_MS;

const PEOPLE: [&str; 12] = [
    "Alice Martin",
    "Bob Durand",
    "Carol Nguyen",
    "Dmitri Ivanov",
    "Élodie Petit",
    "Farid Haddad",
    "Grace Okafor",
    "Hugo Lambert",
    "Inès Moreau",
    "Jun Tanaka",
    "Karim Benali",
    "Léa Fontaine",
];
const TOPICS: [&str; 10] = [
    "Design crew",
    "Release train",
    "Lunch",
    "Backend",
    "Product",
    "Support",
    "Marketing",
    "Infra",
    "Random",
    "Mobile",
];
const SENTENCES: [&str; 8] = [
    "ok",
    "On my way, see you in ten minutes.",
    "Can someone review the pull request before noon? It touches the timeline and the room list.",
    "Thanks!",
    "Lunch at noon?",
    "I pushed a fix for the flaky test, please rebase when you can and tell me if it still fails on your machine.",
    "Great idea, let's try it on Monday and see how the numbers move.",
    "No worries.",
];

/// A small xorshift generator: the same data at every start.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Everything the preview shows.
pub struct PreviewData {
    /// The user.
    pub me: UserId,
    /// Display names of the people.
    pub names: HashMap<UserId, String>,
    /// The room list.
    pub rooms: Vec<RoomSummary>,
    /// The history of each room, oldest first.
    pub histories: HashMap<RoomId, Vec<Message>>,
    /// The room with `BIG_ROOM_MESSAGES` messages.
    pub big_room: RoomId,
}

fn user(local: &str) -> UserId {
    UserId::parse(&format!("@{local}:example.org")).expect("a literal user id parses")
}

fn people() -> Vec<(UserId, String)> {
    PEOPLE
        .iter()
        .enumerate()
        .map(|(n, name)| (user(&format!("person{n}")), (*name).to_owned()))
        .collect()
}

fn room_id(n: usize) -> RoomId {
    RoomId::parse(&format!("!room{n}:example.org")).expect("a literal room id parses")
}

fn event_id(room: usize, n: usize) -> EventId {
    EventId::parse(&format!("$e{room}x{n}")).expect("a literal event id parses")
}

fn text(rng: &mut Rng) -> String {
    let base = SENTENCES[rng.below(SENTENCES.len())].to_owned();
    if rng.below(12) == 0 {
        // A long message, a few sentences.
        (0..4).fold(base, |all, _| {
            all + " " + SENTENCES[rng.below(SENTENCES.len())]
        })
    } else {
        base
    }
}

fn history(
    room: usize,
    count: usize,
    now: Timestamp,
    me: &UserId,
    people: &[(UserId, String)],
    rng: &mut Rng,
) -> Vec<Message> {
    // Oldest first, spread over `count` slots of about 6 minutes, ending a few minutes ago.
    let mut ts = now
        .as_millis()
        .saturating_sub((count as u64 + 2) * 6 * MINUTE_MS);
    let mut sender = me.clone();
    let mut out: Vec<Message> = Vec::with_capacity(count);
    for n in 0..count {
        // Bursts: most messages follow within two minutes, some after a long silence.
        ts += match rng.below(10) {
            0 => 20 * MINUTE_MS + rng.next() % (3 * 60 * MINUTE_MS),
            _ => MINUTE_MS / 4 + rng.next() % (2 * MINUTE_MS),
        };
        // The newest messages mix both sides, so the first view shows own bubbles.
        let from_end = count - 1 - n;
        if from_end < OWN_TAIL.len() {
            sender = if OWN_TAIL[from_end] {
                me.clone()
            } else {
                people[from_end % people.len()].0.clone()
            };
        } else if rng.below(3) == 0 {
            sender = if rng.below(3) == 0 {
                me.clone()
            } else {
                people[rng.below(people.len())].0.clone()
            };
        }
        let mut message = Message::remote(
            event_id(room, n),
            sender.clone(),
            Timestamp::from_millis(ts.min(now.as_millis())),
            text(rng),
        );
        if n > 5 && rng.below(40) == 0 {
            message = message.with_reply_to(event_id(room, n - 1 - rng.below(5)));
        }
        if rng.below(60) == 0 {
            message = message.edited();
        }
        out.push(message);
    }
    out
}

fn summary(n: usize, now: Timestamp, rng: &mut Rng, people: &[(UserId, String)]) -> RoomSummary {
    let kind = match n % 5 {
        0 | 3 => RoomKind::Direct,
        1 | 2 => RoomKind::Group,
        _ => RoomKind::Broadcast,
    };
    let name = match kind {
        RoomKind::Direct => people[n % people.len()].1.clone(),
        _ => format!("{} {}", TOPICS[n % TOPICS.len()], n),
    };
    let age = rng.next() % (10 * DAY_MS);
    let unread = if rng.below(3) == 0 {
        UnreadCount {
            unread: rng.below(130) as u32,
            mentions: u32::from(rng.below(5) == 0),
            marked_unread: false,
        }
    } else {
        UnreadCount {
            unread: 0,
            mentions: 0,
            marked_unread: false,
        }
    };
    let sender = if rng.below(4) == 0 {
        LastSender::Me
    } else {
        LastSender::Other(people[rng.below(people.len())].1.clone())
    };
    let mut room = RoomSummary::new(room_id(n), name, kind)
        .with_last_message_preview(text(rng))
        .with_last_activity(Timestamp::from_millis(now.as_millis() - age))
        .with_unread(unread)
        .with_last_sender(sender);
    if n.is_multiple_of(11) {
        room = room.favourite();
    }
    if n.is_multiple_of(7) {
        room = room.muted();
    }
    room
}

/// The fixtures, as of `now`.
pub fn preview_data(now: Timestamp) -> PreviewData {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let me = user("me");
    let people = people();
    let mut rooms: Vec<RoomSummary> = (0..ROOMS)
        .map(|n| summary(n, now, &mut rng, &people))
        .collect();
    // The big room is the newest of the list.
    if let Some(first) = rooms.first_mut() {
        first.name = "Twake Chat 10k".to_owned();
        first.kind = RoomKind::Group;
        first.last_activity = now;
    }
    let mut histories = HashMap::new();
    for n in 0..ROOMS {
        let count = if n == 0 {
            BIG_ROOM_MESSAGES
        } else {
            12 + rng.below(40)
        };
        histories.insert(room_id(n), history(n, count, now, &me, &people, &mut rng));
    }
    PreviewData {
        names: people.into_iter().collect(),
        big_room: room_id(0),
        me,
        rooms,
        histories,
    }
}
