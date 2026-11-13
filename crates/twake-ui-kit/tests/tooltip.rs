//! Style and timing of a tooltip (`Tooltip` of twake-mui as the app sets it), without a window.
//! R = ~/Developer/twake-space-chat; MUI = R/node_modules/@mui/material. Sources:
//! twake-mui `dist/lib/overrides.js:1179-1197` (grey 800 at 90 %, radius sm, arrow, line height 1.3),
//! tokens.json `components.MuiTooltip` (same, "rgba(78, 80, 82, 0.9)"), the APP's override
//! `R/src/app/theme.ts:180-200` (13 px, padding 4 x 8, top, enter delay 1000 ms, no fade),
//! `docs/spec/design-system.md:343`, MUI `Tooltip/Tooltip.js:125-148,234-248,311`.
//! The app's values win over twake-mui's (16 px, 8 x 12): the Rust app mirrors the React app.
//! Expected API (new `twake_ui_kit::tooltip`): `TooltipStyle::resolve(&Theme) -> TooltipStyle` with fields
//! `background, text: Rgba8; radius_px, padding_x_px, padding_y_px, font_size_px, line_height_ratio: f32;
//! font_weight: u16; max_width_px, offset_px: f32; arrow: bool; z_index: u32;
//! enter_delay_ms, enter_next_delay_ms, leave_delay_ms, hysteresis_ms, fade_ms: u32`.

use twake_tokens::{Rgba8, Z_TOOLTIP};
use twake_ui_kit::theme::{ColorScheme, Theme};
use twake_ui_kit::tooltip::TooltipStyle;

fn themes() -> [Theme; 2] {
    [Theme::new(ColorScheme::Light), Theme::new(ColorScheme::Dark)]
}

// overrides.js:1183 `alpha(theme.palette.grey[800], 0.9)`: `palette`, not `vars`, so the same in
// both schemes; grey 800 = #4E5052 (tokens.json paletteRaw.Grey). 0.9 * 255 = 229.5, rounded to 230.
#[test]
fn the_background_is_grey_800_at_90_percent_in_both_schemes() {
    for t in themes() {
        assert_eq!(TooltipStyle::resolve(&t).background, Rgba8::new(78, 80, 82, 230));
    }
}

// Tooltip.js:126 `common.white`.
#[test]
fn the_text_is_white_in_both_schemes() {
    for t in themes() {
        assert_eq!(TooltipStyle::resolve(&t).text, Rgba8::new(255, 255, 255, 255));
    }
}

// overrides.js:1184 `radius.sm` = 4 px (not the 6 px of shape.borderRadius); theme.ts:194-195.
#[test]
fn it_has_a_4px_radius_13px_text_and_4_by_8_padding() {
    let s = TooltipStyle::resolve(&themes()[0]);
    assert_eq!(s.radius_px, 4.0);
    assert_eq!(s.font_size_px, 13.0); // 0.8125rem
    assert_eq!((s.padding_x_px, s.padding_y_px), (8.0, 4.0));
}

// overrides.js:1190 `lineHeight: 1.3`; Tooltip.js:133 `fontWeightMedium`; Tooltip.js:130 maxWidth 300.
#[test]
fn it_is_medium_weight_with_a_line_height_of_1_3_and_300px_at_most() {
    let s = TooltipStyle::resolve(&themes()[0]);
    assert_eq!((s.line_height_ratio, s.font_weight, s.max_width_px), (1.3, 500, 300.0));
}

// Tooltip.js:144 (top placement `marginBottom: 14px`, kept with an arrow); theme.ts:187 `placement: 'top'`;
// overrides.js:1180 `arrow: true`.
#[test]
fn it_floats_14px_over_its_target_with_an_arrow() {
    let s = TooltipStyle::resolve(&themes()[0]);
    assert_eq!(s.offset_px, 14.0);
    assert!(s.arrow);
}

// theme.ts:188-189 `enterDelay: 1000, enterNextDelay: 1000`; Tooltip.js:247 `leaveDelay = 0`;
// Tooltip.js:311 hysteresis `800 + leaveDelay`; theme.ts:186 `transition: { timeout: 0 }` (no fade).
#[test]
fn it_opens_after_one_second_closes_at_once_and_does_not_fade() {
    let s = TooltipStyle::resolve(&themes()[0]);
    assert_eq!((s.enter_delay_ms, s.enter_next_delay_ms), (1000, 1000));
    assert_eq!((s.leave_delay_ms, s.hysteresis_ms, s.fade_ms), (0, 800, 0));
}

// zIndex.tooltip 1500 (tokens.json zIndex).
#[test]
fn it_sits_above_dialogs_and_snackbars() {
    assert_eq!(TooltipStyle::resolve(&themes()[0]).z_index, Z_TOOLTIP);
}
