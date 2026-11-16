//! Mentions: who a message notifies (`m.mentions`), the names linked in its HTML, and the lists
//! offered after "@" and "#" (FR-054, D401, D431, D487, D502).

mod find;
mod link;

pub use find::{LinkableRoom, Member, MentionCandidate, match_members, match_rooms};
pub use link::link_names;

use crate::{RoomAlias, UserId};

/// Most people one list offers (FR-054); a message names as many as were picked.
pub const MAX_MENTIONS: usize = 25;

/// What a mention points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MentionTarget {
    /// A person: notified.
    User(UserId),
    /// A room, linked by its alias: notifies nobody.
    Room(RoomAlias),
    /// `@room`: notifies the whole room.
    Everyone,
}

/// A mention picked in a list, and the name written for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mention {
    /// What it points at.
    pub target: MentionTarget,
    /// The name as written in the text.
    pub name: String,
}

impl Mention {
    /// A person.
    pub fn user(id: UserId, name: &str) -> Self {
        Self {
            target: MentionTarget::User(id),
            name: name.into(),
        }
    }

    /// A room.
    pub fn room(alias: RoomAlias, name: &str) -> Self {
        Self {
            target: MentionTarget::Room(alias),
            name: name.into(),
        }
    }

    /// The whole room, written `@room`.
    pub fn everyone() -> Self {
        Self {
            target: MentionTarget::Everyone,
            name: "@room".into(),
        }
    }
}

/// The `m.mentions` of a message.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mentions {
    /// The people notified.
    pub user_ids: Vec<UserId>,
    /// The whole room is notified.
    pub room: bool,
}

/// D487/D401: the picked mentions the text still names, each person once in the order picked. A room link notifies nobody.
pub fn extract(text: &str, picked: &[Mention]) -> Mentions {
    let mut found = Mentions::default();
    for mention in picked.iter().filter(|m| text.contains(&m.name)) {
        match &mention.target {
            MentionTarget::Everyone => found.room = true,
            MentionTarget::User(id) => {
                if !found.user_ids.contains(id) {
                    found.user_ids.push(id.clone());
                }
            }
            MentionTarget::Room(_) => {}
        }
    }
    found
}
