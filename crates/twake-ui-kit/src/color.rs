//! Colour glue between `twake-tokens` bytes and GPUI floats.

use twake_gpui::Rgba;
use twake_tokens::Rgba8;

/// Bytes to floats in `0..=1`.
pub fn to_gpui_rgba(color: Rgba8) -> Rgba {
    let unit = |byte: u8| f32::from(byte) / 255.0;
    Rgba {
        r: unit(color.r),
        g: unit(color.g),
        b: unit(color.b),
        a: unit(color.a),
    }
}

/// Floats to rounded bytes, clamped to `0..=255`.
pub fn from_gpui_rgba(color: Rgba) -> Rgba8 {
    // The clamp makes the cast exact.
    let byte = |unit: f32| (unit * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgba8::new(byte(color.r), byte(color.g), byte(color.b), byte(color.a))
}
