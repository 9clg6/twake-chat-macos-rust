//! Slash commands of the client and commands of the bots of the room (D27, FR-061).

use crate::UserId;

const SHRUG: &str = "¯\\_(ツ)_/¯";

/// A command a bot announced in the room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotCommand {
    /// The bot that answers it.
    pub bot: UserId,
    /// The first word of the syntax: `leave`.
    pub name: String,
    /// The whole syntax: `leave {days} {reason...}`.
    pub syntax: String,
    /// What it does.
    pub description: String,
}

/// A command that runs on the device and sends nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalCommand {
    /// `/discardsession`: the next message goes out with a new encryption session.
    DiscardSession,
}

/// What a text typed in the composer comes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prepared {
    /// A text message; `//x` gives `/x`, `/shrug` gives the shrug.
    Message {
        /// The text to send.
        body: String,
    },
    /// `/me waves` gives `waves`.
    Emote {
        /// The text of the emote.
        body: String,
    },
    /// A command of a bot of the room: `/leave 2` gives `!leave 2`.
    Bot {
        /// The command.
        command: BotCommand,
        /// The text to send.
        body: String,
    },
    /// A command of the device.
    Local(LocalCommand),
    /// `/foo bar` gives `foo` (lower case); nothing is sent.
    Unknown(String),
}

fn message(body: impl Into<String>) -> Prepared {
    Prepared::Message { body: body.into() }
}

/// The rules of `prepareCommandText` and `findLocalCommand`, on a text already trimmed. A name
/// is `[^\s/]+` followed by a space or the end, case ignored. The commands of the room win over
/// those of the client.
pub fn prepare(text: &str, commands: &[BotCommand]) -> Prepared {
    let Some(rest) = text.strip_prefix('/') else {
        return message(text);
    };
    if rest.starts_with('/') {
        return message(rest);
    }
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let name = &rest[..end];
    if name.is_empty() || name.contains('/') {
        return message(text);
    }
    let args = rest[end..].trim_start();
    command(&name.to_lowercase(), args, commands)
}

fn command(name: &str, args: &str, commands: &[BotCommand]) -> Prepared {
    if let Some(found) = commands.iter().find(|c| c.name.to_lowercase() == name) {
        let body = if args.is_empty() {
            format!("!{}", found.name)
        } else {
            format!("!{} {args}", found.name)
        };
        return Prepared::Bot {
            command: found.clone(),
            body,
        };
    }
    match (name, args.is_empty()) {
        ("me", false) => Prepared::Emote { body: args.into() },
        ("shrug", true) => message(SHRUG),
        ("shrug", false) => message(format!("{args} {SHRUG}")),
        ("discardsession", _) => Prepared::Local(LocalCommand::DiscardSession),
        _ => Prepared::Unknown(name.into()),
    }
}

/// D27: the commands whose name starts with `query`, case ignored, in the order announced.
pub fn match_commands<'a>(commands: &'a [BotCommand], query: &str) -> Vec<&'a BotCommand> {
    let query = query.to_lowercase();
    commands
        .iter()
        .filter(|c| c.name.to_lowercase().starts_with(&query))
        .collect()
}
