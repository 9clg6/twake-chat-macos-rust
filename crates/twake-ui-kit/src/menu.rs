//! Style of a menu and of its items (`Menu`, `MenuItem`, `Divider` of twake-mui), without a window.
//! twake-mui `overrides.js:1272-1303`, MUI `Menu.js:55`, `Popover.js:85-110` (elevation 8),
//! `Paper.js:56`, `List.js:44-48`, `MenuItem.js:66-137`. With `focusVisible: true` (the app's theme)
//! a focused item draws an inset 2 px primary ring and no fill. The shadow is `SHADOWS_*[8]`.

use twake_tokens::{Rgba8, SHADOWS_DARK, SHADOWS_LIGHT, SHAPE_RADIUS, TYPE_BODY1, TypeSpec};

use crate::button::FocusRing;
use crate::color::{tint, transparent};
use crate::theme::{ColorScheme, Theme};

const ELEVATION: usize = 8;
const FOCUS_RING_PX: f32 = 2.0;

/// The panel that holds the items.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuStyle {
    /// Paper colour.
    pub background: Rgba8,
    /// Corner radius.
    pub radius_px: f32,
    /// CSS `box-shadow` of elevation 8.
    pub shadow: &'static str,
    /// Widest the panel grows.
    pub max_width_px: f32,
    /// Narrowest the panel is.
    pub min_width_px: f32,
    /// Space above the first and below the last item.
    pub padding_y_px: f32,
    /// Gap kept to the window edges.
    pub viewport_margin_px: f32,
    /// Taken off the window height for the panel's maximum height.
    pub max_height_inset_px: f32,
}

impl MenuStyle {
    /// The panel style in this theme.
    pub fn resolve(theme: &Theme) -> Self {
        let shadows = match theme.scheme {
            ColorScheme::Light => &SHADOWS_LIGHT,
            ColorScheme::Dark => &SHADOWS_DARK,
        };
        Self {
            background: theme.colors().background.paper,
            radius_px: SHAPE_RADIUS,
            shadow: shadows[ELEVATION],
            max_width_px: 320.0,
            min_width_px: 16.0,
            padding_y_px: 8.0,
            viewport_margin_px: 16.0,
            max_height_inset_px: 96.0,
        }
    }
}

/// State of an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItemState {
    /// Nothing.
    Rest,
    /// Pointer over.
    Hover,
    /// The chosen one.
    Selected,
    /// The chosen one, pointer over.
    SelectedHover,
    /// Reached by keyboard.
    FocusVisible,
    /// Not usable.
    Disabled,
}

/// One row of a menu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuItemStyle {
    /// Fill; alpha 0 when there is none.
    pub background: Rgba8,
    /// Text colour.
    pub foreground: Rgba8,
    /// Icon colour.
    pub icon_color: Rgba8,
    /// Label style.
    pub font: TypeSpec,
    /// Least height.
    pub min_height_px: f32,
    /// Horizontal padding.
    pub padding_x_px: f32,
    /// Vertical padding.
    pub padding_y_px: f32,
    /// Space between the icon and the label.
    pub gap_px: f32,
    /// Space above and below the label.
    pub text_margin_y_px: f32,
    /// 1, or the disabled fade.
    pub opacity: f32,
    /// FocusVisible only.
    pub focus_ring: Option<FocusRing>,
    /// The ring is drawn inside the row.
    pub ring_inset: bool,
}

impl MenuItemStyle {
    /// The style of an item in this state and theme.
    pub fn resolve(state: MenuItemState, theme: &Theme) -> Self {
        let scheme = theme.colors();
        let o = theme.opacities();
        let background = match state {
            MenuItemState::Hover => scheme.action.hover,
            MenuItemState::Selected => tint(scheme.primary.main, o.selected),
            MenuItemState::SelectedHover => tint(scheme.primary.main, o.selected + o.hover),
            _ => transparent(scheme.text.primary),
        };
        Self {
            background,
            foreground: scheme.text.primary,
            icon_color: scheme.text.icon,
            font: TYPE_BODY1,
            min_height_px: 40.0,
            padding_x_px: 16.0,
            padding_y_px: 4.0,
            gap_px: 16.0,
            text_margin_y_px: 1.0,
            opacity: if state == MenuItemState::Disabled {
                o.disabled
            } else {
                1.0
            },
            focus_ring: (state == MenuItemState::FocusVisible).then_some(FocusRing {
                width_px: FOCUS_RING_PX,
                color: scheme.primary.main,
            }),
            ring_inset: state == MenuItemState::FocusVisible,
        }
    }
}

/// The line between two groups of items.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeparatorStyle {
    /// Thickness.
    pub height_px: f32,
    /// Colour.
    pub color: Rgba8,
    /// Space above and below.
    pub margin_y_px: f32,
}

impl SeparatorStyle {
    /// The separator in this theme.
    pub fn resolve(theme: &Theme) -> Self {
        Self {
            height_px: 1.0,
            color: theme.colors().divider,
            margin_y_px: 8.0,
        }
    }
}
