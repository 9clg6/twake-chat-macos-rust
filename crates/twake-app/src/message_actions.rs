//! What the menu of a message offers, and what the bar of a selection offers (D274, D183, D260).

use twake_domain::power_levels::PowerLevels;
use twake_domain::timeline::{Message, SendState};
use twake_domain::{EventId, UserId};

/// An entry of the menu of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// The card of quick reactions at the head of the menu, with its chevron to the picker.
    React,
    /// Reply (section 1).
    Reply,
    /// Copy the text (section 1).
    CopyText,
    /// Pin (section 1).
    Pin,
    /// Unpin (section 1).
    Unpin,
    /// Start a selection (section 1).
    Select,
    /// Start a thread (section 2).
    Thread,
    /// Edit (section 2).
    Edit,
    /// Translate by the Scribe, for the reader alone (section 3, D588).
    Translate,
    /// Copy the link (section 4).
    CopyLink,
    /// Report (section 5).
    Report,
    /// Delete, last (section 5).
    Delete,
    /// Send again, a failed message only.
    Retry,
    /// Give up a failed message only.
    Discard,
}

/// What the message list knows beyond `Message`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MessageFacts {
    /// The message was deleted.
    pub deleted: bool,
    /// Its text could not be decrypted.
    pub undecryptable: bool,
    /// It is pinned.
    pub pinned: bool,
    /// It is a reply inside a thread.
    pub in_thread: bool,
}

/// What the room says about the actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomCtx {
    /// Its power levels.
    pub power: PowerLevels,
    /// The Scribe is offered here (D215).
    pub scribe: bool,
}

/// The message and its surroundings, as the rules read them.
struct Case<'a> {
    message: &'a Message,
    facts: &'a MessageFacts,
    room: &'a RoomCtx,
    me: &'a UserId,
}

impl Case<'_> {
    fn own(&self) -> bool {
        &self.message.sender == self.me
    }

    fn can_send(&self) -> bool {
        self.room.power.can_send_message(self.me)
    }

    fn can_pin(&self) -> bool {
        self.room.power.can_pin(self.me)
    }

    /// There is a text to copy or translate (D130).
    fn has_text(&self) -> bool {
        !self.message.body.is_empty() && !self.facts.undecryptable
    }

    fn can_delete(&self) -> bool {
        self.room.power.can_redact(self.me, &self.message.sender)
    }
}

type Rule = (Action, fn(&Case) -> bool);

/// The entries of the menu in order, each with its rule.
const RULES: [Rule; 12] = [
    (Action::React, |c| c.room.power.can_send_reaction(c.me)),
    (Action::Reply, |c| c.can_send()),
    (Action::CopyText, |c| c.has_text()),
    (Action::Pin, |c| c.can_pin() && !c.facts.pinned),
    (Action::Unpin, |c| c.can_pin() && c.facts.pinned),
    (Action::Select, |_| true),
    (Action::Thread, |c| {
        c.can_send() && !c.facts.in_thread && !c.facts.undecryptable
    }),
    (Action::Edit, |c| {
        c.own() && c.can_send() && !c.facts.undecryptable
    }),
    (Action::Translate, |c| {
        !c.own() && c.has_text() && c.room.scribe
    }),
    (Action::CopyLink, |_| true),
    (Action::Report, |c| !c.own()),
    (Action::Delete, |c| c.can_delete()),
];

/// The entries of the menu of `message`, in menu order. A message being sent or deleted has
/// none; a failed one has `Retry` and `Discard` alone.
pub fn message_actions(
    message: &Message,
    facts: &MessageFacts,
    room: &RoomCtx,
    me: &UserId,
) -> Vec<Action> {
    if facts.deleted {
        return Vec::new();
    }
    match message.send_state {
        SendState::Sending => Vec::new(),
        SendState::Failed => vec![Action::Retry, Action::Discard],
        SendState::Sent => {
            let case = Case {
                message,
                facts,
                room,
                me,
            };
            RULES
                .iter()
                .filter(|(_, allowed)| allowed(&case))
                .map(|(action, _)| *action)
                .collect()
        }
    }
}

/// The line of the menu an action sits on; the `React` card is above, in none.
fn section(action: Action) -> Option<u8> {
    match action {
        Action::React => None,
        Action::Reply | Action::CopyText | Action::Pin | Action::Unpin | Action::Select => Some(1),
        Action::Thread | Action::Edit => Some(2),
        Action::Translate => Some(3),
        Action::CopyLink => Some(4),
        Action::Report | Action::Delete => Some(5),
        Action::Retry | Action::Discard => Some(6),
    }
}

/// The menu lines (D274); an empty one is absent. The input order is kept.
pub fn menu_sections(actions: &[Action]) -> Vec<Vec<Action>> {
    (1..=6)
        .map(|line| {
            let on_line = |a: &&Action| section(**a) == Some(line);
            actions.iter().filter(on_line).copied().collect::<Vec<_>>()
        })
        .filter(|line| !line.is_empty())
        .collect()
}

/// An entry of the bar of a selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkAction {
    /// Copy the texts.
    Copy,
    /// Pin them.
    Pin,
    /// Unpin them.
    Unpin,
    /// Delete those that can be.
    Delete,
}

/// The bar of a selection, in order Copy, Pin or Unpin, Delete; none for an empty selection.
/// `Unpin` when every ticked message is pinned. `Delete` when one at least can be deleted.
pub fn selection_actions(
    items: &[(&Message, &MessageFacts)],
    room: &RoomCtx,
    me: &UserId,
) -> Vec<BulkAction> {
    if items.is_empty() {
        return Vec::new();
    }
    let all_pinned = items.iter().all(|(_, facts)| facts.pinned);
    let pin = match (room.power.can_pin(me), all_pinned) {
        (false, _) => None,
        (true, true) => Some(BulkAction::Unpin),
        (true, false) => Some(BulkAction::Pin),
    };
    let delete = (!bulk_delete_targets(items, room, me).is_empty()).then_some(BulkAction::Delete);
    [Some(BulkAction::Copy), pin, delete]
        .into_iter()
        .flatten()
        .collect()
}

/// The messages a bulk Delete removes: the ticked ones that are one's own or redactable, in the
/// order given. The others, and those with no event id yet, are skipped.
pub fn bulk_delete_targets(
    items: &[(&Message, &MessageFacts)],
    room: &RoomCtx,
    me: &UserId,
) -> Vec<EventId> {
    items
        .iter()
        .filter(|(message, _)| room.power.can_redact(me, &message.sender))
        .filter_map(|(message, _)| message.event_id().cloned())
        .collect()
}
