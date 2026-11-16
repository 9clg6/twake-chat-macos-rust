#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    dead_code
)]
//! Text buffer: content, insertion, paste, deletion by grapheme, word and line, `max_len`.
//!
//! Need (not the code): `docs/spec/features/composer.md` (D264 Shift+Enter is a newline, D332
//! paste is plain text only), `docs/dev/text-input-api-sketch.md` (conventions, sanitising,
//! words, lines). Offsets are bytes; "é" is 2 bytes, the family emoji 18, a flag 8.

mod tb;

use tb::{DE, E_ACUTE, FAMILY, FR, at, buf, deleted, sel, span};
use twake_ui_kit::text_buffer::{Delete, TextBuffer};

// --- content

#[test]
fn a_new_buffer_is_empty_with_the_cursor_at_zero() {
    let b = TextBuffer::new();
    assert_eq!(b.text(), "");
    assert_eq!(span(&b), (0, 0));
    assert_eq!(b.selected_text(), "");
    assert!(!b.can_undo());
}

#[test]
fn set_text_puts_the_cursor_at_the_end_in_bytes() {
    let b = buf("héllo");
    assert_eq!(b.text(), "héllo");
    assert_eq!(span(&b), (6, 6));
}

#[test]
fn set_text_cleans_what_it_is_given() {
    let b = buf("a\r\nb\u{7}c");
    assert_eq!(b.text(), "a\nbc");
}

// --- insertion

#[test]
fn insert_writes_at_the_cursor() {
    let mut b = at("ac", 1);
    b.insert("b");
    assert_eq!(b.text(), "abc");
    assert_eq!(span(&b), (2, 2));
}

#[test]
fn insert_replaces_the_selection() {
    let mut b = sel("abcd", 1, 3);
    b.insert("X");
    assert_eq!(b.text(), "aXd");
    assert_eq!(span(&b), (2, 2));
}

#[test]
fn a_selection_made_backwards_is_replaced_too() {
    let mut b = sel("abcd", 3, 1);
    b.insert("X");
    assert_eq!(b.text(), "aXd");
    assert_eq!(span(&b), (2, 2));
}

#[test]
fn the_cursor_moves_by_the_bytes_of_what_was_typed() {
    let mut b = at("ab", 1);
    b.insert("é");
    assert_eq!(b.text(), "aéb");
    assert_eq!(span(&b), (3, 3));
}

#[test]
fn a_newline_typed_is_one_byte() {
    let mut b = at("ab", 1);
    b.insert("\n");
    assert_eq!(b.text(), "a\nb");
    assert_eq!(span(&b), (2, 2));
}

#[test]
fn text_typed_before_a_combining_mark_leaves_the_cursor_after_the_whole_cluster() {
    let mut b = at("\u{301}", 0);
    b.insert("e");
    assert_eq!(b.text(), E_ACUTE);
    assert_eq!(span(&b), (3, 3));
}

#[test]
fn deleting_a_line_break_that_joins_a_mark_to_a_letter_leaves_the_cursor_after_the_cluster() {
    assert_eq!(
        deleted("a\n\u{301}", 2, Delete::Backward),
        ("a\u{301}".into(), 3)
    );
}

// --- paste and sanitising

#[test]
fn paste_turns_crlf_into_one_newline() {
    let mut b = TextBuffer::new();
    b.paste("a\r\nb");
    assert_eq!(b.text(), "a\nb");
    assert_eq!(span(&b), (3, 3));
}

#[test]
fn paste_turns_a_lone_carriage_return_into_a_newline() {
    let mut b = TextBuffer::new();
    b.paste("a\rb\r\r\nc");
    assert_eq!(b.text(), "a\nb\n\nc");
}

#[test]
fn paste_keeps_tabs_and_newlines() {
    let mut b = TextBuffer::new();
    b.paste("a\tb\nc");
    assert_eq!(b.text(), "a\tb\nc");
}

#[test]
fn control_characters_other_than_newline_and_tab_are_refused() {
    let mut b = TextBuffer::new();
    b.insert("a\u{0}b\u{7}c\u{1b}d\u{7f}e\u{85}f");
    assert_eq!(b.text(), "abcdef");
}

#[test]
fn the_zero_width_joiner_and_variation_selectors_are_kept() {
    let mut b = TextBuffer::new();
    b.insert(FAMILY);
    b.insert("\u{2764}\u{FE0F}");
    assert_eq!(b.text(), format!("{FAMILY}\u{2764}\u{FE0F}"));
}

#[test]
fn an_input_of_control_characters_only_changes_nothing() {
    let mut b = sel("abcd", 1, 3);
    b.insert("\u{7}\u{0}");
    assert_eq!(b.text(), "abcd");
    assert_eq!(span(&b), (1, 3));
    assert!(!b.can_undo());
}

#[test]
fn an_empty_insert_does_not_delete_the_selection() {
    let mut b = sel("abcd", 1, 3);
    b.insert("");
    assert_eq!(b.text(), "abcd");
    assert_eq!(span(&b), (1, 3));
}

#[test]
fn selected_text_is_what_the_selection_covers_whatever_its_direction() {
    assert_eq!(sel("abcd", 1, 3).selected_text(), "bc");
    assert_eq!(sel("abcd", 3, 1).selected_text(), "bc");
    assert_eq!(at("abcd", 2).selected_text(), "");
}

// --- delete by grapheme

#[test]
fn backspace_removes_one_char() {
    assert_eq!(deleted("abc", 3, Delete::Backward), ("ab".into(), 2));
    assert_eq!(deleted("abc", 2, Delete::Backward), ("ac".into(), 1));
}

#[test]
fn backspace_removes_a_letter_with_its_combining_accent() {
    let text = format!("a{E_ACUTE}");
    assert_eq!(deleted(&text, 4, Delete::Backward), ("a".into(), 1));
}

#[test]
fn backspace_removes_a_whole_zwj_emoji() {
    let text = format!("a{FAMILY}");
    assert_eq!(deleted(&text, 19, Delete::Backward), ("a".into(), 1));
}

#[test]
fn backspace_removes_a_flag_and_only_one() {
    let text = format!("a{FR}{DE}");
    assert_eq!(deleted(&text, 17, Delete::Backward), (format!("a{FR}"), 9));
}

#[test]
fn backspace_removes_one_newline() {
    assert_eq!(deleted("a\nb", 2, Delete::Backward), ("ab".into(), 1));
}

#[test]
fn delete_forward_removes_the_char_after_the_cursor() {
    assert_eq!(deleted("abc", 1, Delete::Forward), ("ac".into(), 1));
}

#[test]
fn delete_forward_removes_a_whole_zwj_emoji() {
    let text = format!("{FAMILY}b");
    assert_eq!(deleted(&text, 0, Delete::Forward), ("b".into(), 0));
}

#[test]
fn deleting_at_an_edge_does_nothing() {
    let mut b = at("ab", 0);
    b.delete(Delete::Backward);
    b.delete(Delete::WordBackward);
    b.delete(Delete::ToLineStart);
    let mut e = buf("ab");
    e.delete(Delete::Forward);
    e.delete(Delete::WordForward);
    assert_eq!((b.text(), span(&b)), ("ab", (0, 0)));
    assert_eq!((e.text(), span(&e)), ("ab", (2, 2)));
    assert!(!b.can_undo() && !e.can_undo());
}

#[test]
fn every_kind_of_deletion_removes_only_a_selection_when_there_is_one() {
    let kinds = [
        Delete::Backward,
        Delete::Forward,
        Delete::WordBackward,
        Delete::WordForward,
        Delete::ToLineStart,
    ];
    for kind in kinds {
        let mut b = sel("abcd efg", 1, 3);
        b.delete(kind);
        assert_eq!(b.text(), "ad efg", "{kind:?}");
        assert_eq!(span(&b), (1, 1), "{kind:?}");
    }
}

// --- delete by word

#[test]
fn alt_backspace_removes_the_word_before_the_cursor() {
    assert_eq!(
        deleted("hello world", 11, Delete::WordBackward),
        ("hello ".into(), 6)
    );
    assert_eq!(
        deleted("hello ", 6, Delete::WordBackward),
        (String::new(), 0)
    );
}

#[test]
fn alt_backspace_takes_the_separators_with_it() {
    assert_eq!(
        deleted("hello, world", 12, Delete::WordBackward),
        ("hello, ".into(), 7)
    );
    assert_eq!(
        deleted("hello, ", 7, Delete::WordBackward),
        (String::new(), 0)
    );
}

#[test]
fn an_apostrophe_does_not_cut_a_word() {
    assert_eq!(
        deleted("say don't", 9, Delete::WordBackward),
        ("say ".into(), 4)
    );
}

#[test]
fn a_word_with_a_combining_accent_goes_in_one_piece() {
    let text = format!("caf{E_ACUTE} au");
    assert_eq!(
        deleted(&text, 9, Delete::WordBackward),
        (format!("caf{E_ACUTE} "), 7)
    );
    assert_eq!(
        deleted(&format!("caf{E_ACUTE} "), 7, Delete::WordBackward),
        (String::new(), 0)
    );
}

#[test]
fn alt_backspace_goes_back_over_a_line_break() {
    assert_eq!(
        deleted("one\ntwo", 7, Delete::WordBackward),
        ("one\n".into(), 4)
    );
    assert_eq!(
        deleted("one\n", 4, Delete::WordBackward),
        (String::new(), 0)
    );
}

#[test]
fn alt_delete_removes_the_word_after_the_cursor() {
    assert_eq!(
        deleted("hello world", 0, Delete::WordForward),
        (" world".into(), 0)
    );
    assert_eq!(
        deleted(" world", 0, Delete::WordForward),
        (String::new(), 0)
    );
}

#[test]
fn alt_delete_from_inside_a_word_removes_its_end() {
    assert_eq!(
        deleted("hello world", 2, Delete::WordForward),
        ("he world".into(), 2)
    );
}

// --- delete to the start of the line

#[test]
fn cmd_backspace_removes_up_to_the_start_of_the_line() {
    assert_eq!(
        deleted("ab\ncdef", 5, Delete::ToLineStart),
        ("ab\nef".into(), 3)
    );
    assert_eq!(
        deleted("ab\ncd", 5, Delete::ToLineStart),
        ("ab\n".into(), 3)
    );
}

#[test]
fn cmd_backspace_on_the_first_line_removes_up_to_the_start_of_the_text() {
    assert_eq!(deleted("abcd", 2, Delete::ToLineStart), ("cd".into(), 0));
}

#[test]
fn cmd_backspace_at_the_start_of_a_line_removes_the_line_break_before() {
    assert_eq!(
        deleted("ab\ncd", 3, Delete::ToLineStart),
        ("abcd".into(), 2)
    );
}

// --- max_len (characters = Unicode scalar values)

#[test]
fn a_new_buffer_has_no_limit() {
    let mut b = TextBuffer::new();
    b.insert(&"a".repeat(1000));
    assert_eq!(b.text().len(), 1000);
}

#[test]
fn what_does_not_fit_is_cut() {
    let mut b = TextBuffer::with_max_len(5);
    b.insert("abc");
    b.insert("defgh");
    assert_eq!(b.text(), "abcde");
}

#[test]
fn the_limit_counts_chars_not_bytes() {
    let mut b = TextBuffer::with_max_len(3);
    b.insert("éé\u{1F600}é");
    assert_eq!(b.text(), "éé\u{1F600}");
}

#[test]
fn a_cluster_that_does_not_fit_is_dropped_whole() {
    let mut b = TextBuffer::with_max_len(4);
    b.insert(&format!("ab{FAMILY}"));
    assert_eq!(b.text(), "ab");
    let mut c = TextBuffer::with_max_len(2);
    c.insert("ae\u{301}x");
    assert_eq!(c.text(), "a");
}

#[test]
fn a_cluster_that_fits_exactly_is_kept() {
    let mut b = TextBuffer::with_max_len(7);
    b.insert(&format!("ab{FAMILY}z"));
    assert_eq!(b.text(), format!("ab{FAMILY}"));
    let mut c = TextBuffer::with_max_len(3);
    c.insert("ae\u{301}x");
    assert_eq!(c.text(), format!("a{E_ACUTE}"));
}

#[test]
fn the_selection_that_is_replaced_frees_room() {
    let mut b = TextBuffer::with_max_len(5);
    b.insert("abcde");
    b.select(0, 5);
    b.insert("vwxyz!");
    assert_eq!(b.text(), "vwxyz");
}

#[test]
fn a_full_buffer_refuses_typing_without_an_undo_step() {
    let mut b = TextBuffer::with_max_len(3);
    b.set_text("abc");
    b.insert("d");
    assert_eq!(b.text(), "abc");
    assert!(!b.can_undo());
}

#[test]
fn paste_and_set_text_obey_the_limit_too() {
    let mut b = TextBuffer::with_max_len(4);
    b.set_text("ab");
    b.paste("cdefg");
    assert_eq!(b.text(), "abcd");
    b.set_text("123456");
    assert_eq!(b.text(), "1234");
}
