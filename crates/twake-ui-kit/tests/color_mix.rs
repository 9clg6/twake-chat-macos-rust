//! Mixing two opaque colours, for the own bubble: 12 % of `primary.main` over `background.paper`
//! (`color-mix(in srgb, primary 12%, paper)` of `MessageBubble.tsx:205`). Public item:
//! `twake_ui_kit::color::mix(top, bottom, amount)` = `bottom + (top - bottom) * amount` per channel,
//! rounded to the nearest byte; the alpha of `bottom` is kept; `amount` is clamped to `0..=1`.

use twake_tokens::{DARK, LIGHT, Rgba8};
use twake_ui_kit::color::mix;

#[test]
fn twelve_percent_of_the_light_primary_over_white_is_the_pale_blue_of_own_bubbles() {
    let out = mix(LIGHT.primary.main, LIGHT.background.paper, 0.12);
    assert_eq!(out, Rgba8::new(226, 240, 255, 255));
}

#[test]
fn twelve_percent_of_the_dark_primary_over_the_dark_paper() {
    let out = mix(DARK.primary.main, DARK.background.paper, 0.12);
    assert_eq!(out, Rgba8::new(83, 93, 103, 255));
}

#[test]
fn zero_gives_the_bottom_and_one_gives_the_top() {
    let (top, bottom) = (Rgba8::new(10, 20, 30, 255), Rgba8::new(200, 210, 220, 255));
    assert_eq!(mix(top, bottom, 0.0), bottom);
    assert_eq!(mix(top, bottom, 1.0), top);
}

#[test]
fn the_middle_is_rounded_to_the_nearest_byte() {
    let out = mix(Rgba8::new(0, 0, 0, 255), Rgba8::new(255, 101, 1, 255), 0.5);
    assert_eq!(out, Rgba8::new(128, 51, 1, 255));
}

#[test]
fn an_amount_out_of_range_is_clamped() {
    let (top, bottom) = (Rgba8::new(10, 20, 30, 255), Rgba8::new(200, 210, 220, 255));
    assert_eq!(mix(top, bottom, 2.0), top);
    assert_eq!(mix(top, bottom, -1.0), bottom);
}
