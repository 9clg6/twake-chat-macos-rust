//! Style and timing of a tooltip (`Tooltip` of twake-mui as the app sets it), without a window.
//! twake-mui `overrides.js:1179-1197` (grey 800 at 90 %, radius sm, arrow, line height 1.3), MUI
//! `Tooltip.js:125-148,234-248,311`, and the app's override `src/app/theme.ts:180-200` (13 px,
//! padding 4 x 8, top, enter delay 1000 ms, no fade). The app's values win over twake-mui's
//! (16 px, 8 x 12): the Rust app mirrors the React app. The colour of the arrow is not drawn yet (A029).

use twake_tokens::{GREY, RADIUS_SM, Rgba8, WHITE, Z_TOOLTIP};

use crate::color::tint;
use crate::theme::Theme;

const BACKGROUND_OPACITY: f32 = 0.9;

/// How a tooltip looks and when it shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TooltipStyle {
    /// Fill.
    pub background: Rgba8,
    /// Text colour.
    pub text: Rgba8,
    /// Corner radius.
    pub radius_px: f32,
    /// Horizontal padding.
    pub padding_x_px: f32,
    /// Vertical padding.
    pub padding_y_px: f32,
    /// Text size.
    pub font_size_px: f32,
    /// Line height, a multiple of the text size.
    pub line_height_ratio: f32,
    /// Text weight.
    pub font_weight: u16,
    /// Widest the box grows.
    pub max_width_px: f32,
    /// Gap between the target and the box (arrow included).
    pub offset_px: f32,
    /// Whether an arrow points at the target.
    pub arrow: bool,
    /// Stacking order.
    pub z_index: u32,
    /// Wait before it shows, hovering from nothing.
    pub enter_delay_ms: u32,
    /// Wait before it shows, when another tooltip just closed.
    pub enter_next_delay_ms: u32,
    /// Wait before it hides.
    pub leave_delay_ms: u32,
    /// How long after closing counts as "just closed".
    pub hysteresis_ms: u32,
    /// Fade in and out; 0 is none.
    pub fade_ms: u32,
}

impl TooltipStyle {
    /// The tooltip style; it is the same in both schemes (`palette`, not `vars`).
    pub fn resolve(_theme: &Theme) -> Self {
        Self {
            background: tint(GREY.shades[8], BACKGROUND_OPACITY),
            text: WHITE,
            radius_px: RADIUS_SM,
            padding_x_px: 8.0,
            padding_y_px: 4.0,
            font_size_px: 13.0,
            line_height_ratio: 1.3,
            font_weight: 500,
            max_width_px: 300.0,
            offset_px: 14.0,
            arrow: true,
            z_index: Z_TOOLTIP,
            enter_delay_ms: 1000,
            enter_next_delay_ms: 1000,
            leave_delay_ms: 0,
            hysteresis_ms: 800,
            fade_ms: 0,
        }
    }
}
