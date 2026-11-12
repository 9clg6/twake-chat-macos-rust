#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    dead_code
)]
//! Text buffer: composition (IME and dead keys), as macOS drives it through `NSTextInputClient`
//! (`setMarkedText`, `insertText`, `unmarkText`).
//!
//! Need (not the code): `docs/spec/features/composer.md` D602 (US International: the dead-key
//! backtick or acute is composed with the next character), `docs/dev/text-input-api-sketch.md`
//! (Composition). "´" is 2 bytes, "に" 3, "日" 3.

mod tb;

use tb::{at, buf, sel, span};
use twake_ui_kit::text_buffer::{Move, TextBuffer};

fn composing(text: &str, cursor: usize, marked: &str) -> TextBuffer {
    let mut b = at(text, cursor);
    b.set_marked_text(None, marked);
    b
}

#[test]
fn marked_text_is_part_of_the_text_and_is_reported() {
    let b = composing("ab", 1, "´");
    assert_eq!(b.text(), "a´b");
    assert_eq!(b.marked_range(), Some(1..3));
    assert_eq!(span(&b), (3, 3));
}

#[test]
fn a_new_marked_text_replaces_the_previous_one_in_place() {
    let mut b = composing("axb", 1, "ni");
    assert_eq!(b.text(), "anixb");
    b.set_marked_text(None, "に");
    assert_eq!(b.text(), "aにxb");
    assert_eq!(b.marked_range(), Some(1..4));
    assert_eq!(span(&b), (4, 4));
}

#[test]
fn marked_text_replaces_the_selection() {
    let mut b = sel("abcd", 1, 3);
    b.set_marked_text(None, "X");
    assert_eq!(b.text(), "aXd");
    assert_eq!(b.marked_range(), Some(1..2));
    assert_eq!(span(&b), (2, 2));
}

#[test]
fn marked_text_can_replace_a_given_range() {
    let mut b = buf("abcd");
    b.set_marked_text(Some(1..3), "Z");
    assert_eq!(b.text(), "aZd");
    assert_eq!(b.marked_range(), Some(1..2));
}

#[test]
fn a_replacement_range_past_the_end_is_clamped() {
    let mut b = buf("abc");
    b.set_marked_text(Some(2..99), "Z");
    assert_eq!(b.text(), "abZ");
    assert_eq!(b.marked_range(), Some(2..3));
}

#[test]
fn marked_text_is_cleaned_and_cut_like_any_input() {
    let mut b = TextBuffer::with_max_len(3);
    b.set_marked_text(None, "a\u{7}bcdef");
    assert_eq!(b.text(), "abc");
}

#[test]
fn an_empty_marked_text_ends_the_composition() {
    let mut b = composing("x", 1, "ni");
    b.set_marked_text(None, "");
    assert_eq!(b.text(), "x");
    assert_eq!(b.marked_range(), None);
    assert_eq!(span(&b), (1, 1));
}

// --- insert while composing: the dead key cases

#[test]
fn a_dead_key_and_a_letter_make_one_accented_letter() {
    let mut b = composing("ab", 1, "´");
    b.insert("é");
    assert_eq!(b.text(), "aéb");
    assert_eq!(b.marked_range(), None);
    assert_eq!(span(&b), (3, 3));
}

#[test]
fn a_dead_key_and_a_letter_that_does_not_combine_keep_both() {
    let mut b = composing("ab", 1, "´");
    b.insert("´x");
    assert_eq!(b.text(), "a´xb");
    assert_eq!(b.marked_range(), None);
    assert_eq!(span(&b), (4, 4));
}

#[test]
fn a_dead_key_and_a_space_leave_the_bare_accent() {
    let mut b = composing("", 0, "´");
    b.insert("´");
    assert_eq!(b.text(), "´");
    assert_eq!(b.marked_range(), None);
}

// --- commit and cancel

#[test]
fn commit_keeps_the_text_and_drops_the_mark() {
    let mut b = composing("", 0, "に");
    b.commit_marked();
    assert_eq!(b.text(), "に");
    assert_eq!(b.marked_range(), None);
    assert_eq!(span(&b), (3, 3));
}

#[test]
fn commit_leaves_the_cursor_after_the_cluster_the_text_joined() {
    let mut b = composing("\u{301}", 0, "e");
    assert_eq!(b.marked_range(), Some(0..1));
    b.commit_marked();
    assert_eq!(span(&b), (3, 3));
    assert_eq!(b.marked_range(), None);
}

#[test]
fn cancel_removes_the_marked_text() {
    let mut b = composing("ab", 1, "´");
    b.cancel_marked();
    assert_eq!(b.text(), "ab");
    assert_eq!(b.marked_range(), None);
    assert_eq!(span(&b), (1, 1));
}

#[test]
fn cancel_brings_back_the_selection_the_composition_replaced() {
    let mut b = sel("abcd", 1, 3);
    b.set_marked_text(None, "n");
    b.set_marked_text(None, "ni");
    assert_eq!(b.text(), "anid");
    b.cancel_marked();
    assert_eq!(b.text(), "abcd");
    assert_eq!(span(&b), (1, 3));
}

#[test]
fn cancel_brings_back_the_range_the_composition_replaced() {
    let mut b = buf("abcd");
    b.set_marked_text(Some(1..3), "Z");
    b.cancel_marked();
    assert_eq!(b.text(), "abcd");
    assert_eq!(span(&b), (4, 4));
}

#[test]
fn commit_and_cancel_without_a_composition_do_nothing() {
    let mut b = sel("abcd", 1, 3);
    b.commit_marked();
    b.cancel_marked();
    assert_eq!(b.text(), "abcd");
    assert_eq!(span(&b), (1, 3));
    assert!(!b.can_undo());
}

#[test]
fn a_click_during_a_composition_commits_it() {
    let mut b = composing("", 0, "に");
    b.select(0, 0);
    assert_eq!(b.text(), "に");
    assert_eq!(b.marked_range(), None);
    assert_eq!(span(&b), (0, 0));
}

#[test]
fn a_move_during_a_composition_commits_it() {
    let mut b = composing("a", 1, "に");
    b.move_by(Move::Left, false);
    assert_eq!(b.text(), "aに");
    assert_eq!(b.marked_range(), None);
}

#[test]
fn set_text_drops_the_composition() {
    let mut b = composing("a", 1, "に");
    b.set_text("x");
    assert_eq!(b.text(), "x");
    assert_eq!(b.marked_range(), None);
}

// --- history

#[test]
fn an_open_composition_is_not_an_undo_step() {
    let mut b = composing("", 0, "ni");
    b.set_marked_text(None, "に");
    assert!(!b.can_undo());
}

#[test]
fn the_committed_phrase_is_one_undo_step() {
    let mut b = composing("", 0, "ni");
    b.set_marked_text(None, "に");
    b.insert("日");
    assert_eq!(b.text(), "日");
    assert!(b.undo());
    assert_eq!(b.text(), "");
    assert!(b.redo());
    assert_eq!(b.text(), "日");
}

#[test]
fn a_commit_by_unmark_is_one_undo_step() {
    let mut b = composing("", 0, "に");
    b.commit_marked();
    assert!(b.undo());
    assert_eq!(b.text(), "");
}

#[test]
fn a_cancelled_composition_leaves_no_undo_step() {
    let mut b = composing("", 0, "に");
    b.cancel_marked();
    assert!(!b.can_undo());
}

#[test]
fn a_committed_phrase_is_not_merged_with_the_typing_before_it() {
    let mut b = buf("");
    b.insert("ab");
    b.set_marked_text(None, "に");
    b.insert("日");
    assert!(b.undo());
    assert_eq!(b.text(), "ab");
    assert!(b.undo());
    assert_eq!(b.text(), "");
}

#[test]
fn undo_and_redo_are_refused_during_a_composition() {
    let mut b = buf("");
    b.insert("a");
    b.set_marked_text(None, "に");
    assert!(!b.undo());
    assert!(!b.redo());
    assert_eq!(b.text(), "aに");
    assert_eq!(b.marked_range(), Some(1..4));
}
