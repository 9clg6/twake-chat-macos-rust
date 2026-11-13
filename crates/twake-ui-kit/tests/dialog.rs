//! Style of a dialog (`Dialog`, `DialogTitle`, `DialogContent`, `DialogActions`, `Backdrop` of twake-mui),
//! without a window. R = ~/Developer/twake-space-chat; MUI = R/node_modules/@mui/material.
//! Sources: twake-mui `dist/lib/overrides.js` (`MuiBackdrop` 444-460, `MuiDialog` 461-489, title 490-513,
//! content 514-557, actions 558-579, close/back buttons 741-756), MUI `Dialog/Dialog.js:115` (margin 32),
//! `:131` (max height `100% - 64px`), `:306` (elevation 24), `Paper/Paper.js:56` (radius 6 =
//! `shape.borderRadius`, tokens.json `shape`), breakpoints md 769 and lg 1024 (`makeTheme.js:12-17`).
//! The shadow is `SHADOWS_*[24]`. No dark paper overlay (`makeTheme.js:38`).
//! Expected API (new `twake_ui_kit::dialog`): `DialogSize {Small, Medium, Large, Full}`;
//! `DialogStyle::resolve(DialogSize, window_width_px: f32, &Theme) -> DialogStyle {
//!   background, backdrop: Rgba8, radius_px, shadow: &'static str, max_width_px: Option<f32> (None = full),
//!   margin_px, max_height_inset_px, padding_x_px, padding_bottom_px (paper padding, narrow `small` only),
//!   title: DialogTitleStyle { font: TypeSpec, padding_x_px, padding_y_px },
//!   content_padding: (x, top, bottom) as `Insets { left, top, right, bottom }`, actions_margin_x_px,
//!   actions_margin_y_px, actions_gap_px, close_button_offset_px }`.

use twake_tokens::{Rgba8, SHADOWS_DARK, SHADOWS_LIGHT, SHAPE_RADIUS, TYPE_H3, TYPE_H4};
use twake_ui_kit::dialog::{DialogSize, DialogStyle};
use twake_ui_kit::theme::{ColorScheme, Theme};

use DialogSize::{Full, Large, Medium, Small};

const DESKTOP: f32 = 1280.0;
const NARROW: f32 = 700.0; // < md (769), < lg (1024)
const TABLET: f32 = 900.0; // >= md, < lg

fn light() -> Theme {
    Theme::new(ColorScheme::Light)
}

fn dark() -> Theme {
    Theme::new(ColorScheme::Dark)
}

fn r(size: DialogSize, w: f32, t: &Theme) -> DialogStyle {
    DialogStyle::resolve(size, w, t)
}

// overrides.js:469-484: small 480, medium 544, large 800, full 100 %.
#[test]
fn the_widest_a_dialog_grows_depends_on_its_size() {
    let t = light();
    assert_eq!(r(Small, DESKTOP, &t).max_width_px, Some(480.0));
    assert_eq!(r(Medium, DESKTOP, &t).max_width_px, Some(544.0));
    assert_eq!(r(Large, DESKTOP, &t).max_width_px, Some(800.0));
    assert_eq!(r(Full, DESKTOP, &t).max_width_px, None);
}

// Paper.js:56 (radius 6), Dialog.js:306 (elevation 24), Paper background.
#[test]
fn a_dialog_is_a_paper_with_a_6px_radius_and_the_elevation_24_shadow() {
    let (l, d) = (r(Medium, DESKTOP, &light()), r(Medium, DESKTOP, &dark()));
    assert_eq!(l.background, light().colors().background.paper);
    assert_eq!(d.background, dark().colors().background.paper);
    assert_eq!(l.radius_px, SHAPE_RADIUS);
    assert_eq!(l.shadow, SHADOWS_LIGHT[24]);
    assert_eq!(d.shadow, SHADOWS_DARK[24]);
}

// overrides.js:451-453: black at 12 % (light), 50 % (dark); 0.12 * 255 = 30.6 -> 31, 0.5 * 255 -> 128.
#[test]
fn the_backdrop_is_black_at_12_percent_in_light_and_50_percent_in_dark() {
    assert_eq!(r(Medium, DESKTOP, &light()).backdrop, Rgba8::new(0, 0, 0, 31));
    assert_eq!(r(Medium, DESKTOP, &dark()).backdrop, Rgba8::new(0, 0, 0, 128));
}

// Dialog.js:115 margin 32, :131 `calc(100% - 64px)`.
#[test]
fn a_desktop_dialog_keeps_32px_to_the_window_edges() {
    let d = r(Small, DESKTOP, &light());
    assert_eq!((d.margin_px, d.max_height_inset_px), (32.0, 64.0));
    assert_eq!((d.padding_x_px, d.padding_bottom_px), (0.0, 0.0));
}

// overrides.js:490-496: title = typography h3 (24/600), padding 1.5rem 2rem = 32 x 24.
#[test]
fn the_title_is_h3_with_24_by_32_padding() {
    let t = r(Medium, DESKTOP, &light()).title;
    assert_eq!(t.font, TYPE_H3);
    assert_eq!((t.padding_x_px, t.padding_y_px), (32.0, 24.0));
}

// overrides.js:514-519: content `24px 32px 0`.
#[test]
fn the_content_has_24px_above_32px_aside_and_nothing_below() {
    let c = r(Medium, DESKTOP, &light()).content_padding;
    assert_eq!((c.left, c.top, c.right, c.bottom), (32.0, 24.0, 32.0, 0.0));
}

// overrides.js:558-563: actions margin `16px 32px`, 4 px between buttons.
#[test]
fn the_actions_have_a_16_by_32_margin_and_4px_between_buttons() {
    let d = r(Medium, DESKTOP, &light());
    assert_eq!((d.actions_margin_x_px, d.actions_margin_y_px, d.actions_gap_px), (32.0, 16.0, 4.0));
}

// overrides.js:743-755: `dialogIconButton` top / right `1.15rem` = 18.4 px (0.25rem = 4 px below md).
#[test]
fn the_close_button_sits_18_point_4px_from_the_corner_and_4px_on_a_narrow_window() {
    assert_eq!(r(Medium, DESKTOP, &light()).close_button_offset_px, 18.4);
    assert_eq!(r(Medium, NARROW, &light()).close_button_offset_px, 4.0);
}

// Below md (769 px; overrides.js:496-499,515,559-562): title h4 with 12 x 16, content 24 x 16, actions 8 x 16.
#[test]
fn below_769px_the_title_shrinks_and_the_gutters_halve() {
    let d = r(Medium, NARROW, &light());
    assert_eq!(d.title.font, TYPE_H4);
    assert_eq!((d.title.padding_x_px, d.title.padding_y_px), (16.0, 12.0));
    let c = d.content_padding;
    assert_eq!((c.left, c.top, c.right, c.bottom), (16.0, 24.0, 16.0, 0.0));
    assert_eq!((d.actions_margin_x_px, d.actions_margin_y_px), (16.0, 8.0));
    // from md up, the desktop values hold
    assert_eq!(r(Medium, TABLET, &light()).title.font, TYPE_H3);
}

// Below lg (1024 px; overrides.js:470-479): a `small` dialog gets margin 16, padding `0 8px 8px`,
// max height `100% - 32px`; medium and large are not touched.
#[test]
fn a_small_dialog_below_1024px_hugs_the_window_closer() {
    let d = r(Small, TABLET, &light());
    assert_eq!((d.margin_px, d.max_height_inset_px), (16.0, 32.0));
    assert_eq!((d.padding_x_px, d.padding_bottom_px), (8.0, 8.0));
    let m = r(Medium, TABLET, &light());
    assert_eq!((m.margin_px, m.padding_x_px), (32.0, 0.0));
}
