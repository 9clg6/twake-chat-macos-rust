//! What the text before the cursor asks the suggestion layer for.

/// The list asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionKind {
    /// After "@".
    Person,
    /// After "#".
    Room,
    /// After ":".
    Emoji,
    /// After "/" at the very start.
    Command,
}

/// A symbol and what follows it up to the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trigger {
    /// The list asked for.
    pub kind: SuggestionKind,
    /// The byte index of the symbol (`@ # : /`).
    pub start: usize,
    /// What follows the symbol up to the cursor.
    pub query: String,
}

/// A letter, then letters, digits and `_ + -`.
fn is_emoji_query(query: &str) -> bool {
    let mut chars = query.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || "_+-".contains(c))
}

/// The suggestion layer asked by the text before `cursor` (a byte index; one inside a letter goes
/// back to its start). "@" and "#" at the start or after whitespace, then no space; ":" likewise,
/// then an emoji name; "/" at the very start of the text, then no space and no "/".
pub fn find_trigger(text: &str, cursor: usize) -> Option<Trigger> {
    let before = &text[..text.floor_char_boundary(cursor)];
    let start = before
        .char_indices()
        .rfind(|(_, c)| c.is_whitespace())
        .map_or(0, |(at, c)| at + c.len_utf8());
    let word = &before[start..];
    let symbol = word.chars().next()?;
    let query = &word[symbol.len_utf8()..];
    let kind = match symbol {
        '@' => SuggestionKind::Person,
        '#' => SuggestionKind::Room,
        ':' if is_emoji_query(query) => SuggestionKind::Emoji,
        '/' if start == 0 && !query.contains('/') => SuggestionKind::Command,
        _ => return None,
    };
    Some(Trigger {
        kind,
        start,
        query: query.into(),
    })
}
