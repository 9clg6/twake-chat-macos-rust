//! Design tokens (colours, type, radii, breakpoints, shadows, transitions, avatar gradients); GPUI-free and dependency-free.
//! The constants come from `docs/spec/tokens.json` through `cargo xtask tokens` (`generated.rs`).

use std::fmt;

// The generator writes long lines (shadows, gradients): rustfmt must not rewrite it, `--check` compares bytes.
#[rustfmt::skip]
mod generated;

pub use generated::*;

/// A colour as four bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba8 {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha (255 is opaque).
    pub a: u8,
}

/// The text is not `#rgb`, `#rrggbb` or `#rrggbbaa`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseColorError;

impl fmt::Display for ParseColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected a colour as #rgb, #rrggbb or #rrggbbaa")
    }
}

impl std::error::Error for ParseColorError {}

const fn digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// `(short, channel count)` of `#rgb`, `#rrggbb` or `#rrggbbaa`.
const fn layout(bytes: &[u8]) -> Option<(bool, usize)> {
    if bytes.is_empty() || bytes[0] != b'#' {
        return None;
    }
    match bytes.len() {
        4 => Some((true, 3)),
        7 => Some((false, 3)),
        9 => Some((false, 4)),
        _ => None,
    }
}

/// Channel `i` of `#rgb` (`short`) or of `#rrggbb[aa]`.
const fn channel(bytes: &[u8], i: usize, short: bool) -> Option<u8> {
    if short {
        match digit(bytes[1 + i]) {
            Some(d) => Some(d * 17),
            None => None,
        }
    } else {
        match (digit(bytes[1 + 2 * i]), digit(bytes[2 + 2 * i])) {
            (Some(high), Some(low)) => Some(high * 16 + low),
            _ => None,
        }
    }
}

impl Rgba8 {
    /// A colour from its four bytes.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// An opaque colour from `0xRRGGBB`.
    pub const fn rgb(packed: u32) -> Self {
        Self::new((packed >> 16) as u8, (packed >> 8) as u8, packed as u8, 255)
    }

    /// Parses `#rgb` (each digit doubled), `#rrggbb` or `#rrggbbaa`, in either case.
    pub const fn from_hex(text: &str) -> Result<Self, ParseColorError> {
        let bytes = text.as_bytes();
        let Some((short, count)) = layout(bytes) else {
            return Err(ParseColorError);
        };
        let mut out = [255u8; 4];
        let mut i = 0;
        while i < count {
            out[i] = match channel(bytes, i, short) {
                Some(value) => value,
                None => return Err(ParseColorError),
            };
            i += 1;
        }
        Ok(Self::new(out[0], out[1], out[2], out[3]))
    }
}

/// Ten shades (50, 100 ... 900) and the text colour that reads on them, when the theme has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// Shades 50, 100, 200 ... 900, in that order.
    pub shades: [Rgba8; 10],
    /// Text colour for use on the shades.
    pub contrast_text: Option<Rgba8>,
}

impl Palette {
    /// The shade `n` (50, 100 ... 900); `None` for any other number.
    pub const fn shade(&self, n: u16) -> Option<Rgba8> {
        match n {
            50 => Some(self.shades[0]),
            100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900 => {
                Some(self.shades[(n / 100) as usize])
            }
            _ => None,
        }
    }
}

/// One semantic colour in four values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tone {
    /// Lighter variant.
    pub light: Rgba8,
    /// Main colour.
    pub main: Rgba8,
    /// Darker variant.
    pub dark: Rgba8,
    /// Text colour on `main`.
    pub contrast_text: Rgba8,
}

/// Text colours of a scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextColors {
    /// Body text.
    pub primary: Rgba8,
    /// Secondary text.
    pub secondary: Rgba8,
    /// Disabled text.
    pub disabled: Rgba8,
    /// Icons.
    pub icon: Rgba8,
}

/// Background colours of a scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundColors {
    /// Window background.
    pub default: Rgba8,
    /// Surfaces (cards, panes).
    pub paper: Rgba8,
    /// Subtle overlay.
    pub contrast: Rgba8,
}

/// Border colours of a scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderColors {
    /// Default border.
    pub main: Rgba8,
    /// Disabled border.
    pub disabled: Rgba8,
    /// Ghost border.
    pub ghost: Rgba8,
    /// Disabled ghost border.
    pub ghost_disabled: Rgba8,
}

/// Colours of interaction states, as ink over a surface (alpha included).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionColors {
    /// Active icons.
    pub active: Rgba8,
    /// Hovered surface.
    pub hover: Rgba8,
    /// Selected surface.
    pub selected: Rgba8,
    /// Disabled content.
    pub disabled: Rgba8,
    /// Disabled surface.
    pub disabled_background: Rgba8,
    /// Focused surface.
    pub focus: Rgba8,
}

/// The colours of one scheme (light or dark).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scheme {
    /// Primary tone.
    pub primary: Tone,
    /// Secondary tone.
    pub secondary: Tone,
    /// Error tone.
    pub error: Tone,
    /// Warning tone.
    pub warning: Tone,
    /// Success tone.
    pub success: Tone,
    /// Info tone.
    pub info: Tone,
    /// Text colours.
    pub text: TextColors,
    /// Background colours.
    pub background: BackgroundColors,
    /// Border colours.
    pub border: BorderColors,
    /// Interaction states.
    pub action: ActionColors,
    /// Divider line.
    pub divider: Rgba8,
}

/// A colour of a gradient and where it sits on the line, in percent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientStop {
    /// The colour.
    pub color: Rgba8,
    /// Position on the gradient line, in percent.
    pub at_percent: f32,
}

/// A CSS `linear-gradient(<angle>deg, from, to)`: the angle is clockwise from "up".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gradient {
    /// CSS angle in degrees.
    pub angle_deg: f32,
    /// First stop.
    pub from: GradientStop,
    /// Last stop.
    pub to: GradientStop,
}

/// A line height: a multiple of the font size, or pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineHeight {
    /// Multiple of the font size.
    Ratio(f32),
    /// Pixels.
    Px(f32),
}

/// One text style.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeSpec {
    /// Size in pixels.
    pub font_size: f32,
    /// Weight (400 regular, 600 semibold ...).
    pub font_weight: u16,
    /// Line height.
    pub line_height: LineHeight,
    /// Letter spacing in pixels.
    pub letter_spacing: f32,
}
