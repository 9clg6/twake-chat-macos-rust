//! What Enter makes of the text: the message to send, or why none.

use twake_domain::commands::{BotCommand, LocalCommand, Prepared};
use twake_domain::markdown::{OutgoingBody, escape_markdown, to_message};
use twake_domain::mention::{Mention, Mentions, extract, link_names};
use twake_domain::{EventId, UserId};

use super::{ComposerMode, DisabledReason};

/// A text message or an emote (`/me`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    /// `m.text`.
    Text,
    /// `m.emote`.
    Emote,
}

/// How the message relates to another event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Relation {
    /// None.
    None,
    /// `m.in_reply_to`.
    Reply {
        /// The event answered.
        to: EventId,
    },
    /// `m.replace`.
    Replace {
        /// The event replaced.
        target: EventId,
    },
}

/// A message ready for the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingMessage {
    /// Text or emote.
    pub kind: MessageKind,
    /// Trimmed, the command prepared (`!leave 2 x`, the emote text).
    pub body: String,
    /// Markdown as HTML with the matrix.to links of the mentions.
    pub formatted_body: Option<String>,
    /// Every mention (an edit: goes in `m.new_content`).
    pub mentions: Mentions,
    /// The top level `m.mentions`: as `mentions`, but an edit notifies the newly named only.
    pub notify: Mentions,
    /// The event answered or replaced.
    pub relation: Relation,
    /// The bot command, if the text was one.
    pub bot_command: Option<BotCommand>,
}

/// Why Enter sent nothing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SubmitError {
    /// Nothing, or spaces and newlines only.
    #[error("there is nothing to send")]
    Empty,
    /// `/foo bar`: the text stays.
    #[error("this command does not exist: {0}")]
    UnknownCommand(String),
    /// The composer cannot post.
    #[error("the composer is disabled: {0}")]
    Disabled(DisabledReason),
    /// Ran on the device, sends nothing.
    #[error("the command ran on this device")]
    LocalCommand(LocalCommand),
}

/// The picked mentions the message may name, and those it names anew (an edit notifies those).
fn mention_sets(mode: &ComposerMode, picked: &[Mention]) -> (Vec<Mention>, Vec<Mention>) {
    let ComposerMode::Edit(edit) = mode else {
        return (picked.to_vec(), picked.to_vec());
    };
    let newly: Vec<Mention> = picked
        .iter()
        .filter(|m| !edit.mentions.contains(m))
        .cloned()
        .collect();
    let all = edit.mentions.iter().chain(&newly).cloned().collect();
    (all, newly)
}

fn add_author(mentions: &mut Mentions, author: &UserId) {
    if !mentions.user_ids.contains(author) {
        mentions.user_ids.push(author.clone());
    }
}

/// `text` with each name written as the list writes it (Markdown escaped); the way back of
/// `unescape_names`. ponytail: names that contain one another are replaced in picked order.
pub(super) fn escape_names(text: &str, picked: &[Mention]) -> String {
    picked.iter().fold(text.to_string(), |text, m| {
        text.replace(&m.name, &escape_markdown(&m.name))
    })
}

/// `text` with each escaped name back to the name itself: what the plain body holds.
fn unescape_names(text: &str, picked: &[Mention]) -> String {
    picked.iter().fold(text.to_string(), |text, m| {
        text.replace(&escape_markdown(&m.name), &m.name)
    })
}

/// The kind, the text and the bot command of a prepared text; a command of the device and an
/// unknown one make no message.
fn parts(prepared: Prepared) -> Result<(MessageKind, String, Option<BotCommand>), SubmitError> {
    match prepared {
        Prepared::Message { body } => Ok((MessageKind::Text, body, None)),
        Prepared::Emote { body } => Ok((MessageKind::Emote, body, None)),
        Prepared::Bot { command, body } => Ok((MessageKind::Text, body, Some(command))),
        Prepared::Local(command) => Err(SubmitError::LocalCommand(command)),
        Prepared::Unknown(name) => Err(SubmitError::UnknownCommand(name)),
    }
}

/// The relation of the message; an answer also mentions (and notifies) its author unless that
/// is the user.
fn relate(
    mode: &ComposerMode,
    me: &UserId,
    mentions: &mut Mentions,
    notify: &mut Mentions,
) -> Relation {
    match mode {
        ComposerMode::Compose => Relation::None,
        ComposerMode::Edit(edit) => Relation::Replace {
            target: edit.event_id.clone(),
        },
        ComposerMode::Reply(reply) => {
            if reply.sender != *me {
                add_author(mentions, &reply.sender);
                add_author(notify, &reply.sender);
            }
            Relation::Reply {
                to: reply.event_id.clone(),
            }
        }
    }
}

/// The message for a prepared text; `picked` are the mentions chosen in the lists.
pub(super) fn build(
    mode: &ComposerMode,
    picked: &[Mention],
    me: &UserId,
    prepared: Prepared,
) -> Result<OutgoingMessage, SubmitError> {
    let (kind, body, bot_command) = parts(prepared)?;
    let (all, newly) = mention_sets(mode, picked);
    let markdown = to_message(&body);
    let body = unescape_names(&body, &all);
    let mut mentions = extract(&body, &all);
    let mut notify = match mode {
        ComposerMode::Edit(_) => extract(&body, &newly),
        _ => mentions.clone(),
    };
    let relation = relate(mode, me, &mut mentions, &mut notify);
    let linked = OutgoingBody {
        body: body.clone(),
        ..markdown
    };
    let formatted_body = link_names(linked, &all).formatted_body;
    Ok(OutgoingMessage {
        kind,
        body,
        formatted_body,
        mentions,
        notify,
        relation,
        bot_command,
    })
}
