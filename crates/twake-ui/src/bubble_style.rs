//! Colours and geometry of a message bubble (`MessageBubble.tsx` of the React app), as data.
//! The 1 px shadow is drawn by the timeline view; the tail is not here yet (A026).

use twake_ui_kit::Rgba8;
use twake_ui_kit::color::mix;
use twake_ui_kit::theme::Theme;

use twake_ui_kit::theme::ColorScheme;

use crate::timeline_rows::Side;

// ponytail: `MessageBubble.tsx` sets these as literals; twake-tokens has no token for them.
const RADIUS_PX: f32 = 16.0;
const PADDING_X_PX: f32 = 16.0;
const PADDING_Y_PX: f32 = 8.0;
const FONT_SIZE_PX: f32 = 15.0;
const LINE_HEIGHT_PX: f32 = 20.0;
/// Share of the column an own bubble tints: `color-mix(in srgb, primary 12%, paper)`.
const OWN_TINT: f32 = 0.12;
const MAX_SHARE: f32 = 0.75;
const MAX_WIDTH_PX: f32 = 520.0;

/// What lies behind the messages: the fallback colour of `wallpaper.ts` of the React app (its
/// image is not drawn here), so that the paper of the bubbles of others stands out on it.
pub fn wallpaper(theme: &Theme) -> Rgba8 {
    match theme.scheme {
        ColorScheme::Light => Rgba8::new(222, 228, 252, 255),
        ColorScheme::Dark => Rgba8::new(44, 52, 72, 255),
    }
}

/// How a bubble is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BubbleStyle {
    /// Fill.
    pub background: Rgba8,
    /// Text colour.
    pub text: Rgba8,
    /// Corner radius.
    pub radius_px: f32,
    /// Left and right padding.
    pub padding_x_px: f32,
    /// Top and bottom padding.
    pub padding_y_px: f32,
    /// Text size.
    pub font_size_px: f32,
    /// Line height.
    pub line_height_px: f32,
    /// Widest the bubble grows: `min(75 %, 520 px)` of the column.
    pub max_width_px: f32,
}

impl BubbleStyle {
    /// The style of a bubble of `side` in a column `container_px` wide.
    pub fn resolve(side: Side, theme: &Theme, container_px: f32) -> Self {
        let colors = theme.colors();
        let background = match side {
            Side::Own => mix(colors.primary.main, colors.background.paper, OWN_TINT),
            Side::Other => colors.background.paper,
        };
        Self {
            background,
            text: colors.text.primary,
            radius_px: RADIUS_PX,
            padding_x_px: PADDING_X_PX,
            padding_y_px: PADDING_Y_PX,
            font_size_px: FONT_SIZE_PX,
            line_height_px: LINE_HEIGHT_PX,
            max_width_px: (container_px * MAX_SHARE).min(MAX_WIDTH_PX),
        }
    }
}
