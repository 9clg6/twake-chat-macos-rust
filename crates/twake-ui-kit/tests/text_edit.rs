#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
//! The pure edit of the text field: which commands change the text, and which are left to the screen.

mod tb;

use tb::{at, buf, sel, span};
use twake_ui_kit::text_buffer::{Delete, Move};
use twake_ui_kit::view::text_keys::{Command, edit};

const TEXT: &str = "one two\nthree four";
/// Just after "three", on the second line.
const CURSOR: usize = 13;

fn applied(command: Command) -> String {
    let mut b = at(TEXT, CURSOR);
    edit(&mut b, command);
    b.text().to_string()
}

fn ignored(command: Command) {
    let mut b = sel(TEXT, 2, 11);
    edit(&mut b, command);
    assert_eq!(b.text(), TEXT);
    assert_eq!(span(&b), (2, 11));
}

#[test]
fn backspace_deletes_the_cluster_before_the_caret() {
    assert_eq!(
        applied(Command::Delete(Delete::Backward)),
        "one two\nthre four"
    );
}

#[test]
fn forward_delete_deletes_the_cluster_after_the_caret() {
    assert_eq!(
        applied(Command::Delete(Delete::Forward)),
        "one two\nthreefour"
    );
}

#[test]
fn alt_backspace_deletes_the_word_before_the_caret() {
    assert_eq!(
        applied(Command::Delete(Delete::WordBackward)),
        "one two\n four"
    );
}

#[test]
fn alt_forward_delete_deletes_the_word_after_the_caret() {
    assert_eq!(
        applied(Command::Delete(Delete::WordForward)),
        "one two\nthree"
    );
}

#[test]
fn cmd_backspace_deletes_to_the_start_of_the_line() {
    assert_eq!(
        applied(Command::Delete(Delete::ToLineStart)),
        "one two\n four"
    );
}

#[test]
fn newline_inserts_a_line_break_at_the_caret() {
    assert_eq!(applied(Command::Newline), "one two\nthree\n four");
}

#[test]
fn newline_replaces_the_selection() {
    let mut b = sel("hello world", 5, 11);
    edit(&mut b, Command::Newline);
    assert_eq!(b.text(), "hello\n");
}

#[test]
fn select_all_selects_the_whole_text() {
    let mut b = at(TEXT, CURSOR);
    edit(&mut b, Command::SelectAll);
    assert_eq!(b.selected_text(), TEXT);
}

#[test]
fn undo_takes_back_an_insertion() {
    let mut b = buf("ab");
    b.insert("cd");
    edit(&mut b, Command::Undo);
    assert_eq!(b.text(), "ab");
}

#[test]
fn redo_brings_back_an_undone_insertion() {
    let mut b = buf("ab");
    b.insert("cd");
    edit(&mut b, Command::Undo);
    edit(&mut b, Command::Redo);
    assert_eq!(b.text(), "abcd");
}

#[test]
fn a_move_without_extend_is_left_to_the_screen() {
    ignored(Command::Move(Move::Left, false));
}

#[test]
fn a_move_that_extends_is_left_to_the_screen() {
    ignored(Command::Move(Move::WordRight, true));
}

#[test]
fn submit_is_left_to_the_screen() {
    ignored(Command::Submit);
}

#[test]
fn copy_is_left_to_the_screen() {
    ignored(Command::Copy);
}

#[test]
fn cut_is_left_to_the_screen() {
    ignored(Command::Cut);
}

#[test]
fn paste_is_left_to_the_screen() {
    ignored(Command::Paste);
}
