//! Style of an icon button (`IconButton` of twake-mui, `ds/IconAction` of the app), without a window.
//! R = ~/Developer/twake-space-chat. Sources: `R/src/app/theme.ts:150-178` (the app's override:
//! default colour `text.secondary`, paddings 5/8/12/16), twake-mui `dist/lib/overrides.js:739-795`,
//! MUI `@mui/material/IconButton/IconButton.js` (round, hover = colour at `hoverOpacity`,
//! disabled = transparent + `action.disabled`), focus ring `R/src/app/theme.ts:92` (`focusVisible: true`)
//! and `docs/spec/design-system.md:332` (2 px `primary.main`).
//! Expected API (new `twake_ui_kit::icon_button`):
//! `IconButtonSize {XSmall, Small, Medium, Large}`, `IconButtonColor {Default, Primary, Secondary, Error}`,
//! `IconButtonState {Rest, Hover, Pressed, FocusVisible, Disabled}`,
//! `ResolvedIconButton { background, foreground, padding_px, radius_fraction, focus_ring: Option<button::FocusRing> }`
//! with `fn diameter_px(&self, icon_px: f32) -> f32`, and `IconButtonStyle::resolve(size, color, state, &Theme)`.

use twake_tokens::Rgba8;
use twake_ui_kit::icon_button::{
    IconButtonColor, IconButtonSize, IconButtonState, IconButtonStyle, ResolvedIconButton,
};
use twake_ui_kit::theme::{ColorScheme, Theme};

use IconButtonColor::{Default, Error, Primary, Secondary};
use IconButtonSize::{Large, Medium, Small, XSmall};
use IconButtonState::{Disabled, FocusVisible, Hover, Pressed, Rest};

fn themes() -> [Theme; 2] {
    [
        Theme::new(ColorScheme::Light),
        Theme::new(ColorScheme::Dark),
    ]
}

fn r(s: IconButtonSize, c: IconButtonColor, st: IconButtonState, t: &Theme) -> ResolvedIconButton {
    IconButtonStyle::resolve(s, c, st, t)
}

// theme.ts:161-164 and overrides.js:766-790 (the app default is `medium`, theme.ts:153).
#[test]
fn paddings_are_5_8_12_16_by_size() {
    let t = &themes()[0];
    let p = |s| r(s, Default, Rest, t).padding_px;
    assert_eq!(
        (p(XSmall), p(Small), p(Medium), p(Large)),
        (5.0, 8.0, 12.0, 16.0)
    );
}

#[test]
fn the_box_is_the_icon_plus_the_padding_on_each_side() {
    let t = &themes()[0];
    assert_eq!(r(Medium, Default, Rest, t).diameter_px(16.0), 40.0);
    assert_eq!(r(XSmall, Default, Rest, t).diameter_px(16.0), 26.0);
    assert_eq!(r(Large, Default, Rest, t).diameter_px(24.0), 56.0); // the 56 px hover of twake-mui's `large` default, theme.ts:151-152
}

// IconButton.js: `borderRadius: '50%'`.
#[test]
fn an_icon_button_is_round() {
    let t = &themes()[0];
    assert_eq!(r(Small, Default, Rest, t).radius_fraction, 0.5);
}

// theme.ts:158-160 / overrides.js:761-763: the default colour is text.secondary, in the scheme on screen.
#[test]
fn the_default_icon_is_text_secondary_and_the_box_is_transparent() {
    for t in themes() {
        let b = r(Medium, Default, Rest, &t);
        assert_eq!(b.foreground, t.colors().text.secondary);
        assert_eq!(b.background.a, 0);
        assert_eq!(b.focus_ring, None);
    }
}

// IconButton.js: hover bg = palette.action.active channel at hoverOpacity = `action.hover`.
#[test]
fn hover_and_press_fill_the_default_button_with_action_hover() {
    for t in themes() {
        for st in [Hover, Pressed] {
            let b = r(Medium, Default, st, &t);
            assert_eq!(b.background, t.colors().action.hover);
            assert_eq!(b.foreground, t.colors().text.secondary);
        }
    }
}

// IconButton.js: colour c: text main, `--IconButton-hoverBg` = main at hoverOpacity (4 % light, 8 % dark).
#[test]
fn a_coloured_icon_button_is_written_in_its_main_and_hovers_in_a_tint_of_it() {
    for t in themes() {
        let s = t.colors();
        for (c, main) in [
            (Primary, s.primary.main),
            (Secondary, s.secondary.main),
            (Error, s.error.main),
        ] {
            let rest = r(Medium, c, Rest, &t);
            assert_eq!((rest.foreground, rest.background.a), (main, 0));
            let hover = r(Medium, c, Hover, &t);
            assert_eq!(
                hover.background,
                Rgba8::new(main.r, main.g, main.b, s.action.hover.a)
            );
            assert_eq!(hover.foreground, main);
        }
    }
}

// IconButton.js `&.Mui-disabled`: transparent, `action.disabled`; no hover fill.
#[test]
fn a_disabled_icon_button_is_transparent_and_grey() {
    for t in themes() {
        for c in [Default, Primary, Error] {
            let b = r(Medium, c, Disabled, &t);
            assert_eq!(b.background.a, 0);
            assert_eq!(b.foreground, t.colors().action.disabled);
        }
    }
}

#[test]
fn keyboard_focus_draws_a_2px_primary_ring_and_no_fill() {
    for t in themes() {
        let b = r(Medium, Default, FocusVisible, &t);
        let ring = b.focus_ring.expect("ring");
        assert_eq!((ring.width_px, ring.color), (2.0, t.colors().primary.main));
        assert_eq!(b.background.a, 0);
    }
}
