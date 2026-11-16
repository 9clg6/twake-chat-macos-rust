#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    dead_code
)]
//! Text buffer: properties that hold after ANY sequence of operations.
//!
//! Need (not the code): `docs/dev/text-input-api-sketch.md` (conventions: cursor and selection
//! always on grapheme boundaries and inside the text, except during a composition; sanitising;
//! `max_len`; history; UTF-16 round trip). The grapheme oracle is `unicode-segmentation`.

mod tb;

use proptest::prelude::*;
use std::collections::HashSet;
use twake_ui_kit::text_buffer::{Delete, Move, TextBuffer};
use unicode_segmentation::UnicodeSegmentation;

const PIECES: &[&str] = &[
    "a",
    "b",
    " ",
    "\n",
    "\t",
    "\r\n",
    "\r",
    "\u{0}",
    "\u{7}",
    "é",
    "e\u{301}",
    "\u{301}",
    "\u{1F44D}",
    "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
    "\u{1F1EB}\u{1F1F7}",
    "\u{1F1E9}",
    "日本",
    "´",
    "don't",
    "hi,",
];

#[derive(Debug, Clone)]
enum Op {
    Insert(String),
    Paste(String),
    Delete(Delete),
    Move(Move, bool),
    Select(usize, usize),
    SelectAll,
    Word(usize),
    Line(usize),
    Marked(Option<(usize, usize)>, String),
    Commit,
    Cancel,
    Undo,
    Redo,
}

fn text_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(PIECES), 0..4).prop_map(|v| v.concat())
}

fn delete_strategy() -> impl Strategy<Value = Delete> {
    prop::sample::select(vec![
        Delete::Backward,
        Delete::Forward,
        Delete::WordBackward,
        Delete::WordForward,
        Delete::ToLineStart,
    ])
}

fn move_strategy() -> impl Strategy<Value = Move> {
    prop::sample::select(vec![
        Move::Left,
        Move::Right,
        Move::WordLeft,
        Move::WordRight,
        Move::LineStart,
        Move::LineEnd,
        Move::Up,
        Move::Down,
        Move::DocStart,
        Move::DocEnd,
    ])
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => text_strategy().prop_map(Op::Insert),
        1 => text_strategy().prop_map(Op::Paste),
        2 => delete_strategy().prop_map(Op::Delete),
        3 => (move_strategy(), any::<bool>()).prop_map(|(m, e)| Op::Move(m, e)),
        1 => (0..40usize, 0..40usize).prop_map(|(a, b)| Op::Select(a, b)),
        1 => Just(Op::SelectAll),
        1 => (0..40usize).prop_map(Op::Word),
        1 => (0..40usize).prop_map(Op::Line),
        2 => (prop::option::of((0..40usize, 0..40usize)), text_strategy())
            .prop_map(|(r, t)| Op::Marked(r, t)),
        1 => Just(Op::Commit),
        1 => Just(Op::Cancel),
        2 => Just(Op::Undo),
        1 => Just(Op::Redo),
    ]
}

fn apply(b: &mut TextBuffer, op: &Op) {
    match op {
        Op::Insert(t) => b.insert(t),
        Op::Paste(t) => b.paste(t),
        Op::Delete(k) => b.delete(*k),
        Op::Move(m, e) => b.move_by(*m, *e),
        Op::Select(x, y) => b.select(*x, *y),
        Op::SelectAll => b.select_all(),
        Op::Word(o) => b.select_word_at(*o),
        Op::Line(o) => b.select_line_at(*o),
        Op::Marked(r, t) => b.set_marked_text(r.map(|(s, e)| s..e), t),
        Op::Commit => b.commit_marked(),
        Op::Cancel => b.cancel_marked(),
        Op::Undo => {
            b.undo();
        }
        Op::Redo => {
            b.redo();
        }
    }
}

fn boundaries(text: &str) -> HashSet<usize> {
    text.grapheme_indices(true)
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .collect()
}

fn clean(text: &str) -> bool {
    text.chars()
        .all(|c| !c.is_control() || c == '\n' || c == '\t')
}

fn check(b: &TextBuffer, max: Option<usize>) -> Result<(), TestCaseError> {
    let text = b.text();
    prop_assert!(clean(text), "control character in {text:?}");
    if let Some(max) = max {
        prop_assert!(text.chars().count() <= max, "too long: {text:?}");
    }
    let s = b.selection();
    prop_assert!(
        s.anchor <= text.len() && s.head <= text.len(),
        "out of bounds"
    );
    prop_assert!(text.is_char_boundary(s.anchor) && text.is_char_boundary(s.head));
    match b.marked_range() {
        Some(r) => {
            prop_assert!(r.start <= r.end && r.end <= text.len());
            prop_assert!(text.is_char_boundary(r.start) && text.is_char_boundary(r.end));
        }
        None => {
            let ok = boundaries(text);
            prop_assert!(
                ok.contains(&s.anchor) && ok.contains(&s.head),
                "{s:?} not on a grapheme boundary of {text:?}"
            );
        }
    }
    Ok(())
}

fn run(max: Option<usize>, ops: &[Op]) -> Result<TextBuffer, TestCaseError> {
    let mut b = max.map_or_else(TextBuffer::new, TextBuffer::with_max_len);
    for op in ops {
        apply(&mut b, op);
        check(&b, max)?;
    }
    Ok(b)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn cursor_and_selection_stay_valid_after_any_operations(
        max in prop::option::of(1..12usize),
        ops in prop::collection::vec(op_strategy(), 0..40),
    ) {
        run(max, &ops)?;
    }

    #[test]
    fn undoing_every_step_reaches_the_empty_start(
        max in prop::option::of(1..12usize),
        ops in prop::collection::vec(op_strategy(), 0..40),
    ) {
        let mut b = run(max, &ops)?;
        b.commit_marked();
        while b.undo() {
            check(&b, max)?;
        }
        prop_assert_eq!(b.text(), "", "undoing every step must reach the empty start");
        prop_assert_eq!(b.selection().range(), 0..0);
    }

    #[test]
    fn redo_after_undo_is_the_identity_on_the_text_and_the_selection(
        ops in prop::collection::vec(op_strategy(), 0..40),
    ) {
        // A paste ends the sequence so that the selection is the one the last step left
        // (a move after the last edit is not in the history).
        let mut b = run(None, &ops)?;
        b.commit_marked();
        b.paste("Z");
        let (text, sel) = (b.text().to_string(), b.selection());
        let mut undone = 0;
        while b.undo() {
            undone += 1;
        }
        for _ in 0..undone {
            prop_assert!(b.redo());
            check(&b, None)?;
        }
        prop_assert_eq!(b.text(), text.as_str());
        prop_assert_eq!(b.selection(), sel);
    }

    #[test]
    fn utf16_offsets_round_trip(text in text_strategy()) {
        let mut b = TextBuffer::new();
        b.insert(&text);
        let t = b.text().to_string();
        for (o, _) in t.char_indices().chain(std::iter::once((t.len(), ' '))) {
            let units = t[..o].encode_utf16().count();
            prop_assert_eq!(b.utf8_to_utf16(o), units);
            prop_assert_eq!(b.utf16_to_utf8(units), o);
        }
    }

    #[test]
    fn utf16_range_conversion_round_trips(text in text_strategy()) {
        let mut b = TextBuffer::new();
        b.insert(&text);
        let all = 0..b.text().len();
        let units = b.utf8_range_to_utf16(all.clone());
        prop_assert_eq!(units.end, b.text().encode_utf16().count());
        prop_assert_eq!(b.utf16_range_to_utf8(units), all);
    }

    #[test]
    fn what_is_pasted_piece_by_piece_reads_back_as_the_cleaned_text(
        pieces in prop::collection::vec(prop::sample::select(PIECES), 0..8),
    ) {
        let mut b = TextBuffer::new();
        for p in &pieces {
            b.paste(p);
        }
        prop_assert!(clean(b.text()));
        prop_assert!(!b.text().contains('\r'));
        let wanted: String = pieces
            .iter()
            .flat_map(|p| p.replace("\r\n", "\n").replace('\r', "\n").chars().collect::<Vec<_>>())
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
            .collect();
        prop_assert_eq!(b.text(), wanted.as_str());
    }
}
