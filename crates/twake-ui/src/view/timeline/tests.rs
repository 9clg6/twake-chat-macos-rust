//! Behaviour of the pure helpers of the timeline: how many lines a text takes, the text of the
//! ticks, and the height a row is given before it is drawn (need: lot 1 of
//! `2026-11-13-design-match.md`; a short message is one line, narrower means taller).

use twake_domain::EventId;
use twake_ui_kit::theme::{ColorScheme, Theme};

use super::{estimated_height, tick_text, wrap_lines};
use crate::avatar::AvatarView;
use crate::timeline_rows::{BubbleView, DayDividerView, QuoteView, RowView, Side, Ticks};

fn bubble(side: Side, body: &str) -> BubbleView {
    BubbleView {
        side,
        author: "Ana".into(),
        avatar: AvatarView::of("Ana", "@ana:x"),
        shows_name: false,
        shows_avatar: false,
        body: body.into(),
        time: "10:00".into(),
        ticks: None,
        edited: None,
        quote: None,
    }
}

fn height(b: BubbleView, column_px: f32) -> f32 {
    estimated_height(
        &RowView::Bubble(b),
        Theme::new(ColorScheme::Light),
        column_px,
    )
}

fn words(count: usize) -> String {
    vec!["word"; count].join(" ")
}

#[test]
fn an_empty_text_still_takes_one_line() {
    assert_eq!(wrap_lines("", 20), 1);
    assert_eq!(wrap_lines("   ", 20), 1);
}

#[test]
fn a_short_text_is_one_line() {
    assert_eq!(wrap_lines("hello world", 20), 1);
}

#[test]
fn a_text_exactly_as_wide_as_the_line_is_one_line_and_one_more_character_makes_two() {
    assert_eq!(wrap_lines("aaaa bbbb", 9), 1);
    assert_eq!(wrap_lines("aaaa bbbb", 8), 2);
}

#[test]
fn a_long_text_wraps_between_words() {
    // 10 words of 4 letters, 9 characters per line: one word and a half does not fit, so 2 a line.
    assert_eq!(wrap_lines(&words(10), 9), 5);
}

#[test]
fn a_word_longer_than_the_line_breaks_over_several_lines() {
    assert_eq!(wrap_lines(&"x".repeat(25), 10), 3);
    assert_eq!(wrap_lines(&"x".repeat(10), 10), 1);
}

#[test]
fn every_newline_starts_a_line_even_an_empty_one() {
    assert_eq!(wrap_lines("a\nb\nc", 20), 3);
    assert_eq!(wrap_lines("a\n\nb", 20), 3);
}

#[test]
fn a_long_word_after_text_starts_on_its_own_line() {
    assert_eq!(wrap_lines(&format!("hi {}", "x".repeat(15)), 10), 3);
}

#[test]
fn multibyte_characters_count_once() {
    assert_eq!(wrap_lines(&"é".repeat(10), 10), 1);
}

#[test]
fn the_ticks_have_a_text_each() {
    assert_eq!(tick_text(Ticks::Clock), "...");
    assert_eq!(tick_text(Ticks::Sent), "\u{2713}");
    assert_eq!(tick_text(Ticks::Read), "\u{2713}\u{2713}");
    assert_eq!(tick_text(Ticks::Failed), "!");
}

#[test]
fn a_day_divider_has_a_fixed_height() {
    let row = RowView::Day(DayDividerView {
        label: "Today".into(),
    });
    let theme = Theme::new(ColorScheme::Light);
    assert_eq!(estimated_height(&row, theme, 800.0), 40.0);
    assert_eq!(estimated_height(&row, theme, 300.0), 40.0);
}

#[test]
fn a_one_word_message_is_one_line_high() {
    let one = height(bubble(Side::Own, "hi"), 800.0);
    let two = height(bubble(Side::Own, "hi\nthere"), 800.0);
    assert_eq!(two - one, 20.0);
    // gap 4 + padding 2 x 8 + line 20 + footer 16
    assert_eq!(one, 56.0);
}

#[test]
fn an_empty_message_is_as_high_as_a_one_line_message() {
    assert_eq!(
        height(bubble(Side::Own, ""), 800.0),
        height(bubble(Side::Own, "hi"), 800.0)
    );
}

#[test]
fn a_long_message_is_taller_than_a_short_one() {
    assert!(height(bubble(Side::Own, &words(200)), 800.0) > height(bubble(Side::Own, "hi"), 800.0));
}

#[test]
fn the_same_text_is_taller_in_a_narrower_column() {
    let text = words(60);
    let wide = height(bubble(Side::Other, &text), 800.0);
    let medium = height(bubble(Side::Other, &text), 500.0);
    let narrow = height(bubble(Side::Other, &text), 300.0);
    assert!(wide < medium && medium < narrow, "{wide} {medium} {narrow}");
}

#[test]
fn the_name_above_a_bubble_adds_to_its_height() {
    let mut named = bubble(Side::Other, "hi");
    named.shows_name = true;
    assert_eq!(
        height(named, 800.0) - height(bubble(Side::Other, "hi"), 800.0),
        18.0
    );
}

#[test]
fn a_quote_in_a_bubble_adds_to_its_height() {
    let mut quoting = bubble(Side::Other, "hi");
    quoting.quote = Some(QuoteView {
        target: EventId::parse("$a").expect("valid event id"),
        author: "Bob".into(),
        text: "earlier".into(),
    });
    assert_eq!(
        height(quoting, 800.0) - height(bubble(Side::Other, "hi"), 800.0),
        40.0
    );
}
