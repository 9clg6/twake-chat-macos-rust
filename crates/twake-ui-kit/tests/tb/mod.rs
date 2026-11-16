#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    dead_code
)]
//! Shared helpers of the text buffer specs (see `docs/dev/text-input-api-sketch.md`).

use twake_ui_kit::text_buffer::{Delete, Move, TextBuffer};

/// Man, woman, girl joined by ZWJ: one cluster, 5 chars, 18 bytes.
pub const FAMILY: &str = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
/// Regional indicators F + R: one cluster, 2 chars, 8 bytes.
pub const FR: &str = "\u{1F1EB}\u{1F1F7}";
/// Regional indicators D + E: one cluster, 2 chars, 8 bytes.
pub const DE: &str = "\u{1F1E9}\u{1F1EA}";
/// A lone regional indicator D: 4 bytes.
pub const LONE_D: &str = "\u{1F1E9}";
/// `e` and a combining acute accent: one cluster, 2 chars, 3 bytes.
pub const E_ACUTE: &str = "e\u{301}";

/// A buffer holding `text`, the cursor at its end.
pub fn buf(text: &str) -> TextBuffer {
    let mut b = TextBuffer::new();
    b.set_text(text);
    b
}

/// A buffer holding `text`, the cursor at `cursor`.
pub fn at(text: &str, cursor: usize) -> TextBuffer {
    sel(text, cursor, cursor)
}

/// A buffer holding `text` with a selection from `anchor` to `head`.
pub fn sel(text: &str, anchor: usize, head: usize) -> TextBuffer {
    let mut b = buf(text);
    b.select(anchor, head);
    b
}

/// (anchor, head).
pub fn span(b: &TextBuffer) -> (usize, usize) {
    let s = b.selection();
    (s.anchor, s.head)
}

/// Where the cursor is after one move from `cursor`.
pub fn head_after(text: &str, cursor: usize, to: Move) -> usize {
    let mut b = at(text, cursor);
    b.move_by(to, false);
    span(&b).1
}

/// The cursor after each of `moves`, one buffer, from `cursor`.
pub fn heads_after(text: &str, cursor: usize, moves: &[Move]) -> Vec<usize> {
    let mut b = at(text, cursor);
    moves
        .iter()
        .map(|m| {
            b.move_by(*m, false);
            span(&b).1
        })
        .collect()
}

/// The text and the cursor after one deletion with the cursor at `cursor`.
pub fn deleted(text: &str, cursor: usize, kind: Delete) -> (String, usize) {
    let mut b = at(text, cursor);
    b.delete(kind);
    (b.text().to_string(), span(&b).1)
}

/// Types each piece with its own `insert`.
pub fn typed(b: &mut TextBuffer, pieces: &[&str]) {
    for p in pieces {
        b.insert(p);
    }
}
