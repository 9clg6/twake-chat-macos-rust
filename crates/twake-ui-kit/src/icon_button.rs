//! Style of an icon button (`IconButton` of twake-mui, `ds/IconAction` of the app), without a window.
//! Sources: the app's override `src/app/theme.ts:150-178` of twake-space-chat (default colour
//! `text.secondary`, paddings 5/8/12/16, default size `medium`), twake-mui `overrides.js:739-795`,
//! MUI `IconButton.js` (round, hover = the colour at `hoverOpacity`).

use twake_tokens::Rgba8;

use crate::button::FocusRing;
use crate::color::{transparent, with_alpha};
use crate::theme::Theme;

const FOCUS_RING_PX: f32 = 2.0;

/// Padding around the icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconButtonSize {
    /// 5 px.
    XSmall,
    /// 8 px.
    Small,
    /// 12 px, the app default.
    #[default]
    Medium,
    /// 16 px.
    Large,
}

/// Which colour writes the icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconButtonColor {
    /// `text.secondary`.
    #[default]
    Default,
    /// The brand blue.
    Primary,
    /// The secondary tone.
    Secondary,
    /// The error tone.
    Error,
}

/// Interaction state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconButtonState {
    /// Nothing.
    Rest,
    /// Pointer over.
    Hover,
    /// Pointer down (same look as hover).
    Pressed,
    /// Reached by keyboard.
    FocusVisible,
    /// Not usable.
    Disabled,
}

/// Everything a renderer needs to draw the button around its icon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedIconButton {
    /// Fill; alpha 0 when there is none.
    pub background: Rgba8,
    /// Icon colour.
    pub foreground: Rgba8,
    /// Space between the icon and the edge, on each side.
    pub padding_px: f32,
    /// Corner radius as a fraction of the side (0.5 is a circle).
    pub radius_fraction: f32,
    /// FocusVisible only.
    pub focus_ring: Option<FocusRing>,
}

impl ResolvedIconButton {
    /// Side of the box around an icon of this size.
    pub fn diameter_px(&self, icon_px: f32) -> f32 {
        icon_px + 2.0 * self.padding_px
    }
}

const fn padding_px(size: IconButtonSize) -> f32 {
    match size {
        IconButtonSize::XSmall => 5.0,
        IconButtonSize::Small => 8.0,
        IconButtonSize::Medium => 12.0,
        IconButtonSize::Large => 16.0,
    }
}

/// Resolves an icon button style.
pub struct IconButtonStyle;

impl IconButtonStyle {
    /// The style of this icon button in this theme.
    pub fn resolve(
        size: IconButtonSize,
        color: IconButtonColor,
        state: IconButtonState,
        theme: &Theme,
    ) -> ResolvedIconButton {
        let scheme = theme.colors();
        let ink = match color {
            IconButtonColor::Default => scheme.text.secondary,
            IconButtonColor::Primary => scheme.primary.main,
            IconButtonColor::Secondary => scheme.secondary.main,
            IconButtonColor::Error => scheme.error.main,
        };
        let hovered = matches!(state, IconButtonState::Hover | IconButtonState::Pressed);
        let background = match (hovered, color) {
            (false, _) => transparent(ink),
            (true, IconButtonColor::Default) => scheme.action.hover,
            (true, _) => with_alpha(ink, scheme.action.hover.a),
        };
        let disabled = state == IconButtonState::Disabled;
        ResolvedIconButton {
            background,
            foreground: if disabled {
                scheme.action.disabled
            } else {
                ink
            },
            padding_px: padding_px(size),
            radius_fraction: 0.5,
            focus_ring: (state == IconButtonState::FocusVisible).then_some(FocusRing {
                width_px: FOCUS_RING_PX,
                color: scheme.primary.main,
            }),
        }
    }
}
