//! The reactions of a message: `m.reaction` events grouped by emoji (D174).

use crate::{EventId, UserId};

/// Whether two emojis are the same one, ignoring the variation selector `\u{FE0F}` (D174).
pub fn same_emoji(a: &str, b: &str) -> bool {
    let bare = |s: &str| s.chars().filter(|c| *c != '\u{FE0F}').collect::<Vec<_>>();
    bare(a) == bare(b)
}

/// One `m.reaction` event: `sender` put `key` on a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionEvent {
    /// The id of the reaction event itself.
    pub event_id: EventId,
    /// Who reacted.
    pub sender: UserId,
    /// The emoji, as the sender spelled it.
    pub key: String,
}

/// The people who gave one emoji to one message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReactionGroup {
    key: String,
    senders: Vec<UserId>,
    events: Vec<EventId>,
}

impl ReactionGroup {
    fn first(event: ReactionEvent) -> Self {
        Self {
            key: event.key,
            senders: vec![event.sender],
            events: vec![event.event_id],
        }
    }

    /// The spelling first seen (D174).
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The people who reacted, first reaction first, once each.
    pub fn senders(&self) -> &[UserId] {
        &self.senders
    }

    /// How many people reacted.
    pub fn count(&self) -> usize {
        self.senders.len()
    }

    /// Whether `user` reacted with this emoji.
    pub fn reacted_by(&self, user: &UserId) -> bool {
        self.senders.contains(user)
    }

    /// The reaction event of `user`, if any.
    pub fn event_of(&self, user: &UserId) -> Option<&EventId> {
        let at = self.senders.iter().position(|s| s == user)?;
        self.events.get(at)
    }

    fn remove(&mut self, event: &EventId) -> bool {
        let Some(at) = self.events.iter().position(|e| e == event) else {
            return false;
        };
        self.events.remove(at);
        self.senders.remove(at);
        true
    }
}

/// The reactions of one message, one group per emoji.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reactions {
    groups: Vec<ReactionGroup>,
}

impl Reactions {
    /// No reaction.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a reaction to the group of its emoji, or starts a group, last. A second reaction of
    /// the same sender on the same emoji is ignored: the first event stays.
    pub fn add(&mut self, event: ReactionEvent) {
        match self
            .groups
            .iter_mut()
            .find(|g| same_emoji(&g.key, &event.key))
        {
            Some(group) if group.reacted_by(&event.sender) => {}
            Some(group) => {
                group.senders.push(event.sender);
                group.events.push(event.event_id);
            }
            None => self.groups.push(ReactionGroup::first(event)),
        }
    }

    /// The redaction of a reaction event; whether something was removed. A group left empty
    /// disappears.
    pub fn redact(&mut self, event_id: &EventId) -> bool {
        let removed = self.groups.iter_mut().any(|g| g.remove(event_id));
        self.groups.retain(|g| g.count() > 0);
        removed
    }

    /// The groups, in the order of their first reaction.
    pub fn groups(&self) -> &[ReactionGroup] {
        &self.groups
    }

    /// Whether nobody reacted.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// The group of `emoji`, whichever way it is spelled.
    pub fn group_of(&self, emoji: &str) -> Option<&ReactionGroup> {
        self.groups.iter().find(|g| same_emoji(&g.key, emoji))
    }
}
