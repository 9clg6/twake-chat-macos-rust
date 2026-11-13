//! Style of a dialog (`Dialog`, `DialogTitle`, `DialogContent`, `DialogActions`, `Backdrop` of twake-mui),
//! without a window. Sources: twake-mui `overrides.js` (`MuiBackdrop` 444-460, `MuiDialog` 461-489,
//! title 490-513, content 514-557, actions 558-579, close button 741-756), MUI `Dialog.js:115,131,306`,
//! `Paper.js:56`. The shadow is `SHADOWS_*[24]`; no dark overlay on the paper.

use twake_tokens::{
    BREAKPOINT_LG, BREAKPOINT_MD, Rgba8, SHADOWS_DARK, SHADOWS_LIGHT, SHAPE_RADIUS, TYPE_H3,
    TYPE_H4, TypeSpec,
};

use crate::color::tint;
use crate::theme::{ColorScheme, Theme};

const ELEVATION: usize = 24;

/// How wide a dialog grows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogSize {
    /// 480 px.
    Small,
    /// 544 px.
    Medium,
    /// 800 px.
    Large,
    /// The whole window.
    Full,
}

/// Space on four sides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Insets {
    /// Left.
    pub left: f32,
    /// Top.
    pub top: f32,
    /// Right.
    pub right: f32,
    /// Bottom.
    pub bottom: f32,
}

/// The title row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DialogTitleStyle {
    /// Text style.
    pub font: TypeSpec,
    /// Horizontal padding.
    pub padding_x_px: f32,
    /// Vertical padding.
    pub padding_y_px: f32,
}

/// A dialog and its parts, for one size and window width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DialogStyle {
    /// Paper colour.
    pub background: Rgba8,
    /// Colour of the screen behind it.
    pub backdrop: Rgba8,
    /// Corner radius.
    pub radius_px: f32,
    /// CSS `box-shadow` of elevation 24.
    pub shadow: &'static str,
    /// Widest it grows; `None` is the whole width.
    pub max_width_px: Option<f32>,
    /// Gap kept to the window edges.
    pub margin_px: f32,
    /// Taken off the window height for the dialog's maximum height.
    pub max_height_inset_px: f32,
    /// Paper padding at the sides (a narrow `small` dialog only).
    pub padding_x_px: f32,
    /// Paper padding at the bottom (a narrow `small` dialog only).
    pub padding_bottom_px: f32,
    /// The title row.
    pub title: DialogTitleStyle,
    /// Padding of the content.
    pub content_padding: Insets,
    /// Side margin of the buttons row.
    pub actions_margin_x_px: f32,
    /// Top and bottom margin of the buttons row.
    pub actions_margin_y_px: f32,
    /// Space between two buttons.
    pub actions_gap_px: f32,
    /// Distance of the close button from the top and right edges.
    pub close_button_offset_px: f32,
}

const fn max_width(size: DialogSize) -> Option<f32> {
    match size {
        DialogSize::Small => Some(480.0),
        DialogSize::Medium => Some(544.0),
        DialogSize::Large => Some(800.0),
        DialogSize::Full => None,
    }
}

/// Values that change below `md` (769 px): `(title font, title y, gutter, actions y, close offset)`.
const fn by_width(narrow: bool) -> (TypeSpec, f32, f32, f32, f32) {
    if narrow {
        (TYPE_H4, 12.0, 16.0, 8.0, 4.0)
    } else {
        (TYPE_H3, 24.0, 32.0, 16.0, 18.4)
    }
}

impl DialogStyle {
    /// The style of a dialog of this size in a window of this width.
    pub fn resolve(size: DialogSize, window_width_px: f32, theme: &Theme) -> Self {
        let (shadows, backdrop) = match theme.scheme {
            ColorScheme::Light => (&SHADOWS_LIGHT, 0.12),
            ColorScheme::Dark => (&SHADOWS_DARK, 0.5),
        };
        let (font, title_y, gutter, actions_y, close) = by_width(window_width_px < BREAKPOINT_MD);
        // A `small` dialog below lg hugs the window closer (`overrides.js:470-479`).
        let (margin, inset, pad) = if size == DialogSize::Small && window_width_px < BREAKPOINT_LG {
            (16.0, 32.0, 8.0)
        } else {
            (32.0, 64.0, 0.0)
        };
        Self {
            background: theme.colors().background.paper,
            backdrop: tint(Rgba8::new(0, 0, 0, 255), backdrop),
            radius_px: SHAPE_RADIUS,
            shadow: shadows[ELEVATION],
            max_width_px: max_width(size),
            margin_px: margin,
            max_height_inset_px: inset,
            padding_x_px: pad,
            padding_bottom_px: pad,
            title: DialogTitleStyle {
                font,
                padding_x_px: gutter,
                padding_y_px: title_y,
            },
            content_padding: Insets {
                left: gutter,
                top: 24.0,
                right: gutter,
                bottom: 0.0,
            },
            actions_margin_x_px: gutter,
            actions_margin_y_px: actions_y,
            actions_gap_px: 4.0,
            close_button_offset_px: close,
        }
    }
}
