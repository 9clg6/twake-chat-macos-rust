//! The lists offered after "@" and "#".

use std::cmp::Reverse;

use super::MAX_MENTIONS;
use crate::search::fold;
use crate::{RoomAlias, UserId};

/// A member of the room, as the lists see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The Matrix id.
    pub user_id: UserId,
    /// The name shown (a contact may carry a private one, D302).
    pub name: String,
    /// The name the person gave on Matrix.
    pub matrix_name: Option<String>,
}

/// One line of the list after "@".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MentionCandidate {
    /// A member.
    Person(Member),
    /// `@room`.
    Everyone,
}

/// A room that has an alias (D502).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkableRoom {
    /// The alias the link uses.
    pub alias: RoomAlias,
    /// The name shown.
    pub name: String,
}

/// Words of a folded text: split on spaces and `. _ : -`.
fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| c.is_whitespace() || ".:_-".contains(c))
        .filter(|w| !w.is_empty())
}

/// 2 when a word of `text` or one of `ids` starts with `query`, 1 when either holds it inside,
/// `None` otherwise. All folded.
fn rank(text: &str, ids: &[String], query: &str) -> Option<u8> {
    if words(text).any(|w| w.starts_with(query)) || ids.iter().any(|i| i.starts_with(query)) {
        Some(2)
    } else if text.contains(query) || ids.iter().any(|i| i.contains(query)) {
        Some(1)
    } else {
        None
    }
}

/// What an item is searched by, all folded.
struct Fields {
    /// Names and words, searched by word and inside.
    text: String,
    /// Whole ids, searched from their start and inside.
    ids: Vec<String>,
    /// The name the matches are ordered by.
    name: String,
}

/// The items that match, best first, then by folded name, at most `MAX_MENTIONS`.
fn ranked<T>(items: impl Iterator<Item = T>, query: &str, fields: impl Fn(&T) -> Fields) -> Vec<T> {
    let mut found: Vec<(u8, String, T)> = items
        .filter_map(|item| {
            let f = fields(&item);
            rank(&f.text, &f.ids, query).map(|r| (r, f.name, item))
        })
        .collect();
    found.sort_by(|a, b| (Reverse(a.0), &a.1).cmp(&(Reverse(b.0), &b.1)));
    found.truncate(MAX_MENTIONS);
    found.into_iter().map(|(_, _, item)| item).collect()
}

fn member_fields(member: &Member) -> Fields {
    let id = member.user_id.as_str();
    let matrix_name = member.matrix_name.as_deref().unwrap_or_default();
    let text = format!(
        "{} {matrix_name} {}",
        member.name,
        member.user_id.localpart()
    );
    Fields {
        text: fold(&text),
        ids: vec![fold(id), fold(id.trim_start_matches('@'))],
        name: fold(&member.name),
    }
}

/// Where `@room` stands in the list.
enum EveryoneSlot {
    First,
    Last,
    Absent,
}

fn everyone_slot(query: &str, listed: usize) -> EveryoneSlot {
    if query.is_empty() {
        return if listed > 0 {
            EveryoneSlot::Last
        } else {
            EveryoneSlot::Absent
        };
    }
    if ["room", "all", "everyone"]
        .iter()
        .any(|word| word.starts_with(query))
    {
        EveryoneSlot::First
    } else {
        EveryoneSlot::Absent
    }
}

/// The list after "@<query>" (D401, D431): see `docs/dev/composer-api-sketch.md`. The query may
/// start with a second "@" for a full id. The user is never offered; `@room` only when
/// `can_mention_room`.
pub fn match_members(
    members: &[Member],
    query: &str,
    me: &UserId,
    can_mention_room: bool,
) -> Vec<MentionCandidate> {
    let query = fold(query.strip_prefix('@').unwrap_or(query));
    let others = members.iter().filter(|m| &m.user_id != me);
    let mut list: Vec<MentionCandidate> = ranked(others, &query, |m| member_fields(m))
        .into_iter()
        .map(|m| MentionCandidate::Person(m.clone()))
        .collect();
    if can_mention_room {
        match everyone_slot(&query, list.len()) {
            EveryoneSlot::First => list.insert(0, MentionCandidate::Everyone),
            EveryoneSlot::Last => list.push(MentionCandidate::Everyone),
            EveryoneSlot::Absent => {}
        }
    }
    list
}

fn room_fields(room: &LinkableRoom) -> Fields {
    let alias = room.alias.as_str();
    let text = format!("{} {}", room.name, room.alias.localpart());
    Fields {
        text: fold(&text),
        ids: vec![fold(alias), fold(alias.trim_start_matches('#'))],
        name: fold(&room.name),
    }
}

/// The rooms after "#<query>" (D502): a word of the name or of the alias starting with the query
/// first, then containing it; folded; by name; at most `MAX_MENTIONS`.
pub fn match_rooms<'a>(rooms: &'a [LinkableRoom], query: &str) -> Vec<&'a LinkableRoom> {
    ranked(rooms.iter(), &fold(query), |r| room_fields(r))
}
