//! Button `ghost` variant and the `success`, `warning`, `info` colours, without a window.
//! Need: twake-mui 19.3.0 `dist/lib/overrides.js` (R = ~/Developer/twake-space-chat/node_modules/@linagora/twake-mui),
//! `MuiButton` lines 164-290; opacities from `dist/lib/makePalette.js:95-96,187-188`
//! (`ghostOpacity` 0.08, `hoverGhostOpacity` 0.16, same in both schemes).
//! Expected API (to create in `twake_ui_kit::button`): `ButtonVariant::Ghost`,
//! `ButtonColor::{Success, Warning, Info}`; `ButtonStyle::resolve` unchanged.

use twake_tokens::{Rgba8, Tone};
use twake_ui_kit::button::{
    Border, ButtonColor, ButtonSize, ButtonState, ButtonStyle, ButtonVariant, ResolvedButton,
};
use twake_ui_kit::theme::{ColorScheme, Theme};

use ButtonColor::{Error, Info, Primary, Secondary, Success, Warning};
use ButtonSize::{Large, Medium, Small, XSmall};
use ButtonState::{Disabled, FocusVisible, Hover, Pressed, Rest};
use ButtonVariant::{Contained, Ghost, Outlined, Text};

fn themes() -> [Theme; 2] {
    [
        Theme::new(ColorScheme::Light),
        Theme::new(ColorScheme::Dark),
    ]
}

/// CSS alpha `f` as the byte of `Rgba8` (rounded, as the generated `action.*` are).
fn a(f: f32) -> u8 {
    (f * 255.0).round() as u8
}

fn tint(c: Rgba8, f: f32) -> Rgba8 {
    Rgba8::new(c.r, c.g, c.b, a(f))
}

fn tone(color: ButtonColor, theme: &Theme) -> Tone {
    let s = theme.colors();
    match color {
        Primary => s.primary,
        Secondary => s.secondary,
        Error => s.error,
        Success => s.success,
        Warning => s.warning,
        Info => s.info,
    }
}

fn r(
    v: ButtonVariant,
    c: ButtonColor,
    s: ButtonSize,
    st: ButtonState,
    t: &Theme,
) -> ResolvedButton {
    ButtonStyle::resolve(v, c, s, st, t)
}

// overrides.js:189 (ghost padding '5px 15px'), then size rules 218-248: xsmall '4px 8px'
// (no ghost rule, so it wins), small '9px 16px', large '13px 32px'.
#[test]
fn a_ghost_button_has_the_paddings_of_its_size() {
    for t in themes() {
        let p = |s| {
            let b = r(Ghost, Primary, s, Rest, &t);
            (b.padding_x, b.padding_y, b.height_px)
        };
        assert_eq!(p(XSmall), (8.0, 4.0, 24.0));
        assert_eq!(p(Small), (16.0, 9.0, 36.0));
        assert_eq!(p(Medium), (15.0, 5.0, 40.0));
        assert_eq!(p(Large), (32.0, 13.0, 48.0));
    }
}

#[test]
fn a_ghost_button_is_a_pill_without_border_at_rest() {
    for t in themes() {
        let b = r(Ghost, Primary, Medium, Rest, &t);
        assert_eq!(b.radius_px, 100.0); // radius.pill, overrides.js:173
        assert_eq!(b.border, None);
        assert_eq!(b.focus_ring, None);
        assert_eq!(b.opacity, 1.0);
    }
}

// overrides.js:187-198: primary ghost = text.primary on primary.main at 25 %, 32 % on hover.
#[test]
fn a_primary_ghost_is_tinted_25_percent_and_32_percent_on_hover() {
    for t in themes() {
        let c = t.colors();
        let rest = r(Ghost, Primary, Medium, Rest, &t);
        assert_eq!(rest.background, tint(c.primary.main, 0.25));
        assert_eq!(rest.foreground, c.text.primary);
        for st in [Hover, Pressed] {
            let h = r(Ghost, Primary, Medium, st, &t);
            assert_eq!(h.background, tint(c.primary.main, 0.32));
            assert_eq!(h.foreground, c.text.primary);
        }
    }
}

// overrides.js:203-214 + makePalette.js:95-96: other colours are written in their main
// colour on 8 % of it, 16 % on hover.
#[test]
fn a_coloured_ghost_is_tinted_8_percent_and_16_percent_on_hover() {
    for t in themes() {
        for color in [Secondary, Success, Error, Warning, Info] {
            let main = tone(color, &t).main;
            let rest = r(Ghost, color, Medium, Rest, &t);
            assert_eq!(rest.background, tint(main, 0.08), "{color:?} rest");
            assert_eq!(rest.foreground, main, "{color:?} text");
            let hover = r(Ghost, color, Medium, Hover, &t);
            assert_eq!(hover.background, tint(main, 0.16), "{color:?} hover");
            assert_eq!(hover.foreground, main);
        }
    }
}

// overrides.js:196-200: disabled = transparent, 1 px border action.disabledBackground (any colour).
#[test]
fn a_disabled_ghost_is_transparent_with_a_faint_border() {
    for t in themes() {
        for color in [Primary, Secondary, Success, Error, Warning, Info] {
            let b = r(Ghost, color, Medium, Disabled, &t);
            assert_eq!(b.background.a, 0, "{color:?}");
            assert_eq!(
                b.border,
                Some(Border {
                    width_px: 1.0,
                    color: t.colors().action.disabled_background
                })
            );
            assert_eq!(b.foreground, t.colors().action.disabled);
        }
    }
}

#[test]
fn a_ghost_button_reached_by_keyboard_has_the_2px_primary_ring_and_keeps_its_fill() {
    for t in themes() {
        let b = r(Ghost, Success, Medium, FocusVisible, &t);
        let ring = b.focus_ring.expect("ring");
        assert_eq!((ring.width_px, ring.color), (2.0, t.colors().primary.main));
        assert_eq!(b.background, tint(t.colors().success.main, 0.08));
    }
}

// The three new colours follow the contained / outlined / text rules of the existing ones.
#[test]
fn success_warning_info_contained_buttons_use_their_tone() {
    for t in themes() {
        for color in [Success, Warning, Info] {
            let tn = tone(color, &t);
            let rest = r(Contained, color, Medium, Rest, &t);
            assert_eq!(
                (rest.background, rest.foreground),
                (tn.main, tn.contrast_text)
            );
            let hover = r(Contained, color, Medium, Hover, &t);
            assert_eq!(hover.background, tn.dark);
        }
    }
}

#[test]
fn success_warning_info_outlined_and_text_buttons_are_written_in_their_main_colour() {
    for t in themes() {
        for color in [Success, Warning, Info] {
            let main = tone(color, &t).main;
            for v in [Outlined, Text] {
                let b = r(v, color, Medium, Rest, &t);
                assert_eq!(b.foreground, main, "{v:?} {color:?}");
                assert_eq!(b.background.a, 0);
            }
            let o = r(Outlined, color, Medium, Rest, &t);
            assert_eq!(
                o.border,
                Some(Border {
                    width_px: 1.0,
                    color: main
                })
            );
        }
    }
}
