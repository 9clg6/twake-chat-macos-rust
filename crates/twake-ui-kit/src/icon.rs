//! The SVG an icon draws at a size, in a colour.

use twake_icons::Icon;
use twake_tokens::Rgba8;

/// The colour of an icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconColor {
    /// The colour of the host (`currentColor`).
    Current,
    /// A fixed colour.
    Fixed(Rgba8),
}

/// An icon, its box size and its colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconSpec {
    /// The icon.
    pub icon: &'static Icon,
    /// Side of the square box, in pixels.
    pub size_px: f32,
    /// Its colour.
    pub color: IconColor,
}

fn hex(color: Rgba8) -> String {
    if color.a == 255 {
        format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
    } else {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            color.r, color.g, color.b, color.a
        )
    }
}

/// The SVG document of the spec. A body with hard-coded colours is never recoloured (A013).
pub fn svg_for(spec: &IconSpec) -> String {
    let size = spec.size_px;
    let mut svg = spec.icon.to_svg();
    if let (IconColor::Fixed(color), false) = (spec.color, spec.icon.body.contains("fill=\"#")) {
        svg = svg.replacen(
            "fill=\"currentColor\"",
            &format!("fill=\"{}\"", hex(color)),
            1,
        );
    }
    svg.replacen(
        "<svg ",
        &format!("<svg width=\"{size}\" height=\"{size}\" "),
        1,
    )
}
