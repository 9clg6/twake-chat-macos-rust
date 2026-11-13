//! Geometry and colours of a room row and of its unread badge (`RoomRow.tsx` of the React app), as data.

use twake_ui_kit::theme::Theme;
use twake_ui_kit::{Rgba8, TYPE_BODY2, TypeSpec};

use crate::room_row::BadgeStyle;

/// Width of the room list column.
pub const LIST_WIDTH_PX: f32 = 360.0;

// ponytail: `RoomRow.tsx` sets these as literals; twake-tokens has no token for them.
pub(crate) const ROW_HEIGHT_PX: f32 = 56.0;
const AVATAR_PX: f32 = 48.0;
const GAP_PX: f32 = 8.0;
const BADGE_HEIGHT_PX: f32 = 16.0;
const BADGE_PADDING_PX: f32 = 4.0;
const BADGE_FONT_PX: f32 = 10.0;
const BADGE_WEIGHT: u16 = 500;
const DOT_PX: f32 = 8.0;

/// What the pointer and the app do to a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowState {
    /// The open room.
    pub selected: bool,
    /// The pointer is over it.
    pub hovered: bool,
}

/// How a row is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowStyle {
    /// Height.
    pub height_px: f32,
    /// Avatar diameter.
    pub avatar_px: f32,
    /// Space between the avatar and the texts.
    pub gap_px: f32,
    /// Style of the name.
    pub name_font: TypeSpec,
    /// Fill; transparent at rest.
    pub background: Rgba8,
}

impl RowStyle {
    /// The style of a row in `state`.
    pub fn resolve(state: RowState, theme: &Theme) -> Self {
        let action = theme.colors().action;
        let background = match (state.selected, state.hovered) {
            (true, true) => action.focus,
            (true, false) => action.selected,
            (false, true) => action.hover,
            (false, false) => Rgba8::new(0, 0, 0, 0),
        };
        Self {
            height_px: ROW_HEIGHT_PX,
            avatar_px: AVATAR_PX,
            gap_px: GAP_PX,
            name_font: TYPE_BODY2,
            background,
        }
    }
}

/// How the unread badge is drawn: a pill with a count, or a dot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BadgeSpec {
    /// Height.
    pub height_px: f32,
    /// Narrowest width (a count of one digit is round).
    pub min_width_px: f32,
    /// Left and right padding.
    pub padding_x_px: f32,
    /// Corner radius.
    pub radius_px: f32,
    /// Text size.
    pub font_size_px: f32,
    /// Text weight.
    pub font_weight: u16,
    /// Fill.
    pub background: Rgba8,
    /// Text colour.
    pub text: Rgba8,
}

impl BadgeSpec {
    /// The badge of `style`, a dot when `dot`.
    pub fn resolve(style: BadgeStyle, dot: bool, theme: &Theme) -> Self {
        let colors = theme.colors();
        let (background, text) = match style {
            BadgeStyle::Normal | BadgeStyle::Mention => {
                (colors.primary.main, colors.primary.contrast_text)
            }
            BadgeStyle::Muted => (colors.action.disabled_background, colors.text.primary),
        };
        let (height_px, padding_x_px) = if dot {
            (DOT_PX, 0.0)
        } else {
            (BADGE_HEIGHT_PX, BADGE_PADDING_PX)
        };
        Self {
            height_px,
            min_width_px: height_px,
            padding_x_px,
            radius_px: height_px / 2.0,
            font_size_px: BADGE_FONT_PX,
            font_weight: BADGE_WEIGHT,
            background,
            text,
        }
    }
}
