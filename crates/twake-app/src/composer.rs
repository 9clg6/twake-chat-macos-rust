//! The composer: text, cursor, reply and edit banner, suggestions, and what Enter sends
//! (FR-050 to FR-057, FR-061). Pure state; the screen draws it and does the I/O.

mod outgoing;
mod state;
mod suggest;
mod trigger;

pub use outgoing::{MessageKind, OutgoingMessage, Relation, SubmitError};
pub use state::{ComposerState, EscapeOutcome};
pub use suggest::{EmojiEntry, Sources, Suggestion};
pub use trigger::{SuggestionKind, Trigger, find_trigger};

use twake_domain::mention::Mention;
use twake_domain::{EventId, UserId};

/// The text and the mentions picked in it (D487), kept per room.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Draft {
    /// The text as typed.
    pub text: String,
    /// The mentions picked from the lists.
    pub mentions: Vec<Mention>,
}

/// What the round button is (FR-050).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryAction {
    /// The microphone: the field holds nothing but spaces.
    Mic,
    /// Send.
    Send,
}

/// Why the composer cannot post.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DisabledReason {
    /// FR-056, D194.
    #[error("only moderators can post")]
    OnlyModeratorsPost,
}

/// Whether the user can write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Can.
    Enabled,
    /// Cannot, and why.
    Disabled(DisabledReason),
}

/// FR-056: posting needs `required` (`events_default`); a member below it cannot.
pub fn can_post(my_power: i64, required: i64) -> Access {
    if my_power >= required {
        Access::Enabled
    } else {
        Access::Disabled(DisabledReason::OnlyModeratorsPost)
    }
}

/// The message a reply answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplyTarget {
    /// The event answered.
    pub event_id: EventId,
    /// Its author.
    pub sender: UserId,
    /// A short extract, for the banner.
    pub preview: String,
}

/// The message an edit replaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditTarget {
    /// The event edited.
    pub event_id: EventId,
    /// Its text.
    pub body: String,
    /// Its mentions.
    pub mentions: Vec<Mention>,
}

/// The banner above the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposerMode {
    /// A new message.
    Compose,
    /// An answer.
    Reply(ReplyTarget),
    /// An edit.
    Edit(EditTarget),
}

/// A selection in the text, in byte offsets on letter boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// Where it started.
    pub anchor: usize,
    /// Where it ends: the cursor.
    pub head: usize,
}
