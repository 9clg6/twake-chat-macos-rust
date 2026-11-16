#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    dead_code
)]
//! Text buffer: click, select all, double click (word), triple click (line).
//!
//! Need (not the code): `docs/dev/text-input-api-sketch.md` (conventions, double and triple click).
//! Offsets are bytes and always land on a grapheme boundary.

mod tb;

use tb::{FAMILY, at, buf, span};
use twake_ui_kit::text_buffer::TextBuffer;

fn after_word_click(text: &str, offset: usize) -> (usize, usize) {
    let mut b = buf(text);
    b.select_word_at(offset);
    span(&b)
}

fn after_line_click(text: &str, offset: usize) -> (usize, usize) {
    let mut b = buf(text);
    b.select_line_at(offset);
    span(&b)
}

#[test]
fn select_sets_anchor_and_head() {
    let mut b = buf("abcdef");
    b.select(4, 1);
    assert_eq!(span(&b), (4, 1));
    assert_eq!(b.selection().range(), 1..4);
    assert!(!b.selection().is_collapsed());
}

#[test]
fn a_cursor_is_a_collapsed_selection() {
    let b = at("abc", 2);
    assert!(b.selection().is_collapsed());
    assert_eq!(b.selection().range(), 2..2);
}

#[test]
fn an_offset_past_the_end_goes_to_the_end() {
    let mut b = buf("abc");
    b.select(1, 99);
    assert_eq!(span(&b), (1, 3));
}

#[test]
fn an_offset_inside_a_cluster_goes_to_its_start() {
    let text = format!("a{FAMILY}");
    let mut b = buf(&text);
    b.select(5, 100);
    assert_eq!(span(&b), (1, 19));
    b.select(2, 2);
    assert_eq!(span(&b), (1, 1));
    b.select(18, 18);
    assert_eq!(span(&b), (1, 1));
}

#[test]
fn an_offset_inside_a_char_goes_to_its_start() {
    let mut b = buf("éa");
    b.select(1, 1);
    assert_eq!(span(&b), (0, 0));
}

#[test]
fn select_all_covers_the_whole_text_from_the_start() {
    let mut b = at("héllo\nworld", 3);
    b.select_all();
    assert_eq!(span(&b), (0, 12));
    assert_eq!(b.selected_text(), "héllo\nworld");
}

#[test]
fn select_all_of_nothing_is_a_cursor() {
    let mut b = TextBuffer::new();
    b.select_all();
    assert_eq!(span(&b), (0, 0));
}

// --- double click

#[test]
fn a_double_click_selects_the_word_under_it() {
    assert_eq!(after_word_click("hello world", 2), (0, 5));
    assert_eq!(after_word_click("hello world", 8), (6, 11));
}

#[test]
fn a_double_click_at_the_end_of_the_text_selects_the_last_word() {
    assert_eq!(after_word_click("hello world", 11), (6, 11));
}

#[test]
fn a_double_click_on_spaces_selects_the_run_of_spaces() {
    assert_eq!(after_word_click("a   b", 2), (1, 4));
}

#[test]
fn a_double_click_on_a_word_with_an_apostrophe_takes_it_whole() {
    assert_eq!(after_word_click("say don't go", 7), (4, 9));
}

#[test]
fn a_double_click_inside_a_cluster_selects_the_cluster() {
    let text = format!("a{FAMILY} b");
    assert_eq!(after_word_click(&text, 5), (1, 19));
}

#[test]
fn a_word_boundary_inside_a_cluster_is_moved_to_a_cluster_edge() {
    // Word rules skip the accent and pair F with D; the cluster rules do not: "D+accent" | "F R".
    let text = "\u{1F1E9}\u{301}\u{1F1EB}\u{1F1F7}";
    let (a, h) = after_word_click(text, 0);
    assert!(
        [0, 6, 14].contains(&a) && [0, 6, 14].contains(&h),
        "{a} {h}"
    );
}

#[test]
fn a_double_click_in_an_empty_buffer_selects_nothing() {
    assert_eq!(after_word_click("", 0), (0, 0));
    assert_eq!(after_word_click("", 5), (0, 0));
}

#[test]
fn a_double_click_gives_the_selected_text() {
    let mut b = buf("hello world");
    b.select_word_at(8);
    assert_eq!(b.selected_text(), "world");
}

// --- triple click

#[test]
fn a_triple_click_selects_the_line_and_its_line_break() {
    let text = "ab\ncd\nef";
    assert_eq!(after_line_click(text, 4), (3, 6));
    assert_eq!(after_line_click(text, 0), (0, 3));
}

#[test]
fn the_last_line_has_no_line_break_to_take() {
    assert_eq!(after_line_click("ab\ncd\nef", 7), (6, 8));
    assert_eq!(after_line_click("ab\ncd\nef", 8), (6, 8));
}

#[test]
fn a_click_on_the_line_break_belongs_to_the_line_it_ends() {
    assert_eq!(after_line_click("ab\ncd\nef", 2), (0, 3));
    assert_eq!(after_line_click("ab\ncd\nef", 5), (3, 6));
}

#[test]
fn a_triple_click_on_an_empty_line_selects_its_break() {
    assert_eq!(after_line_click("ab\n\ncd", 3), (3, 4));
}

#[test]
fn a_triple_click_in_an_empty_buffer_selects_nothing() {
    assert_eq!(after_line_click("", 0), (0, 0));
}

#[test]
fn a_triple_click_gives_the_selected_line() {
    let mut b = buf("ab\ncd\nef");
    b.select_line_at(4);
    assert_eq!(b.selected_text(), "cd\n");
}
