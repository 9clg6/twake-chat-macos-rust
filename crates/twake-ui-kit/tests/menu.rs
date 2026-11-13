//! Style of a menu and of its items (`Menu`, `MenuItem`, `Divider` of twake-mui), without a window.
//! R = ~/Developer/twake-space-chat; MUI = R/node_modules/@mui/material. Sources: twake-mui
//! `dist/lib/overrides.js:1272-1303` (`Menu` max width 320; `MenuItem` gap 16, padding 4 x 16, min height 40
//! from 544 px, icon `text.icon`), MUI `Menu/Menu.js:55`, `Popover/Popover.js:85-88,110` (elevation 8),
//! `Paper/Paper.js:56` (radius = shape.borderRadius 6), `List/List.js:44-48` (padding 8 0),
//! `MenuItem/MenuItem.js:66-137` (hover, selected, disabled, divider margin 8), `Divider/Divider.js`,
//! makePalette.js (`selectedOpacity`, `hoverOpacity`, `disabledOpacity`). With `focusVisible: true`
//! (R/src/app/theme.ts:92) MenuItem.js:102 draws an INSET 2 px primary ring and no focus fill.
//! The shadow is `SHADOWS_*[8]` of twake-tokens (makeShadows.js: 25 elevations, index = elevation).
//! Expected API (new `twake_ui_kit::menu`):
//! `MenuStyle::resolve(&Theme) -> MenuStyle { background: Rgba8, radius_px: f32, shadow: &'static str,
//!   max_width_px: f32, min_width_px: f32, padding_y_px: f32, viewport_margin_px: f32, max_height_inset_px: f32 }`;
//! `MenuItemState {Rest, Hover, Selected, SelectedHover, FocusVisible, Disabled}`;
//! `MenuItemStyle::resolve(MenuItemState, &Theme) -> MenuItemStyle { background, foreground, icon_color: Rgba8,
//!   font: TypeSpec, min_height_px, padding_x_px, padding_y_px, gap_px, text_margin_y_px, opacity: f32,
//!   focus_ring: Option<button::FocusRing>, ring_inset: bool }`;
//! `SeparatorStyle::resolve(&Theme) -> SeparatorStyle { height_px, color: Rgba8, margin_y_px }`.

use twake_tokens::{Rgba8, SHADOWS_DARK, SHADOWS_LIGHT, SHAPE_RADIUS, TYPE_BODY1};
use twake_ui_kit::menu::{MenuItemState, MenuItemStyle, MenuStyle, SeparatorStyle};
use twake_ui_kit::theme::{ColorScheme, Theme};

use MenuItemState::{Disabled, FocusVisible, Hover, Rest, Selected, SelectedHover};

fn themes() -> [Theme; 2] {
    [Theme::new(ColorScheme::Light), Theme::new(ColorScheme::Dark)]
}

fn a(f: f32) -> u8 {
    (f * 255.0).round() as u8
}

fn tint(c: Rgba8, f: f32) -> Rgba8 {
    Rgba8::new(c.r, c.g, c.b, a(f))
}

#[test]
fn a_menu_is_a_paper_with_a_6px_radius_and_the_elevation_8_shadow() {
    let [light, dark] = themes();
    let l = MenuStyle::resolve(&light);
    let d = MenuStyle::resolve(&dark);
    assert_eq!(l.background, light.colors().background.paper);
    assert_eq!(d.background, dark.colors().background.paper); // makeTheme.js:38: no dark overlay
    assert_eq!(l.radius_px, SHAPE_RADIUS);
    assert_eq!(l.shadow, SHADOWS_LIGHT[8]);
    assert_eq!(d.shadow, SHADOWS_DARK[8]);
}

// overrides.js:1274 (320), Popover.js:85,87 (min 16, `calc(100% - 32px)`), Menu.js:55 (`calc(100% - 96px)`),
// List.js:46-47 (8 px above and below the items).
#[test]
fn a_menu_is_at_most_320px_wide_and_keeps_a_gap_to_the_window() {
    let m = MenuStyle::resolve(&themes()[0]);
    assert_eq!((m.max_width_px, m.min_width_px), (320.0, 16.0));
    assert_eq!((m.viewport_margin_px, m.max_height_inset_px), (16.0, 96.0));
    assert_eq!(m.padding_y_px, 8.0);
}

// overrides.js:1280-1297 + MenuItem.js:66 (body1) + MenuItem.js:132 (gutters 16).
#[test]
fn an_item_is_40px_high_with_4_by_16_padding_and_a_16px_gap() {
    for t in themes() {
        let i = MenuItemStyle::resolve(Rest, &t);
        assert_eq!(i.min_height_px, 40.0);
        assert_eq!((i.padding_x_px, i.padding_y_px, i.gap_px), (16.0, 4.0, 16.0));
        assert_eq!(i.text_margin_y_px, 1.0); // overrides.js:1290-1293
        assert_eq!(i.font, TYPE_BODY1);
    }
}

// MenuItem is transparent at rest; text.primary (Paper colour); icons text.icon (overrides.js:1287).
#[test]
fn an_item_at_rest_is_transparent_with_text_and_icon_colours() {
    for t in themes() {
        let i = MenuItemStyle::resolve(Rest, &t);
        assert_eq!(i.background.a, 0);
        assert_eq!(i.foreground, t.colors().text.primary);
        assert_eq!(i.icon_color, t.colors().text.icon);
        assert_eq!((i.opacity, i.focus_ring), (1.0, None));
    }
}

// MenuItem.js:79: hover = action.hover.
#[test]
fn a_hovered_item_is_filled_with_action_hover() {
    for t in themes() {
        assert_eq!(MenuItemStyle::resolve(Hover, &t).background, t.colors().action.hover);
    }
}

// MenuItem.js:86: selected = primary.main at selectedOpacity (0.08 light, 0.16 dark);
// MenuItem.js:94: selected + hover = at selectedOpacity + hoverOpacity (0.12 light, 0.24 dark).
#[test]
fn a_selected_item_is_tinted_with_primary() {
    let [light, dark] = themes();
    let (lp, dp) = (light.colors().primary.main, dark.colors().primary.main);
    assert_eq!(MenuItemStyle::resolve(Selected, &light).background, tint(lp, 0.08));
    assert_eq!(MenuItemStyle::resolve(Selected, &dark).background, tint(dp, 0.16));
    assert_eq!(MenuItemStyle::resolve(SelectedHover, &light).background, tint(lp, 0.12));
    assert_eq!(MenuItemStyle::resolve(SelectedHover, &dark).background, tint(dp, 0.24));
}

// MenuItem.js:102 `applyInsetFocusVisible(1)`: ring inside, no fill; width 2 px primary.main.
#[test]
fn an_item_reached_by_keyboard_has_an_inset_2px_primary_ring_and_no_fill() {
    for t in themes() {
        let i = MenuItemStyle::resolve(FocusVisible, &t);
        let ring = i.focus_ring.expect("ring");
        assert_eq!((ring.width_px, ring.color), (2.0, t.colors().primary.main));
        assert!(i.ring_inset);
        assert_eq!(i.background.a, 0);
    }
}

// MenuItem.js:108 `opacity: action.disabledOpacity` (0.32, makePalette.js:90,182); no hover fill.
#[test]
fn a_disabled_item_is_faded_to_32_percent() {
    for t in themes() {
        let i = MenuItemStyle::resolve(Disabled, &t);
        assert_eq!(i.opacity, 0.32);
        assert_eq!(i.background.a, 0);
    }
}

// Divider.js:33-34 (`divider` colour, `thin` bottom border), MenuItem.js:111-113 (`& + divider`
// margin `spacing(1)` = 8 px above and below).
#[test]
fn a_separator_is_a_1px_divider_line_with_8px_around() {
    for t in themes() {
        let s = SeparatorStyle::resolve(&t);
        assert_eq!((s.height_px, s.margin_y_px), (1.0, 8.0));
        assert_eq!(s.color, t.colors().divider);
    }
}
