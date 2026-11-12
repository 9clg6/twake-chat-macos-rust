//! Colour glue between `twake-tokens` bytes and GPUI floats.
//! Need: docs/dev/ui-kit-api-sketch.md section 4 (module `color` of twake-ui-kit); A010 (every GPUI type goes through `twake-gpui`).

use twake_gpui::Rgba;
use twake_tokens::Rgba8;
use twake_ui_kit::color::{from_gpui_rgba, to_gpui_rgba};

fn close(left: f32, right: f32) -> bool {
    (left - right).abs() < 1e-6
}

#[test]
fn bytes_become_floats_channel_by_channel() {
    let out = to_gpui_rgba(Rgba8::new(51, 102, 153, 204));
    assert!(close(out.r, 0.2), "r = {}", out.r);
    assert!(close(out.g, 0.4), "g = {}", out.g);
    assert!(close(out.b, 0.6), "b = {}", out.b);
    assert!(close(out.a, 0.8), "a = {}", out.a);
}

#[test]
fn black_transparent_and_white_opaque_are_the_ends_of_the_range() {
    let clear = to_gpui_rgba(Rgba8::new(0, 0, 0, 0));
    assert_eq!((clear.r, clear.g, clear.b, clear.a), (0.0, 0.0, 0.0, 0.0));
    let white = to_gpui_rgba(Rgba8::new(255, 255, 255, 255));
    assert_eq!((white.r, white.g, white.b, white.a), (1.0, 1.0, 1.0, 1.0));
}

#[test]
fn the_alpha_channel_is_not_mixed_with_the_others() {
    let out = to_gpui_rgba(Rgba8::new(255, 0, 0, 51));
    assert!(close(out.r, 1.0) && close(out.g, 0.0) && close(out.b, 0.0));
    assert!(close(out.a, 0.2), "a = {}", out.a);
}

#[test]
fn floats_become_rounded_bytes() {
    let out = from_gpui_rgba(Rgba {
        r: 1.0,
        g: 0.0,
        b: 0.5,
        a: 0.2,
    });
    assert_eq!(out, Rgba8::new(255, 0, 128, 51));
}

#[test]
fn floats_out_of_range_are_clamped() {
    let out = from_gpui_rgba(Rgba {
        r: 1.5,
        g: -0.5,
        b: 2.0,
        a: -1.0,
    });
    assert_eq!(out, Rgba8::new(255, 0, 255, 0));
}

#[test]
fn every_byte_value_survives_the_round_trip() {
    for value in 0..=255u8 {
        let color = Rgba8::new(value, 255 - value, value / 2, value / 3);
        assert_eq!(from_gpui_rgba(to_gpui_rgba(color)), color, "byte {value}");
    }
}
