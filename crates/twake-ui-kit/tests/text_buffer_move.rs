#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    dead_code
)]
//! Text buffer: cursor moves (grapheme, word, line, column, document) and their Shift variants.
//!
//! Need (not the code): `docs/spec/features/accessibility.md` B3 and B16 (keyboard only, no trap),
//! `docs/dev/text-input-api-sketch.md` (words, lines and columns). Offsets are bytes.

mod tb;

use tb::{DE, E_ACUTE, FAMILY, FR, LONE_D, at, buf, head_after, heads_after, sel, span};
use twake_ui_kit::text_buffer::Move;

// --- by grapheme

#[test]
fn right_steps_over_a_whole_zwj_emoji() {
    let text = format!("a{FAMILY}b");
    assert_eq!(
        heads_after(
            &text,
            0,
            &[Move::Right, Move::Right, Move::Right, Move::Right]
        ),
        [1, 19, 20, 20]
    );
}

#[test]
fn left_steps_back_over_a_whole_zwj_emoji() {
    let text = format!("a{FAMILY}b");
    assert_eq!(
        heads_after(&text, 20, &[Move::Left, Move::Left, Move::Left, Move::Left]),
        [19, 1, 0, 0]
    );
}

#[test]
fn flags_are_clusters_of_two_indicators() {
    let text = format!("{FR}{DE}");
    assert_eq!(heads_after(&text, 0, &[Move::Right, Move::Right]), [8, 16]);
    assert_eq!(heads_after(&text, 16, &[Move::Left, Move::Left]), [8, 0]);
}

#[test]
fn a_third_indicator_is_a_cluster_of_its_own() {
    let text = format!("{FR}{LONE_D}");
    assert_eq!(heads_after(&text, 0, &[Move::Right, Move::Right]), [8, 12]);
}

#[test]
fn a_letter_and_its_combining_accent_are_one_step() {
    let text = format!("{E_ACUTE}x");
    assert_eq!(heads_after(&text, 0, &[Move::Right, Move::Right]), [3, 4]);
    assert_eq!(heads_after(&text, 4, &[Move::Left, Move::Left]), [3, 0]);
}

#[test]
fn a_pasted_crlf_is_one_step() {
    let mut b = buf("");
    b.paste("a\r\nb");
    b.move_by(Move::Left, false);
    b.move_by(Move::Left, false);
    assert_eq!(span(&b), (1, 1));
}

#[test]
fn a_collapsed_left_or_right_with_a_selection_goes_to_its_edge() {
    let mut a = sel("abcd", 1, 3);
    a.move_by(Move::Left, false);
    assert_eq!(span(&a), (1, 1));
    let mut b = sel("abcd", 1, 3);
    b.move_by(Move::Right, false);
    assert_eq!(span(&b), (3, 3));
    let mut c = sel("abcd", 3, 1);
    c.move_by(Move::Left, false);
    assert_eq!(span(&c), (1, 1));
    let mut d = sel("abcd", 3, 1);
    d.move_by(Move::Right, false);
    assert_eq!(span(&d), (3, 3));
}

#[test]
fn shift_left_and_right_move_the_head_and_keep_the_anchor() {
    let mut b = at("abcd", 2);
    b.move_by(Move::Left, true);
    assert_eq!(span(&b), (2, 1));
    b.move_by(Move::Left, true);
    assert_eq!(span(&b), (2, 0));
    b.move_by(Move::Right, true);
    b.move_by(Move::Right, true);
    b.move_by(Move::Right, true);
    assert_eq!(span(&b), (2, 3));
}

#[test]
fn shift_right_selects_a_whole_cluster() {
    let mut b = at(FAMILY, 0);
    b.move_by(Move::Right, true);
    assert_eq!(span(&b), (0, 18));
    assert_eq!(b.selected_text(), FAMILY);
}

#[test]
fn shift_back_to_the_anchor_leaves_a_cursor() {
    let mut b = at("abcd", 2);
    b.move_by(Move::Left, true);
    b.move_by(Move::Right, true);
    assert_eq!(span(&b), (2, 2));
    assert_eq!(b.selected_text(), "");
}

// --- by word

#[test]
fn alt_left_goes_to_the_start_of_the_word() {
    assert_eq!(
        heads_after(
            "hello world",
            11,
            &[Move::WordLeft, Move::WordLeft, Move::WordLeft]
        ),
        [6, 0, 0]
    );
    assert_eq!(head_after("hello world", 8, Move::WordLeft), 6);
}

#[test]
fn alt_left_skips_punctuation_and_spaces() {
    assert_eq!(
        heads_after("hello, world", 12, &[Move::WordLeft, Move::WordLeft]),
        [7, 0]
    );
}

#[test]
fn alt_right_goes_to_the_end_of_the_word() {
    assert_eq!(
        heads_after(
            "hello world",
            0,
            &[Move::WordRight, Move::WordRight, Move::WordRight]
        ),
        [5, 11, 11]
    );
    assert_eq!(head_after("hello world", 7, Move::WordRight), 11);
}

#[test]
fn alt_right_skips_the_spaces_before_the_word() {
    assert_eq!(head_after("  hi", 0, Move::WordRight), 4);
}

#[test]
fn an_apostrophe_and_an_underscore_do_not_cut_a_word() {
    assert_eq!(
        heads_after("don't stop", 0, &[Move::WordRight, Move::WordRight]),
        [5, 10]
    );
    assert_eq!(
        heads_after("don't stop", 10, &[Move::WordLeft, Move::WordLeft]),
        [6, 0]
    );
    assert_eq!(
        heads_after("foo_bar 42", 0, &[Move::WordRight, Move::WordRight]),
        [7, 10]
    );
    assert_eq!(
        heads_after("foo_bar 42", 10, &[Move::WordLeft, Move::WordLeft]),
        [8, 0]
    );
}

#[test]
fn words_are_crossed_from_one_line_to_the_next() {
    assert_eq!(
        heads_after("one\ntwo", 0, &[Move::WordRight, Move::WordRight]),
        [3, 7]
    );
    assert_eq!(
        heads_after("one\ntwo", 7, &[Move::WordLeft, Move::WordLeft]),
        [4, 0]
    );
}

#[test]
fn a_word_with_a_combining_accent_is_one_word() {
    let text = format!("caf{E_ACUTE} au");
    assert_eq!(head_after(&text, 0, Move::WordRight), 6);
    assert_eq!(head_after(&text, 6, Move::WordLeft), 0);
}

#[test]
fn shift_alt_right_selects_the_word() {
    let mut b = at("hello world", 0);
    b.move_by(Move::WordRight, true);
    assert_eq!(span(&b), (0, 5));
    assert_eq!(b.selected_text(), "hello");
}

// --- by line and document

#[test]
fn cmd_left_goes_to_the_start_of_the_line() {
    let text = "ab\ncd\nef";
    assert_eq!(head_after(text, 4, Move::LineStart), 3);
    assert_eq!(head_after(text, 3, Move::LineStart), 3);
    assert_eq!(head_after(text, 5, Move::LineStart), 3);
    assert_eq!(head_after(text, 7, Move::LineStart), 6);
    assert_eq!(head_after(text, 1, Move::LineStart), 0);
}

#[test]
fn cmd_right_goes_to_the_end_of_the_line() {
    let text = "ab\ncd\nef";
    assert_eq!(head_after(text, 3, Move::LineEnd), 5);
    assert_eq!(head_after(text, 5, Move::LineEnd), 5);
    assert_eq!(head_after(text, 0, Move::LineEnd), 2);
    assert_eq!(head_after(text, 2, Move::LineEnd), 2);
    assert_eq!(head_after(text, 7, Move::LineEnd), 8);
}

#[test]
fn shift_cmd_left_selects_to_the_start_of_the_line() {
    let mut b = at("ab\ncd", 4);
    b.move_by(Move::LineStart, true);
    assert_eq!(span(&b), (4, 3));
    let mut c = at("ab\ncd", 3);
    c.move_by(Move::LineEnd, true);
    assert_eq!(span(&c), (3, 5));
}

#[test]
fn cmd_up_and_down_go_to_the_edges_of_the_text() {
    let text = "ab\ncd\nef";
    assert_eq!(head_after(text, 4, Move::DocStart), 0);
    assert_eq!(head_after(text, 4, Move::DocEnd), 8);
    assert_eq!(head_after(text, 0, Move::DocStart), 0);
    assert_eq!(head_after(text, 8, Move::DocEnd), 8);
}

#[test]
fn shift_cmd_down_selects_to_the_end_of_the_text() {
    let mut b = at("ab\ncd", 1);
    b.move_by(Move::DocEnd, true);
    assert_eq!(span(&b), (1, 5));
    let mut c = at("ab\ncd", 4);
    c.move_by(Move::DocStart, true);
    assert_eq!(span(&c), (4, 0));
}

// --- up and down keep the column

#[test]
fn up_and_down_keep_the_column() {
    assert_eq!(head_after("abc\ndef", 5, Move::Up), 1);
    assert_eq!(head_after("abc\ndef", 1, Move::Down), 5);
    assert_eq!(head_after("abc\ndef", 3, Move::Down), 7);
    assert_eq!(head_after("abc\ndef", 7, Move::Up), 3);
}

#[test]
fn up_on_the_first_line_goes_to_the_start_and_down_on_the_last_to_the_end() {
    assert_eq!(head_after("ab\ncd", 1, Move::Up), 0);
    assert_eq!(head_after("ab\ncd", 4, Move::Down), 5);
}

#[test]
fn a_short_line_clamps_the_column_and_the_next_long_one_gets_it_back() {
    let text = "abcdef\nab\nabcdef";
    assert_eq!(
        heads_after(text, 14, &[Move::Up, Move::Up, Move::Down, Move::Down]),
        [9, 4, 9, 14]
    );
}

#[test]
fn an_empty_line_in_between_keeps_the_column() {
    let text = "abc\n\ndef";
    assert_eq!(
        heads_after(text, 7, &[Move::Up, Move::Up, Move::Down, Move::Down]),
        [4, 2, 4, 7]
    );
}

#[test]
fn any_other_move_forgets_the_column() {
    let text = "abcdef\nab\nabcdef";
    assert_eq!(
        heads_after(text, 14, &[Move::Up, Move::Left, Move::Up]),
        [9, 8, 1]
    );
}

#[test]
fn a_click_forgets_the_column() {
    let mut b = at("abcdef\nab\nabcdef", 14);
    b.move_by(Move::Up, false);
    b.select(8, 8);
    b.move_by(Move::Up, false);
    assert_eq!(span(&b), (1, 1));
}

#[test]
fn columns_count_graphemes_not_bytes() {
    let text = format!("ab\n{FR}cd");
    assert_eq!(head_after(&text, 13, Move::Up), 2);
    assert_eq!(head_after(&text, 11, Move::Up), 1);
    assert_eq!(head_after(&text, 1, Move::Down), 11);
}

#[test]
fn shift_up_extends_the_selection_by_a_line() {
    let mut b = at("ab\ncd", 4);
    b.move_by(Move::Up, true);
    assert_eq!(span(&b), (4, 1));
    let mut c = at("ab\ncd", 1);
    c.move_by(Move::Down, true);
    assert_eq!(span(&c), (1, 4));
}

#[test]
fn up_from_a_selection_starts_at_its_head() {
    let mut b = sel("abc\ndef", 1, 5);
    b.move_by(Move::Up, false);
    assert_eq!(span(&b), (1, 1));
}
