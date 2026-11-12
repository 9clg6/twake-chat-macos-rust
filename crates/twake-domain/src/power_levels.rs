//! `m.room.power_levels`: who may do what in a room (D418, spec defaults when a field is absent).

use std::collections::HashMap;

use crate::UserId;

const MESSAGE: &str = "m.room.message";
const REACTION: &str = "m.reaction";
const PINNED: &str = "m.room.pinned_events";

/// The rule of a redaction alone: one's own events always, the others' from `required`.
pub fn can_redact(own: bool, level: i64, required: i64) -> bool {
    own || level >= required
}

/// The power levels of a room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerLevels {
    users: HashMap<UserId, i64>,
    users_default: i64,
    events: HashMap<String, i64>,
    events_default: i64,
    state_default: i64,
    redact: i64,
    ban: i64,
    creator: Option<UserId>,
    creator_infinite: bool,
}

impl Default for PowerLevels {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerLevels {
    /// The defaults of the spec: users 0, events 0, state 50, redact 50, ban 50.
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            users_default: 0,
            events: HashMap::new(),
            events_default: 0,
            state_default: 50,
            redact: 50,
            ban: 50,
            creator: None,
            creator_infinite: false,
        }
    }

    /// Gives `user` a level.
    pub fn with_user(mut self, user: UserId, level: i64) -> Self {
        self.users.insert(user, level);
        self
    }

    /// The level of whoever has none.
    pub fn with_users_default(mut self, level: i64) -> Self {
        self.users_default = level;
        self
    }

    /// The level needed to send events of `event_type`.
    pub fn with_event(mut self, event_type: &str, level: i64) -> Self {
        self.events.insert(event_type.to_owned(), level);
        self
    }

    /// The level needed to send a message-like event with no entry of its own.
    pub fn with_events_default(mut self, level: i64) -> Self {
        self.events_default = level;
        self
    }

    /// The level needed to send a state event with no entry of its own.
    pub fn with_state_default(mut self, level: i64) -> Self {
        self.state_default = level;
        self
    }

    /// The level needed to redact the events of others.
    pub fn with_redact(mut self, level: i64) -> Self {
        self.redact = level;
        self
    }

    /// The level needed to ban.
    pub fn with_ban(mut self, level: i64) -> Self {
        self.ban = level;
        self
    }

    /// The creator of the room.
    pub fn with_creator(mut self, user: UserId) -> Self {
        self.creator = Some(user);
        self
    }

    /// Whether the room (v12) gives its creator infinite power. Off by default.
    pub fn with_creator_infinite(mut self, infinite: bool) -> Self {
        self.creator_infinite = infinite;
        self
    }

    /// The level of `user`; `i64::MAX` for an infinite creator.
    pub fn level_of(&self, user: &UserId) -> i64 {
        if self.creator_infinite && self.creator.as_ref() == Some(user) {
            return i64::MAX;
        }
        self.users.get(user).copied().unwrap_or(self.users_default)
    }

    fn may(&self, user: &UserId, event_type: &str, default: i64) -> bool {
        let needed = self.events.get(event_type).copied().unwrap_or(default);
        self.level_of(user) >= needed
    }

    /// Whether `user` may send a message-like event of `event_type`.
    pub fn can_send_event(&self, user: &UserId, event_type: &str) -> bool {
        self.may(user, event_type, self.events_default)
    }

    /// Whether `user` may send a state event of `event_type`.
    pub fn can_send_state(&self, user: &UserId, event_type: &str) -> bool {
        self.may(user, event_type, self.state_default)
    }

    /// Whether `user` may write messages.
    pub fn can_send_message(&self, user: &UserId) -> bool {
        self.can_send_event(user, MESSAGE)
    }

    /// Whether `user` may react.
    pub fn can_send_reaction(&self, user: &UserId) -> bool {
        self.can_send_event(user, REACTION)
    }

    /// Whether `user` may pin and unpin.
    pub fn can_pin(&self, user: &UserId) -> bool {
        self.can_send_state(user, PINNED)
    }

    /// Whether `user` may ban.
    pub fn can_ban(&self, user: &UserId) -> bool {
        self.level_of(user) >= self.ban
    }

    /// Whether `me` may redact an event of `sender`.
    pub fn can_redact(&self, me: &UserId, sender: &UserId) -> bool {
        can_redact(me == sender, self.level_of(me), self.redact)
    }
}
