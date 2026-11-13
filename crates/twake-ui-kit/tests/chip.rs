//! Style of a chip (`Chip` of twake-mui), without a window.
//! R = ~/Developer/twake-space-chat; MUI = R/node_modules/@mui/material. Sources: MUI `Chip/Chip.js`
//! (root 74-100, delete icon 117-123, small 146-167, colours 170-185, clickable 220-245, outlined 255-318,
//! label 327-356), twake-mui `dist/lib/overrides.js:613-621` (`.square` = radius md 8), `radius.js`.
//! With `focusVisible: true` (R/src/app/theme.ts:92) MUI drops the focus *fill* of Chip.js:203,214,
//! 231,249,265 (`!theme.focusVisible && ...`): keyboard focus = the 2 px primary ring only.
//! Expected API (new `twake_ui_kit::chip`):
//! `ChipVariant {Filled, Outlined}`, `ChipColor {Default, Primary, Secondary, Error, Warning, Success, Info}`,
//! `ChipSize {Small, Medium}`, `ChipState {Rest, Hover, FocusVisible, Disabled}`,
//! `ChipProps {variant, color, size, clickable, deletable, square}` (derive `Default` = Filled, Default,
//! Medium, false, false, false), `ChipStyle::resolve(ChipProps, ChipState, &Theme) -> ResolvedChip` with
//! `ResolvedChip { height_px, radius_px, background, foreground, border: Option<button::Border>,
//! label_padding_x_px, font_size_px, line_height_ratio, opacity, focus_ring: Option<button::FocusRing>,
//! delete_icon: Option<DeleteIcon { size_px, color, hover_color, margin_left_px, margin_right_px }> }`.

use twake_tokens::{Rgba8, Tone};
use twake_ui_kit::button::Border;
use twake_ui_kit::chip::{ChipColor, ChipProps, ChipSize, ChipState, ChipStyle, ChipVariant, ResolvedChip};
use twake_ui_kit::theme::{ColorScheme, Theme};

use ChipColor::{Default as Neutral, Error, Info, Primary, Secondary, Success, Warning};
use ChipSize::{Medium, Small};
use ChipState::{Disabled, FocusVisible, Hover, Rest};
use ChipVariant::{Filled, Outlined};

fn themes() -> [Theme; 2] {
    [Theme::new(ColorScheme::Light), Theme::new(ColorScheme::Dark)]
}

fn a(f: f32) -> u8 {
    (f * 255.0).round() as u8
}

fn tint(c: Rgba8, f: f32) -> Rgba8 {
    Rgba8::new(c.r, c.g, c.b, a(f))
}

fn props(variant: ChipVariant, color: ChipColor, size: ChipSize) -> ChipProps {
    ChipProps { variant, color, size, ..ChipProps::default() }
}

fn clickable(p: ChipProps) -> ChipProps {
    ChipProps { clickable: true, ..p }
}

fn r(p: ChipProps, st: ChipState, t: &Theme) -> ResolvedChip {
    ChipStyle::resolve(p, st, t)
}

fn tone(c: ChipColor, t: &Theme) -> Tone {
    let s = t.colors();
    match c {
        Primary => s.primary,
        Secondary => s.secondary,
        Error => s.error,
        Warning => s.warning,
        Success => s.success,
        Info => s.info,
        Neutral => unreachable!("no tone"),
    }
}

// Chip.js:81 (height 32), 85 (radius 32 / 2), 77 (13 px), 82 (line height 1.5), 329/345 (label 12 / 8),
// 150 (small height 24).
#[test]
fn a_medium_chip_is_32px_high_with_a_16px_radius_and_a_12px_label_inset() {
    for t in themes() {
        let c = r(props(Filled, Neutral, Medium), Rest, &t);
        assert_eq!((c.height_px, c.radius_px, c.label_padding_x_px), (32.0, 16.0, 12.0));
        assert_eq!((c.font_size_px, c.line_height_ratio), (13.0, 1.5));
    }
}

#[test]
fn a_small_chip_is_24px_high_with_an_8px_label_inset() {
    let c = r(props(Filled, Neutral, Small), Rest, &themes()[0]);
    assert_eq!((c.height_px, c.radius_px, c.label_padding_x_px), (24.0, 12.0, 8.0));
}

// Chip.js:337,354: outlined labels are 1 px narrower to make room for the border (11, small 7).
#[test]
fn an_outlined_chip_insets_its_label_one_pixel_less() {
    let t = &themes()[0];
    assert_eq!(r(props(Outlined, Neutral, Medium), Rest, t).label_padding_x_px, 11.0);
    assert_eq!(r(props(Outlined, Neutral, Small), Rest, t).label_padding_x_px, 7.0);
}

// overrides.js:613-619 + tokens.json MuiChip: `&.square` = 8 px.
#[test]
fn a_square_chip_has_an_8px_radius() {
    let c = r(ChipProps { square: true, ..ChipProps::default() }, Rest, &themes()[0]);
    assert_eq!(c.radius_px, 8.0);
}

// Chip.js:78-79: root colour text.primary on action.selected.
#[test]
fn a_filled_default_chip_is_text_on_action_selected() {
    for t in themes() {
        let c = r(props(Filled, Neutral, Medium), Rest, &t);
        assert_eq!(c.background, t.colors().action.selected);
        assert_eq!(c.foreground, t.colors().text.primary);
        assert_eq!(c.border, None);
        assert_eq!(c.opacity, 1.0);
    }
}

// Chip.js:170-180: colour c = main with contrastText.
#[test]
fn a_filled_coloured_chip_is_contrast_text_on_its_main() {
    for t in themes() {
        for c in [Primary, Secondary, Error, Warning, Success, Info] {
            let s = r(props(Filled, c, Medium), Rest, &t);
            assert_eq!((s.background, s.foreground), (tone(c, &t).main, tone(c, &t).contrast_text), "{c:?}");
        }
    }
}

// Chip.js:229: clickable hover = action.selected channel at selectedOpacity + hoverOpacity
// (light 0.08 + 0.04, dark 0.16 + 0.08, makePalette.js:180-185 light, 86-91 dark).
#[test]
fn hovering_a_clickable_default_chip_darkens_it() {
    let [light, dark] = themes();
    let h = |t: &Theme| r(clickable(props(Filled, Neutral, Medium)), Hover, t).background;
    let (l, d) = (light.colors().action.selected, dark.colors().action.selected);
    assert_eq!(h(&light), Rgba8::new(l.r, l.g, l.b, a(0.12)));
    assert_eq!(h(&dark), Rgba8::new(d.r, d.g, d.b, a(0.24)));
}

#[test]
fn hovering_a_non_clickable_chip_changes_nothing() {
    for t in themes() {
        let p = props(Filled, Primary, Medium);
        assert_eq!(r(p, Hover, &t).background, r(p, Rest, &t).background);
    }
}

// Chip.js:236-247: clickable colour c hovers to `dark`.
#[test]
fn hovering_a_clickable_coloured_chip_goes_to_its_dark_tone() {
    for t in themes() {
        let c = r(clickable(props(Filled, Primary, Medium)), Hover, &t);
        assert_eq!(c.background, t.colors().primary.dark);
    }
}

// Chip.js:257-262: outlined = transparent, 1 px border Chip.defaultBorder (grey 400 light, 700 dark,
// tokens.json schemes.*.palette.Chip.defaultBorder), text.primary.
#[test]
fn an_outlined_default_chip_has_a_grey_hairline() {
    let [light, dark] = themes();
    let l = r(props(Outlined, Neutral, Medium), Rest, &light);
    let d = r(props(Outlined, Neutral, Medium), Rest, &dark);
    assert_eq!(l.background.a, 0);
    assert_eq!(l.border, Some(Border { width_px: 1.0, color: Rgba8::new(0xC5, 0xC7, 0xCA, 255) }));
    assert_eq!(d.border, Some(Border { width_px: 1.0, color: Rgba8::new(0x75, 0x77, 0x79, 255) }));
    assert_eq!(l.foreground, light.colors().text.primary);
}

// Chip.js:299-316: outlined colour c = text main, border main at 70 %, hover = main at hoverOpacity.
#[test]
fn an_outlined_coloured_chip_has_a_70_percent_border_and_a_faint_hover() {
    for t in themes() {
        let main = t.colors().primary.main;
        let rest = r(props(Outlined, Primary, Medium), Rest, &t);
        assert_eq!(rest.foreground, main);
        assert_eq!(rest.border, Some(Border { width_px: 1.0, color: tint(main, 0.7) }));
        let hover = r(clickable(props(Outlined, Primary, Medium)), Hover, &t);
        assert_eq!(hover.background, Rgba8::new(main.r, main.g, main.b, t.colors().action.hover.a));
    }
}

// Chip.js:262-264 (outlined clickable default hover = action.hover).
#[test]
fn hovering_a_clickable_outlined_default_chip_fills_it_with_action_hover() {
    for t in themes() {
        let c = r(clickable(props(Outlined, Neutral, Medium)), Hover, &t);
        assert_eq!(c.background, t.colors().action.hover);
    }
}

#[test]
fn keyboard_focus_draws_the_2px_primary_ring_and_no_fill_change() {
    for t in themes() {
        let p = clickable(props(Filled, Neutral, Medium));
        let c = r(p, FocusVisible, &t);
        let ring = c.focus_ring.expect("ring");
        assert_eq!((ring.width_px, ring.color), (2.0, t.colors().primary.main));
        assert_eq!(c.background, r(p, Rest, &t).background);
    }
}

// Chip.js:100: disabled = opacity action.disabledOpacity (0.32, makePalette.js:90,182).
#[test]
fn a_disabled_chip_is_faded_to_32_percent() {
    for t in themes() {
        assert_eq!(r(props(Filled, Neutral, Medium), Disabled, &t).opacity, 0.32);
    }
}

// Chip.js:117-123 (22 px, text.primary at 26 %, 40 % on hover, margin 0 5px 0 -6px);
// small 154-160 (16 px, margin-right 4, left -4).
#[test]
fn a_deletable_chip_has_a_faint_cross_that_darkens_on_hover() {
    for t in themes() {
        let p = ChipProps { deletable: true, ..ChipProps::default() };
        let d = r(p, Rest, &t).delete_icon.expect("cross");
        let ink = t.colors().text.primary;
        assert_eq!(d.size_px, 22.0);
        assert_eq!((d.margin_left_px, d.margin_right_px), (-6.0, 5.0));
        assert_eq!(d.color, tint(ink, 0.26));
        assert_eq!(d.hover_color, tint(ink, 0.4));
        let s = r(ChipProps { size: Small, ..p }, Rest, &t).delete_icon.expect("cross");
        assert_eq!((s.size_px, s.margin_left_px, s.margin_right_px), (16.0, -4.0, 4.0));
    }
}

// Chip.js:178-181: on a coloured chip the cross is contrastText at 70 %, full on hover.
#[test]
fn the_cross_of_a_coloured_chip_follows_its_text_colour() {
    for t in themes() {
        let p = ChipProps { deletable: true, color: Primary, ..ChipProps::default() };
        let d = r(p, Rest, &t).delete_icon.expect("cross");
        let ct = t.colors().primary.contrast_text;
        assert_eq!((d.color, d.hover_color), (tint(ct, 0.7), ct));
    }
}

#[test]
fn a_chip_without_delete_has_no_cross() {
    assert_eq!(r(ChipProps::default(), Rest, &themes()[0]).delete_icon, None);
}
