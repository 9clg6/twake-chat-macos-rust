//! Pure functions on `&str` used by the text buffer: cleaning, grapheme and word boundaries,
//! lines, columns and UTF-16 offsets. Offsets are bytes; every function clamps its offset.

use std::ops::Range;
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

/// `"\r\n"` and `'\r'` become `'\n'`; every other control character but `'\t'` is dropped.
pub(super) fn sanitize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                chars.next_if_eq(&'\n');
                out.push('\n');
            }
            '\n' | '\t' => out.push(c),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// The longest start of `s` made of whole clusters that holds at most `room` chars.
pub(super) fn take_chars(s: &str, room: usize) -> &str {
    let mut used = 0;
    let mut end = 0;
    for g in s.graphemes(true) {
        used += g.chars().count();
        if used > room {
            break;
        }
        end += g.len();
    }
    &s[..end]
}

fn cursor_at(text: &str, off: usize) -> GraphemeCursor {
    GraphemeCursor::new(off, text.len(), true)
}

fn is_boundary(text: &str, off: usize) -> bool {
    cursor_at(text, off).is_boundary(text, 0).unwrap_or(true)
}

/// The cluster boundary before `off` (0 at the start).
pub(super) fn prev_boundary(text: &str, off: usize) -> usize {
    let off = text.floor_char_boundary(off);
    cursor_at(text, off)
        .prev_boundary(text, 0)
        .ok()
        .flatten()
        .unwrap_or(0)
}

/// The cluster boundary after `off` (the length at the end).
pub(super) fn next_boundary(text: &str, off: usize) -> usize {
    let off = text.floor_char_boundary(off);
    cursor_at(text, off)
        .next_boundary(text, 0)
        .ok()
        .flatten()
        .unwrap_or(text.len())
}

/// `off` itself on a cluster boundary, else the start of its cluster.
pub(super) fn floor_boundary(text: &str, off: usize) -> usize {
    let off = text.floor_char_boundary(off);
    if is_boundary(text, off) {
        off
    } else {
        prev_boundary(text, off)
    }
}

/// `off` itself on a cluster boundary, else the end of its cluster.
pub(super) fn ceil_boundary(text: &str, off: usize) -> usize {
    let off = text.floor_char_boundary(off);
    if is_boundary(text, off) {
        off
    } else {
        next_boundary(text, off)
    }
}

fn is_word(segment: &str) -> bool {
    segment.chars().any(char::is_alphanumeric)
}

/// The start of the word before `head`, separators skipped.
pub(super) fn word_left(text: &str, head: usize) -> usize {
    text.split_word_bound_indices()
        .rev()
        .filter(|(i, _)| *i < head)
        .find(|(_, s)| is_word(s))
        .map_or(0, |(i, _)| floor_boundary(text, i))
}

/// The end of the word after `head`, separators skipped.
pub(super) fn word_right(text: &str, head: usize) -> usize {
    text.split_word_bound_indices()
        .filter(|(i, s)| i + s.len() > head)
        .find(|(_, s)| is_word(s))
        .map_or(text.len(), |(i, s)| ceil_boundary(text, i + s.len()))
}

/// The word segment holding `off` (the last one at the end), on cluster edges.
pub(super) fn word_at(text: &str, off: usize) -> Range<usize> {
    let off = floor_boundary(text, off);
    text.split_word_bound_indices()
        .take_while(|(i, _)| *i <= off)
        .last()
        .map_or(0..0, |(i, s)| {
            floor_boundary(text, i)..ceil_boundary(text, i + s.len())
        })
}

/// The start of the line holding `off`.
pub(super) fn line_start(text: &str, off: usize) -> usize {
    let off = text.floor_char_boundary(off);
    text[..off].rfind('\n').map_or(0, |i| i + 1)
}

/// The end of the line holding `off`, before its `'\n'`.
pub(super) fn line_end(text: &str, off: usize) -> usize {
    let off = text.floor_char_boundary(off);
    text[off..].find('\n').map_or(text.len(), |i| off + i)
}

/// The line holding `off` with its trailing `'\n'`.
pub(super) fn line_at(text: &str, off: usize) -> Range<usize> {
    let off = floor_boundary(text, off);
    let end = line_end(text, off);
    line_start(text, off)..(end + usize::from(end < text.len()))
}

/// Clusters between the start of the line and `off`.
pub(super) fn column(text: &str, off: usize) -> usize {
    let off = text.floor_char_boundary(off);
    text[line_start(text, off)..off].graphemes(true).count()
}

fn offset_at_column(text: &str, start: usize, col: usize) -> usize {
    let line = &text[start..line_end(text, start)];
    start
        + line
            .grapheme_indices(true)
            .nth(col)
            .map_or(line.len(), |(i, _)| i)
}

/// The same column one line up; the start of the text on the first line.
pub(super) fn line_up(text: &str, head: usize, col: usize) -> usize {
    match line_start(text, head) {
        0 => 0,
        start => offset_at_column(text, line_start(text, start - 1), col),
    }
}

/// The same column one line down; the end of the text on the last line.
pub(super) fn line_down(text: &str, head: usize, col: usize) -> usize {
    match line_end(text, head) {
        end if end >= text.len() => end,
        end => offset_at_column(text, end + 1, col),
    }
}

/// UTF-16 units before the byte offset `off`.
pub(super) fn to_utf16(text: &str, off: usize) -> usize {
    text[..text.floor_char_boundary(off)].encode_utf16().count()
}

/// The byte offset of the char holding the UTF-16 unit `units` (the length past the end).
pub(super) fn from_utf16(text: &str, units: usize) -> usize {
    let mut seen = 0;
    for (i, c) in text.char_indices() {
        if seen + c.len_utf16() > units {
            return i;
        }
        seen += c.len_utf16();
    }
    text.len()
}
