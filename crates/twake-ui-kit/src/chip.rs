//! Style of a chip (`Chip` of twake-mui), without a window. Sources: MUI `Chip/Chip.js` (root 74-100,
//! delete icon 117-123, small 146-167, colours 170-185, clickable 220-245, outlined 255-318, label
//! 327-356), twake-mui `overrides.js:613-621` (`.square` = radius md). With `focusVisible: true` (the
//! app's theme) MUI drops the focus fill: keyboard focus is the 2 px primary ring alone.

use twake_tokens::{GREY, RADIUS_MD, Rgba8, Tone};

use crate::button::{Border, FocusRing};
use crate::color::{tint, transparent, with_alpha};
use crate::theme::{ColorScheme, Theme};

const FOCUS_RING_PX: f32 = 2.0;

/// Filled or drawn with a hairline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChipVariant {
    /// Filled.
    #[default]
    Filled,
    /// Transparent with a border.
    Outlined,
}

/// Which colour the chip takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChipColor {
    /// Neutral: `action.selected` and `text.primary`.
    #[default]
    Default,
    /// The brand blue.
    Primary,
    /// The secondary tone.
    Secondary,
    /// The error tone.
    Error,
    /// The warning tone.
    Warning,
    /// The success tone.
    Success,
    /// The info tone.
    Info,
}

/// Height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChipSize {
    /// 24 px.
    Small,
    /// 32 px.
    #[default]
    Medium,
}

/// Interaction state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipState {
    /// Nothing.
    Rest,
    /// Pointer over (changes a clickable chip only).
    Hover,
    /// Reached by keyboard.
    FocusVisible,
    /// Not usable.
    Disabled,
}

/// What the chip is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChipProps {
    /// Filled or outlined.
    pub variant: ChipVariant,
    /// Colour.
    pub color: ChipColor,
    /// Height.
    pub size: ChipSize,
    /// Reacts to the pointer.
    pub clickable: bool,
    /// Has a cross to remove it.
    pub deletable: bool,
    /// Corners of 8 px instead of a pill.
    pub square: bool,
}

/// The cross of a deletable chip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeleteIcon {
    /// Side of the icon.
    pub size_px: f32,
    /// Colour at rest.
    pub color: Rgba8,
    /// Colour under the pointer.
    pub hover_color: Rgba8,
    /// Margin before it (negative pulls it into the label's padding).
    pub margin_left_px: f32,
    /// Margin after it.
    pub margin_right_px: f32,
}

/// Everything a renderer needs to draw the chip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedChip {
    /// Height.
    pub height_px: f32,
    /// Corner radius.
    pub radius_px: f32,
    /// Fill; alpha 0 when there is none.
    pub background: Rgba8,
    /// Label colour.
    pub foreground: Rgba8,
    /// Outlined only.
    pub border: Option<Border>,
    /// Space at each side of the label.
    pub label_padding_x_px: f32,
    /// Label size.
    pub font_size_px: f32,
    /// Line height, a multiple of the label size.
    pub line_height_ratio: f32,
    /// 1, or the disabled fade.
    pub opacity: f32,
    /// FocusVisible only.
    pub focus_ring: Option<FocusRing>,
    /// Deletable only.
    pub delete_icon: Option<DeleteIcon>,
}

fn tone_of(color: ChipColor, theme: &Theme) -> Option<Tone> {
    let s = theme.colors();
    match color {
        ChipColor::Default => None,
        ChipColor::Primary => Some(s.primary),
        ChipColor::Secondary => Some(s.secondary),
        ChipColor::Error => Some(s.error),
        ChipColor::Warning => Some(s.warning),
        ChipColor::Success => Some(s.success),
        ChipColor::Info => Some(s.info),
    }
}

/// `Chip.defaultBorder` of tokens.json: grey 400 in light, 700 in dark.
fn default_border(theme: &Theme) -> Rgba8 {
    match theme.scheme {
        ColorScheme::Light => GREY.shades[4],
        ColorScheme::Dark => GREY.shades[7],
    }
}

/// `(background, foreground, border)` of the chip in this state.
fn colors(p: ChipProps, hovered: bool, theme: &Theme) -> (Rgba8, Rgba8, Option<Border>) {
    let s = theme.colors();
    let hovered = hovered && p.clickable;
    let tone = tone_of(p.color, theme);
    match (p.variant, tone) {
        (ChipVariant::Filled, None) => {
            let fill = if hovered {
                let o = theme.opacities();
                tint(s.action.selected, o.selected + o.hover)
            } else {
                s.action.selected
            };
            (fill, s.text.primary, None)
        }
        (ChipVariant::Filled, Some(t)) => {
            (if hovered { t.dark } else { t.main }, t.contrast_text, None)
        }
        (ChipVariant::Outlined, None) => {
            let fill = if hovered {
                s.action.hover
            } else {
                transparent(s.text.primary)
            };
            let border = Border {
                width_px: 1.0,
                color: default_border(theme),
            };
            (fill, s.text.primary, Some(border))
        }
        (ChipVariant::Outlined, Some(t)) => {
            let fill = with_alpha(t.main, if hovered { s.action.hover.a } else { 0 });
            let border = Border {
                width_px: 1.0,
                color: tint(t.main, 0.7),
            };
            (fill, t.main, Some(border))
        }
    }
}

fn delete_icon(p: ChipProps, foreground: Rgba8, theme: &Theme) -> DeleteIcon {
    let (size_px, margin_left_px, margin_right_px) = match p.size {
        ChipSize::Small => (16.0, -4.0, 4.0),
        ChipSize::Medium => (22.0, -6.0, 5.0),
    };
    let (color, hover_color) = if p.color == ChipColor::Default {
        let ink = theme.colors().text.primary;
        (tint(ink, 0.26), tint(ink, 0.4))
    } else {
        (tint(foreground, 0.7), foreground)
    };
    DeleteIcon {
        size_px,
        color,
        hover_color,
        margin_left_px,
        margin_right_px,
    }
}

/// Resolves a chip style.
pub struct ChipStyle;

impl ChipStyle {
    /// The style of this chip in this state and theme.
    pub fn resolve(p: ChipProps, state: ChipState, theme: &Theme) -> ResolvedChip {
        let (background, foreground, border) = colors(p, state == ChipState::Hover, theme);
        let small = p.size == ChipSize::Small;
        let height_px = if small { 24.0 } else { 32.0 };
        let hairline = if p.variant == ChipVariant::Outlined {
            1.0
        } else {
            0.0
        };
        ResolvedChip {
            height_px,
            radius_px: if p.square { RADIUS_MD } else { height_px / 2.0 },
            background,
            foreground,
            border,
            label_padding_x_px: (if small { 8.0 } else { 12.0 }) - hairline,
            font_size_px: 13.0,
            line_height_ratio: 1.5,
            opacity: if state == ChipState::Disabled {
                theme.opacities().disabled
            } else {
                1.0
            },
            focus_ring: (state == ChipState::FocusVisible).then_some(FocusRing {
                width_px: FOCUS_RING_PX,
                color: theme.colors().primary.main,
            }),
            delete_icon: p.deletable.then(|| delete_icon(p, foreground, theme)),
        }
    }
}
