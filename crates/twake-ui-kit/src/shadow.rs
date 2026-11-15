//! The CSS `box-shadow` text of the tokens (`SHADOWS_*`) as layers a renderer can draw.
//! Only the form makeShadows.js writes: `<x>px <y>px <blur>px [<spread>px] rgba(r, g, b, a)`, layers
//! separated by commas, or `none`; a layer in any other form is skipped.

use twake_tokens::Rgba8;

/// One layer of a shadow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowLayer {
    /// Horizontal offset in pixels.
    pub x_px: f32,
    /// Vertical offset in pixels.
    pub y_px: f32,
    /// Blur radius in pixels.
    pub blur_px: f32,
    /// Spread radius in pixels.
    pub spread_px: f32,
    /// Colour, alpha rounded to a byte.
    pub color: Rgba8,
}

/// The layers of a CSS `box-shadow` value, back to front as written; `none` has none.
pub fn parse_shadow(css: &str) -> Vec<ShadowLayer> {
    split_layers(css).into_iter().filter_map(layer).collect()
}

/// Splits on the commas outside parentheses.
fn split_layers(css: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0_i32, 0);
    for (i, c) in css.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&css[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&css[start..]);
    parts
}

fn layer(text: &str) -> Option<ShadowLayer> {
    let (lengths, color) = text.trim().split_once("rgba(")?;
    let lengths: Vec<f32> = lengths
        .split_whitespace()
        .map(|n| n.strip_suffix("px")?.parse().ok())
        .collect::<Option<_>>()?;
    // The spread is optional and 0 when absent.
    let (x_px, y_px, blur_px, spread_px) = match lengths[..] {
        [x, y, blur] => (x, y, blur, 0.0),
        [x, y, blur, spread] => (x, y, blur, spread),
        _ => return None,
    };
    let channels: Vec<f32> = color
        .strip_suffix(')')?
        .split(',')
        .map(|n| n.trim().parse().ok())
        .collect::<Option<_>>()?;
    let [r, g, b, a] = channels[..] else {
        return None;
    };
    // The clamp makes the casts exact.
    let byte = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    Some(ShadowLayer {
        x_px,
        y_px,
        blur_px,
        spread_px,
        color: Rgba8::new(byte(r), byte(g), byte(b), byte(a * 255.0)),
    })
}
