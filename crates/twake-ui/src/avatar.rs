//! Initials and colour of an avatar (port of `getInitials` and `nameToColor` of twake-mui).

/// The colours of twake-mui `supportedColors`.
pub const AVATAR_COLOR_COUNT: usize = 11;

/// What an avatar without a picture draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvatarView {
    /// One to two graphemes, upper case.
    pub initials: String,
    /// An index of the palette, a function of the id only so a rename keeps the colour.
    pub color_index: usize,
}

impl AvatarView {
    /// The avatar of `name`, coloured by `id`.
    pub fn of(name: &str, id: &str) -> Self {
        Self {
            initials: initials(name),
            color_index: color_index(id),
        }
    }
}

/// FNV-1a, then modulo the palette.
fn color_index(id: &str) -> usize {
    let hash = id.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    (hash % AVATAR_COLOR_COUNT as u64) as usize
}

/// `@john.doe:server` reads as `john doe`.
fn readable(name: &str) -> String {
    match name.strip_prefix('@').and_then(|rest| rest.split_once(':')) {
        Some((local, _)) => local.replace(['.', '_', '-', '='], " "),
        None => name.to_owned(),
    }
}

fn is_mark(c: char) -> bool {
    matches!(c, '\u{300}'..='\u{36f}' | '\u{200d}' | '\u{fe0f}')
}

// ponytail: a grapheme is a char and its combining marks / joiner; full UAX 29 needs a crate.
fn graphemes(word: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in word.chars() {
        match out.last_mut() {
            Some(last) if is_mark(c) || last.ends_with('\u{200d}') => last.push(c),
            _ => out.push(c.to_string()),
        }
    }
    out
}

fn initials(name: &str) -> String {
    let text = readable(name);
    let words: Vec<&str> = text.split_whitespace().collect();
    let letters: Vec<&str> = words
        .iter()
        .copied()
        .filter(|w| w.chars().next().is_some_and(char::is_alphanumeric))
        .collect();
    let words = if letters.is_empty() { words } else { letters };
    let first = |word: &str| graphemes(word).into_iter().next().unwrap_or_default();
    let picked = match words.as_slice() {
        [] => String::new(),
        [one] => graphemes(one).into_iter().take(2).collect(),
        [head, .., tail] => first(head) + &first(tail),
    };
    picked.to_uppercase()
}
