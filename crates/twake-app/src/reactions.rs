//! Reacting with an emoji: the toggle and the recent emojis (D174, D234, D274).

use twake_domain::reaction::{Reactions, same_emoji};
use twake_domain::{EventId, UserId};

/// What a click on an emoji does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReactionIntent {
    /// Send a reaction with this key.
    Send(String),
    /// Take back this reaction event.
    Redact(EventId),
}

/// The fixed row of the menu and of the hover (D234, D274), in this order.
pub const QUICK_REACTIONS: [&str; 6] = ["❤️", "👍", "👎", "😂", "😮", "😢"];

/// What a click on `emoji` does for `me`: take back my reaction if I already gave it, whichever
/// the spelling; else send it, in the spelling of the existing group if there is one (D174).
pub fn apply_reaction_toggle(state: &Reactions, me: &UserId, emoji: &str) -> ReactionIntent {
    let Some(group) = state.group_of(emoji) else {
        return ReactionIntent::Send(emoji.to_owned());
    };
    match group.event_of(me) {
        Some(event) => ReactionIntent::Redact(event.clone()),
        None => ReactionIntent::Send(group.key().to_owned()),
    }
}

/// The emojis the user reacted with lately, for the "recent" tab of the picker.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecentEmojis {
    list: Vec<String>,
}

impl RecentEmojis {
    /// How many emojis are kept.
    pub const KEPT: usize = 8;

    /// Nothing yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Puts `emoji` first, in the spelling just used, once; only `KEPT` remain.
    pub fn remember(&mut self, emoji: &str) {
        self.list.retain(|kept| !same_emoji(kept, emoji));
        self.list.insert(0, emoji.to_owned());
        self.list.truncate(Self::KEPT);
    }

    /// Newest first.
    pub fn list(&self) -> &[String] {
        &self.list
    }
}
