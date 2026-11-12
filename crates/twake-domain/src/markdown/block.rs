//! The lines of a message grouped in blocks: code, quote, list, text.

use super::inline::{escape, inline};

/// A group of lines.
#[derive(Debug, PartialEq, Eq)]
enum Block<'a> {
    Code(String),
    Quote(Vec<&'a str>),
    List { ordered: bool, items: Vec<&'a str> },
    Text(Vec<&'a str>),
}

/// What one line is, before its neighbours are looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Line<'a> {
    Quote(&'a str),
    Item { ordered: bool, content: &'a str },
    Plain(&'a str),
}

impl<'a> Line<'a> {
    fn of(line: &'a str) -> Self {
        if let Some(content) = line.strip_prefix("> ") {
            return Self::Quote(content);
        }
        if let Some(content) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            return Self::Item {
                ordered: false,
                content,
            };
        }
        match numbered(line) {
            Some(content) => Self::Item {
                ordered: true,
                content,
            },
            None => Self::Plain(line),
        }
    }

    fn same_kind(self, other: Self) -> bool {
        match (self, other) {
            (Self::Quote(_), Self::Quote(_)) | (Self::Plain(_), Self::Plain(_)) => true,
            (Self::Item { ordered: a, .. }, Self::Item { ordered: b, .. }) => a == b,
            _ => false,
        }
    }

    fn content(self) -> &'a str {
        match self {
            Self::Quote(content) | Self::Plain(content) | Self::Item { content, .. } => content,
        }
    }
}

/// `1. a` or `1) a`: the text after the marker.
fn numbered(line: &str) -> Option<&str> {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    let after = line.get(digits..).filter(|_| digits > 0)?;
    after
        .strip_prefix(". ")
        .or_else(|| after.strip_prefix(") "))
}

/// The opening line of a fence: three backticks and at most a language word.
fn is_fence_open(line: &str) -> bool {
    line.strip_prefix("```").is_some_and(|info| {
        info.chars()
            .all(|c| c.is_ascii_alphanumeric() || "_+-.#".contains(c))
    })
}

/// A fence at the start of `lines`, and the lines it takes. An unclosed fence is no fence.
fn fence<'a>(lines: &[&'a str]) -> Option<(Block<'a>, usize)> {
    let first = lines.first()?.trim_end();
    if let Some(code) = first
        .strip_prefix("```")
        .and_then(|rest| rest.strip_suffix("```"))
        .filter(|code| !code.is_empty() && !code.contains('`'))
    {
        return Some((Block::Code(code.to_string()), 1));
    }
    if !is_fence_open(first) {
        return None;
    }
    let inside = lines.get(1..)?;
    let close = inside.iter().position(|l| l.trim_end() == "```")?;
    Some((Block::Code(inside[..close].join("\n")), close + 2))
}

/// How many lines, from the first, are of its kind; a text stops where a fence begins.
fn run_len(lines: &[&str]) -> usize {
    let first = Line::of(lines[0]);
    let breaks = |at: usize| matches!(first, Line::Plain(_)) && fence(&lines[at..]).is_some();
    (1..lines.len())
        .find(|&at| !Line::of(lines[at]).same_kind(first) || breaks(at))
        .unwrap_or(lines.len())
}

fn next_block<'a>(lines: &[&'a str]) -> (Block<'a>, usize) {
    if let Some(found) = fence(lines) {
        return found;
    }
    let used = run_len(lines);
    let kinds: Vec<Line<'a>> = lines[..used].iter().map(|l| Line::of(l)).collect();
    let contents = kinds.iter().map(|k| k.content()).collect();
    let block = match kinds[0] {
        Line::Quote(_) => Block::Quote(contents),
        Line::Item { ordered, .. } => Block::List {
            ordered,
            items: contents,
        },
        Line::Plain(_) => Block::Text(contents),
    };
    (block, used)
}

fn html_of(block: &Block) -> String {
    match block {
        Block::Code(code) => format!("<pre><code>{}</code></pre>", escape(code)),
        Block::Quote(lines) => format!("<blockquote>{}</blockquote>", joined(lines)),
        Block::Text(lines) => joined(lines),
        Block::List { ordered, items } => {
            let tag = if *ordered { "ol" } else { "ul" };
            let items: String = items
                .iter()
                .map(|item| format!("<li>{}</li>", inline(item)))
                .collect();
            format!("<{tag}>{items}</{tag}>")
        }
    }
}

fn joined(lines: &[&str]) -> String {
    lines
        .iter()
        .map(|l| inline(l))
        .collect::<Vec<_>>()
        .join("<br>")
}

/// The HTML of the lines of a message.
pub(super) fn render(lines: &[&str]) -> String {
    let mut html = String::new();
    let mut rest = lines;
    while !rest.is_empty() {
        let (block, used) = next_block(rest);
        html.push_str(&html_of(&block));
        rest = &rest[used..];
    }
    html
}
