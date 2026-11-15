//! Style of a button for a variant, a colour, a size and a state, without a window.
//! Sources: docs/spec/design-system.md section 3 (Button), 1.7 (focus), 1.2 (typography `button`),
//! docs/spec/tokens.json `components.MuiButton`; decision A014 for the `action` colours.

use twake_tokens::{LineHeight, Rgba8, TYPE_BUTTON, Tone, TypeSpec};

use crate::color::{tint, transparent, with_alpha};
use crate::theme::Theme;

/// How the button is filled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Filled with the tone.
    Contained,
    /// Transparent with a border.
    Outlined,
    /// Transparent, no border.
    Text,
    /// Tinted fill of the tone, no border (twake-mui `ghost`).
    Ghost,
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
    /// The success tone.
    Success,
    /// The warning tone.
    Warning,
    /// The info tone.
    Info,
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

/// Fill of a primary ghost button at rest and hovered, `overrides.js:187-198`.
const PRIMARY_GHOST: (f32, f32) = (0.25, 0.32);
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
        ButtonColor::Success => scheme.success,
        ButtonColor::Warning => scheme.warning,
        ButtonColor::Info => scheme.info,
    }
}

/// Per-size values, in the order XSmall, Small, Medium, Large.
const fn by_size<T: Copy>(size: ButtonSize, table: [T; 4]) -> T {
    table[size as usize]
}

const fn height_px(size: ButtonSize) -> f32 {
    by_size(size, [24.0, 36.0, 40.0, 48.0])
}

/// `(x, y)` of tokens.json `MuiButton`; outlined small and large follow the size rule (sketch Q2).
const fn padding(variant: ButtonVariant, size: ButtonSize) -> (f32, f32) {
    match variant {
        ButtonVariant::Contained => {
            by_size(size, [(8.0, 4.0), (16.0, 9.0), (24.0, 10.0), (32.0, 13.0)])
        }
        ButtonVariant::Outlined => {
            by_size(size, [(7.0, 3.0), (16.0, 9.0), (15.0, 5.0), (32.0, 13.0)])
        }
        ButtonVariant::Ghost => by_size(size, [(8.0, 4.0), (16.0, 9.0), (15.0, 5.0), (32.0, 13.0)]),
        ButtonVariant::Text => by_size(size, [(8.0, 4.0), (16.0, 8.0), (8.0, 10.0), (10.0, 14.0)]),
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
            theme.colors().action.disabled_background
        } else {
            transparent(tone.main)
        };
        return (background, theme.colors().action.disabled);
    }
    match (filled, hovered) {
        (true, false) => (tone.main, tone.contrast_text),
        (true, true) => (tone.dark, tone.contrast_text),
        (false, false) => (transparent(tone.main), tone.main),
        (false, true) => (
            with_alpha(tone.main, theme.colors().action.hover.a),
            tone.main,
        ),
    }
}

/// Colours of a ghost button: primary is written in `text.primary` on a stronger tint, the other
/// tones in their main on `ghostOpacity`; disabled is a bare outline (`overrides.js:187-214`).
fn ghost_colors(
    color: ButtonColor,
    tone: Tone,
    hovered: bool,
    disabled: bool,
    theme: &Theme,
) -> (Rgba8, Rgba8) {
    let scheme = theme.colors();
    if disabled {
        return (transparent(tone.main), scheme.action.disabled);
    }
    let (rest, hover) = if color == ButtonColor::Primary {
        PRIMARY_GHOST
    } else {
        (theme.opacities().ghost, theme.opacities().hover_ghost)
    };
    let ink = if color == ButtonColor::Primary {
        scheme.text.primary
    } else {
        tone.main
    };
    (tint(tone.main, if hovered { hover } else { rest }), ink)
}

fn border(
    variant: ButtonVariant,
    foreground: Rgba8,
    disabled: bool,
    theme: &Theme,
) -> Option<Border> {
    let color = match variant {
        ButtonVariant::Outlined => foreground,
        ButtonVariant::Ghost if disabled => theme.colors().action.disabled_background,
        _ => return None,
    };
    Some(Border {
        width_px: 1.0,
        color,
    })
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
        let (background, foreground) = if variant == ButtonVariant::Ghost {
            ghost_colors(color, tone, hovered, disabled, theme)
        } else {
            colors(variant, tone, hovered, disabled, theme)
        };
        let (padding_x, padding_y) = padding(variant, size);
        ResolvedButton {
            background,
            foreground,
            border: border(variant, foreground, disabled, theme),
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
