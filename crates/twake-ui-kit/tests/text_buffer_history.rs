#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    dead_code
)]
//! Text buffer: undo and redo, merging of consecutive typing, boundaries at paste and deletion.
//!
//! Need (not the code): `docs/dev/text-input-api-sketch.md` (History). Typing "abc" is one step;
//! a paste, a deletion, or any move or click closes the run.

mod tb;

use tb::{at, buf, sel, span, typed};
use twake_ui_kit::text_buffer::{Delete, Move, TextBuffer};

#[test]
fn consecutive_typing_is_one_step() {
    let mut b = TextBuffer::new();
    typed(&mut b, &["a", "b", "c"]);
    assert!(b.undo());
    assert_eq!(b.text(), "");
    assert_eq!(span(&b), (0, 0));
    assert!(!b.can_undo());
    assert!(b.can_redo());
    assert!(b.redo());
    assert_eq!(b.text(), "abc");
    assert_eq!(span(&b), (3, 3));
    assert!(!b.can_redo());
}

#[test]
fn a_cursor_move_closes_the_typing_run() {
    let mut b = TextBuffer::new();
    typed(&mut b, &["a", "b"]);
    b.move_by(Move::Left, false);
    b.move_by(Move::Right, false);
    b.insert("c");
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    assert!(b.undo());
    assert_eq!(b.text(), "");
}

#[test]
fn a_click_closes_the_typing_run() {
    let mut b = TextBuffer::new();
    typed(&mut b, &["a", "b"]);
    b.select(2, 2);
    b.insert("c");
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
}

#[test]
fn a_paste_is_its_own_step_and_closes_the_run() {
    let mut b = TextBuffer::new();
    typed(&mut b, &["a", "b"]);
    b.paste("XY");
    b.insert("c");
    assert_eq!(b.text(), "abXYc");
    assert!(b.undo());
    assert_eq!(b.text(), "abXY");
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    assert!(b.undo());
    assert_eq!(b.text(), "");
}

#[test]
fn two_pastes_are_two_steps() {
    let mut b = TextBuffer::new();
    b.paste("one");
    b.paste("two");
    assert!(b.undo());
    assert_eq!(b.text(), "one");
}

#[test]
fn every_deletion_is_its_own_step() {
    let mut b = buf("abc");
    b.delete(Delete::Backward);
    b.delete(Delete::Backward);
    assert_eq!(b.text(), "a");
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    assert!(b.undo());
    assert_eq!(b.text(), "abc");
    assert!(!b.undo());
}

#[test]
fn a_deletion_closes_the_typing_run() {
    let mut b = TextBuffer::new();
    typed(&mut b, &["a", "b", "c"]);
    b.delete(Delete::Backward);
    b.insert("d");
    assert_eq!(b.text(), "abd");
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    assert!(b.undo());
    assert_eq!(b.text(), "abc");
    assert!(b.undo());
    assert_eq!(b.text(), "");
}

#[test]
fn typing_over_a_selection_is_one_step_with_what_follows() {
    let mut b = sel("abcd", 1, 3);
    b.insert("X");
    b.insert("Y");
    assert_eq!(b.text(), "aXYd");
    assert!(b.undo());
    assert_eq!(b.text(), "abcd");
    assert_eq!(span(&b), (1, 3));
    assert!(!b.can_undo());
}

#[test]
fn redo_gives_back_the_text_and_the_cursor_after_the_step() {
    let mut b = sel("abcd", 1, 3);
    b.insert("X");
    assert!(b.undo());
    assert!(b.redo());
    assert_eq!(b.text(), "aXd");
    assert_eq!(span(&b), (2, 2));
}

#[test]
fn undoing_a_deletion_brings_back_the_selection() {
    let mut b = sel("abc", 0, 2);
    b.delete(Delete::Backward);
    assert_eq!((b.text(), span(&b)), ("c", (0, 0)));
    assert!(b.undo());
    assert_eq!((b.text(), span(&b)), ("abc", (0, 2)));
    assert!(b.redo());
    assert_eq!((b.text(), span(&b)), ("c", (0, 0)));
}

#[test]
fn undoing_a_deletion_without_selection_brings_back_the_cursor() {
    let mut b = at("abc", 3);
    b.delete(Delete::Backward);
    assert!(b.undo());
    assert_eq!((b.text(), span(&b)), ("abc", (3, 3)));
}

#[test]
fn a_word_deletion_is_one_step() {
    let mut b = buf("hello world");
    b.delete(Delete::WordBackward);
    assert_eq!(b.text(), "hello ");
    assert!(b.undo());
    assert_eq!(b.text(), "hello world");
    assert_eq!(span(&b), (11, 11));
}

#[test]
fn a_line_deletion_is_one_step() {
    let mut b = buf("ab\ncd");
    b.delete(Delete::ToLineStart);
    assert!(b.undo());
    assert_eq!((b.text(), span(&b)), ("ab\ncd", (5, 5)));
}

#[test]
fn with_nothing_done_there_is_nothing_to_undo_or_redo() {
    let mut b = TextBuffer::new();
    assert!(!b.undo());
    assert!(!b.redo());
    assert!(!b.can_undo());
    assert!(!b.can_redo());
}

#[test]
fn a_new_edit_after_an_undo_forgets_the_redo() {
    let mut b = TextBuffer::new();
    b.insert("a");
    assert!(b.undo());
    b.insert("b");
    assert!(!b.can_redo());
    assert!(!b.redo());
    assert_eq!(b.text(), "b");
}

#[test]
fn set_text_forgets_the_history() {
    let mut b = TextBuffer::new();
    typed(&mut b, &["a", "b"]);
    b.undo();
    b.set_text("x");
    assert!(!b.can_undo());
    assert!(!b.can_redo());
    assert_eq!(b.text(), "x");
}

#[test]
fn moves_and_clicks_are_not_steps() {
    let mut b = buf("abc");
    b.move_by(Move::Left, false);
    b.move_by(Move::WordLeft, true);
    b.select(1, 2);
    b.select_all();
    b.select_word_at(1);
    b.select_line_at(1);
    assert!(!b.can_undo());
}

#[test]
fn undo_goes_back_one_step_at_a_time_to_the_start() {
    let mut b = TextBuffer::new();
    typed(&mut b, &["a", "b"]);
    b.paste("XY");
    b.delete(Delete::Backward);
    let mut seen = vec![b.text().to_string()];
    while b.undo() {
        seen.push(b.text().to_string());
    }
    assert_eq!(seen, ["abX", "abXY", "ab", ""]);
}
