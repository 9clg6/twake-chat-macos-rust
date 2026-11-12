//! Style of a button for a variant, a colour, a size and a state, without a window.
//! Sources: docs/spec/design-system.md section 3 (Button), 1.7 (focus), 1.2 (typography `button`),
//! docs/spec/tokens.json `components.MuiButton`; decision A014 for the `action` colours.

use twake_tokens::{LineHeight, Rgba8, TYPE_BUTTON, Tone, TypeSpec};

use crate::theme::{ColorScheme, Theme};

/// How the button is filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Filled with the tone.
    Contained,
    /// Transparent with a border.
    Outlined,
    /// Transparent, no border.
    Text,
}

/// Which tone colours the button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonColor {
    /// The brand blue.
    Primary,
    /// The secondary tone.
    Secondary,
    /// The error tone.
    Error,
}

/// MUI sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    /// 24 px.
    XSmall,
    /// 36 px.
    Small,
    /// 40 px.
    Medium,
    /// 48 px.
    Large,
}

/// Interaction state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    /// Nothing.
    Rest,
    /// Pointer over.
    Hover,
    /// Pointer down (same look as hover, A014).
    Pressed,
    /// Reached by keyboard.
    FocusVisible,
    /// Not usable.
    Disabled,
}

/// A border.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    /// Width in pixels.
    pub width_px: f32,
    /// Colour.
    pub color: Rgba8,
}

/// The focus ring (design-system.md 1.7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusRing {
    /// Width in pixels.
    pub width_px: f32,
    /// Colour.
    pub color: Rgba8,
}

/// Everything a renderer needs to draw the button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedButton {
    /// Fill; alpha 0 when there is none.
    pub background: Rgba8,
    /// Text colour.
    pub foreground: Rgba8,
    /// Outlined only.
    pub border: Option<Border>,
    /// FocusVisible only.
    pub focus_ring: Option<FocusRing>,
    /// Corner radius.
    pub radius_px: f32,
    /// Horizontal padding.
    pub padding_x: f32,
    /// Vertical padding.
    pub padding_y: f32,
    /// MUI min-height.
    pub height_px: f32,
    /// Label style.
    pub font: TypeSpec,
    /// Always 1: a disabled button changes colour, it does not fade.
    pub opacity: f32,
}

// `action` colours of design-system.md lines 107-114 (A014): the ink of light, white of dark.
const INK: (u8, u8, u8) = (66, 66, 68);
const HOVER_ALPHA: (u8, u8) = (10, 20);
const DISABLED_ALPHA: (u8, u8) = (61, 82);
const DISABLED_BACKGROUND_ALPHA: u8 = 31;
const FOCUS_RING_PX: f32 = 2.0;
const RADIUS_PX: f32 = 100.0;
const XSMALL_FONT: TypeSpec = TypeSpec {
    font_size: 11.0,
    font_weight: 500,
    line_height: LineHeight::Px(16.0),
    letter_spacing: 0.5,
};

fn tone_of(color: ButtonColor, theme: &Theme) -> Tone {
    let scheme = theme.colors();
    match color {
        ButtonColor::Primary => scheme.primary,
        ButtonColor::Secondary => scheme.secondary,
        ButtonColor::Error => scheme.error,
    }
}

fn action_ink(theme: &Theme, alpha: u8) -> Rgba8 {
    match theme.scheme {
        ColorScheme::Light => Rgba8::new(INK.0, INK.1, INK.2, alpha),
        ColorScheme::Dark => Rgba8::new(255, 255, 255, alpha),
    }
}

fn pick(theme: &Theme, (light, dark): (u8, u8)) -> u8 {
    match theme.scheme {
        ColorScheme::Light => light,
        ColorScheme::Dark => dark,
    }
}

fn with_alpha(color: Rgba8, a: u8) -> Rgba8 {
    Rgba8::new(color.r, color.g, color.b, a)
}

const fn height_px(size: ButtonSize) -> f32 {
    match size {
        ButtonSize::XSmall => 24.0,
        ButtonSize::Small => 36.0,
        ButtonSize::Medium => 40.0,
        ButtonSize::Large => 48.0,
    }
}

/// `(x, y)` of tokens.json `MuiButton`; outlined small and large follow the size rule (sketch Q2).
const fn padding(variant: ButtonVariant, size: ButtonSize) -> (f32, f32) {
    use ButtonSize::{Large, Medium, Small, XSmall};
    use ButtonVariant::{Contained, Outlined, Text};
    match (variant, size) {
        (Outlined, XSmall) => (7.0, 3.0),
        (_, XSmall) => (8.0, 4.0),
        (Text, Small) => (16.0, 8.0),
        (_, Small) => (16.0, 9.0),
        (Contained, Medium) => (24.0, 10.0),
        (Outlined, Medium) => (15.0, 5.0),
        (Text, Medium) => (8.0, 10.0),
        (Text, Large) => (10.0, 14.0),
        (_, Large) => (32.0, 13.0),
    }
}

/// Colours of the state, without ring: `(background, foreground)`.
fn colors(
    variant: ButtonVariant,
    tone: Tone,
    hovered: bool,
    disabled: bool,
    theme: &Theme,
) -> (Rgba8, Rgba8) {
    let filled = variant == ButtonVariant::Contained;
    if disabled {
        let background = if filled {
            action_ink(theme, DISABLED_BACKGROUND_ALPHA)
        } else {
            with_alpha(tone.main, 0)
        };
        return (background, action_ink(theme, pick(theme, DISABLED_ALPHA)));
    }
    match (filled, hovered) {
        (true, false) => (tone.main, tone.contrast_text),
        (true, true) => (tone.dark, tone.contrast_text),
        (false, false) => (with_alpha(tone.main, 0), tone.main),
        (false, true) => (with_alpha(tone.main, pick(theme, HOVER_ALPHA)), tone.main),
    }
}

/// Resolves a button style.
pub struct ButtonStyle;

impl ButtonStyle {
    /// The style of this button in this theme.
    pub fn resolve(
        variant: ButtonVariant,
        color: ButtonColor,
        size: ButtonSize,
        state: ButtonState,
        theme: &Theme,
    ) -> ResolvedButton {
        let tone = tone_of(color, theme);
        let hovered = matches!(state, ButtonState::Hover | ButtonState::Pressed);
        let disabled = state == ButtonState::Disabled;
        let (background, foreground) = colors(variant, tone, hovered, disabled, theme);
        let (padding_x, padding_y) = padding(variant, size);
        ResolvedButton {
            background,
            foreground,
            border: (variant == ButtonVariant::Outlined).then_some(Border {
                width_px: 1.0,
                color: foreground,
            }),
            focus_ring: (state == ButtonState::FocusVisible).then_some(FocusRing {
                width_px: FOCUS_RING_PX,
                color: theme.colors().primary.main,
            }),
            radius_px: RADIUS_PX,
            padding_x,
            padding_y,
            height_px: height_px(size),
            font: if size == ButtonSize::XSmall {
                XSMALL_FONT
            } else {
                TYPE_BUTTON
            },
            opacity: 1.0,
        }
    }
}
