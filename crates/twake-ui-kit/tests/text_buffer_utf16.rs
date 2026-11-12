#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    dead_code
)]
//! Text buffer: conversion between byte offsets and UTF-16 code units, which is what
//! `NSRange` and `NSTextInputClient` speak.
//!
//! Need (not the code): `docs/dev/text-input-api-sketch.md` (UTF-16). Text used:
//! "é😀x" is bytes 0..2 / 2..6 / 6..7 and UTF-16 units 0..1 / 1..3 / 3..4.

mod tb;

use tb::buf;

const TEXT: &str = "é\u{1F600}x";

#[test]
fn utf8_offsets_become_utf16_offsets() {
    let b = buf(TEXT);
    let got: Vec<usize> = [0, 2, 6, 7].iter().map(|o| b.utf8_to_utf16(*o)).collect();
    assert_eq!(got, [0, 1, 3, 4]);
}

#[test]
fn utf16_offsets_become_utf8_offsets() {
    let b = buf(TEXT);
    let got: Vec<usize> = [0, 1, 3, 4].iter().map(|o| b.utf16_to_utf8(*o)).collect();
    assert_eq!(got, [0, 2, 6, 7]);
}

#[test]
fn ascii_is_the_same_in_both() {
    let b = buf("hello");
    assert_eq!(b.utf8_to_utf16(3), 3);
    assert_eq!(b.utf16_to_utf8(3), 3);
}

#[test]
fn a_three_byte_char_is_one_unit() {
    let b = buf("日本");
    assert_eq!(b.utf8_to_utf16(3), 1);
    assert_eq!(b.utf8_to_utf16(6), 2);
    assert_eq!(b.utf16_to_utf8(1), 3);
}

#[test]
fn the_middle_of_a_surrogate_pair_goes_to_the_start_of_the_char() {
    let b = buf(TEXT);
    assert_eq!(b.utf16_to_utf8(2), 2);
}

#[test]
fn a_byte_offset_inside_a_char_goes_to_the_start_of_the_char() {
    let b = buf(TEXT);
    assert_eq!(b.utf8_to_utf16(3), 1);
    assert_eq!(b.utf8_to_utf16(1), 0);
}

#[test]
fn offsets_past_the_end_go_to_the_end() {
    let b = buf(TEXT);
    assert_eq!(b.utf8_to_utf16(99), 4);
    assert_eq!(b.utf16_to_utf8(99), 7);
}

#[test]
fn ranges_convert_both_ends() {
    let b = buf(TEXT);
    assert_eq!(b.utf16_range_to_utf8(1..3), 2..6);
    assert_eq!(b.utf8_range_to_utf16(2..7), 1..4);
    assert_eq!(b.utf16_range_to_utf8(0..0), 0..0);
    assert_eq!(b.utf8_range_to_utf16(6..6), 3..3);
}

#[test]
fn a_range_with_an_end_past_the_text_is_clamped() {
    let b = buf(TEXT);
    assert_eq!(b.utf16_range_to_utf8(3..50), 6..7);
    assert_eq!(b.utf8_range_to_utf16(0..50), 0..4);
}

#[test]
fn a_range_from_the_os_can_select_a_cluster() {
    let mut b = buf("a\u{1F468}\u{200D}\u{1F469}b");
    let r = b.utf16_range_to_utf8(1..6);
    b.select(r.start, r.end);
    assert_eq!(b.selected_text(), "\u{1F468}\u{200D}\u{1F469}");
}

#[test]
fn the_marked_range_can_be_given_to_the_os_in_utf16() {
    let mut b = buf("é");
    b.set_marked_text(None, "\u{1F600}");
    let marked = b.marked_range().unwrap();
    assert_eq!(b.utf8_range_to_utf16(marked), 1..3);
}
