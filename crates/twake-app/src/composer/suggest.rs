//! What the suggestion layer offers for a trigger, and what picking one writes.

use twake_domain::commands::{BotCommand, match_commands};
use twake_domain::markdown::escape_markdown;
use twake_domain::mention::{
    LinkableRoom, Member, Mention, MentionCandidate, match_members, match_rooms,
};
use twake_domain::{RoomAlias, UserId};

use super::trigger::{SuggestionKind, Trigger};

/// Most emojis one list offers.
const MAX_EMOJIS: usize = 8;

/// An emoji of the table: `:smile:` and its glyph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmojiEntry {
    /// The name between the colons.
    pub shortcode: String,
    /// The emoji itself.
    pub glyph: String,
}

/// What the suggestions draw from, given by the screen.
#[derive(Debug, Clone, Default)]
pub struct Sources {
    /// The members of the room.
    pub members: Vec<Member>,
    /// The rooms that have an alias.
    pub rooms: Vec<LinkableRoom>,
    /// The commands of the bots of the room, in the order announced.
    pub commands: Vec<BotCommand>,
    /// The emoji table.
    pub emojis: Vec<EmojiEntry>,
    /// The user may notify the whole room.
    pub can_mention_room: bool,
}

/// One line of a list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Suggestion {
    /// A person; `name` is what is written (the Matrix name, D302).
    Person {
        /// The Matrix id.
        user_id: UserId,
        /// The name written in the text.
        name: String,
    },
    /// `@room`.
    Everyone,
    /// A room, written by its alias.
    Room {
        /// The alias.
        alias: RoomAlias,
    },
    /// An emoji.
    Emoji {
        /// The emoji itself.
        glyph: String,
    },
    /// A bot command.
    Command {
        /// The name after the slash.
        name: String,
    },
}

fn person(candidate: MentionCandidate) -> Suggestion {
    match candidate {
        MentionCandidate::Everyone => Suggestion::Everyone,
        MentionCandidate::Person(m) => Suggestion::Person {
            name: m.matrix_name.unwrap_or(m.name),
            user_id: m.user_id,
        },
    }
}

/// The exact name first, then the names it starts, then the others, in table order.
fn emojis(table: &[EmojiEntry], query: &str) -> Vec<Suggestion> {
    let query = query.to_lowercase();
    let rank = |e: &EmojiEntry| {
        let code = e.shortcode.to_lowercase();
        [
            code == query,
            code.starts_with(&query),
            code.contains(&query),
        ]
        .iter()
        .position(|&hit| hit)
    };
    let mut hits: Vec<(usize, &EmojiEntry)> = table
        .iter()
        .filter_map(|e| rank(e).map(|r| (r, e)))
        .collect();
    hits.sort_by_key(|(r, _)| *r);
    hits.into_iter()
        .take(MAX_EMOJIS)
        .map(|(_, e)| Suggestion::Emoji {
            glyph: e.glyph.clone(),
        })
        .collect()
}

/// The list for `trigger`.
pub(super) fn suggestions(sources: &Sources, me: &UserId, trigger: &Trigger) -> Vec<Suggestion> {
    let query = trigger.query.as_str();
    match trigger.kind {
        SuggestionKind::Person => {
            match_members(&sources.members, query, me, sources.can_mention_room)
                .into_iter()
                .map(person)
                .collect()
        }
        SuggestionKind::Room => match_rooms(&sources.rooms, query)
            .into_iter()
            .map(|r| Suggestion::Room {
                alias: r.alias.clone(),
            })
            .collect(),
        SuggestionKind::Emoji => emojis(&sources.emojis, query),
        SuggestionKind::Command => match_commands(&sources.commands, query)
            .into_iter()
            .map(|c| Suggestion::Command {
                name: c.name.clone(),
            })
            .collect(),
    }
}

/// What picking `suggestion` writes in the text (a name with its Markdown escaped, so that it stays
/// literal), and the mention to remember (an emoji has none).
pub(super) fn written(suggestion: &Suggestion) -> (String, Option<Mention>) {
    match suggestion {
        Suggestion::Person { user_id, name } => (
            format!("{} ", escape_markdown(name)),
            Some(Mention::user(user_id.clone(), name)),
        ),
        Suggestion::Everyone => ("@room ".into(), Some(Mention::everyone())),
        Suggestion::Room { alias } => (
            format!("{} ", escape_markdown(alias.as_str())),
            Some(Mention::room(alias.clone(), alias.as_str())),
        ),
        Suggestion::Emoji { glyph } => (glyph.clone(), None),
        Suggestion::Command { name } => (format!("/{name} "), None),
    }
}
