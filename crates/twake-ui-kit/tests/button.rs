//! Style of a button for a variant, a colour, a size and a state, without a window.
//! Need: docs/spec/design-system.md section 3 (row "Button"), section 1.7 (focus), 1.2 (typography `button`),
//! lines 107-114 (`action-*` colours); docs/spec/tokens.json `components.MuiButton.styleOverrides.root`
//! (`borderRadius`, `minHeight`, `padding` and their `variants`); docs/dev/ui-kit-api-sketch.md section 2
//! (the rest is MUI v9 defaults, open questions 1, 2, 4 and 5 there).

use twake_tokens::{DARK, LIGHT, LineHeight, Rgba8, TYPE_BUTTON, Tone, TypeSpec};
use twake_ui_kit::button::{
    Border, ButtonColor, ButtonSize, ButtonState, ButtonStyle, ButtonVariant, FocusRing,
    ResolvedButton,
};
use twake_ui_kit::theme::{ColorScheme, Theme};

use ButtonColor::{Error, Info, Primary, Secondary, Success, Warning};
use ButtonSize::{Large, Medium, Small, XSmall};
use ButtonState::{Disabled, FocusVisible, Hover, Pressed, Rest};
use ButtonVariant::{Contained, Outlined, Text};

fn light() -> Theme {
    Theme::new(ColorScheme::Light)
}

fn dark() -> Theme {
    Theme::new(ColorScheme::Dark)
}

fn resolve(
    variant: ButtonVariant,
    color: ButtonColor,
    size: ButtonSize,
    state: ButtonState,
    theme: &Theme,
) -> ResolvedButton {
    ButtonStyle::resolve(variant, color, size, state, theme)
}

fn rest(variant: ButtonVariant, color: ButtonColor) -> ResolvedButton {
    resolve(variant, color, Medium, Rest, &light())
}

fn with_alpha(color: Rgba8, a: u8) -> Rgba8 {
    Rgba8::new(color.r, color.g, color.b, a)
}

fn tone(color: ButtonColor, theme: &Theme) -> Tone {
    let scheme = theme.colors();
    match color {
        Primary => scheme.primary,
        Secondary => scheme.secondary,
        Success => scheme.success,
        Error => scheme.error,
        Warning => scheme.warning,
        Info => scheme.info,
    }
}

fn padding(variant: ButtonVariant, size: ButtonSize) -> (f32, f32) {
    let style = resolve(variant, Primary, size, Rest, &light());
    (style.padding_x, style.padding_y)
}

// ---- Contained (tokens.json MuiButton root: padding "10px 24px", minHeight 40, borderRadius "100px")

#[test]
fn a_contained_primary_button_at_rest_is_the_blue_pill_of_the_brand() {
    let style = rest(Contained, Primary);
    // design-system.md section 3 "Button": radius 100, min-height 40, padding 10px 24px.
    assert_eq!(style.background, Rgba8::new(10, 132, 255, 255));
    assert_eq!(style.foreground, Rgba8::new(255, 255, 255, 255));
    assert_eq!(style.radius_px, 100.0);
    assert_eq!(style.height_px, 40.0);
    assert_eq!((style.padding_x, style.padding_y), (24.0, 10.0));
}

#[test]
fn a_contained_button_has_no_border_and_no_ring_at_rest() {
    let style = rest(Contained, Primary);
    assert_eq!(style.border, None);
    assert_eq!(style.focus_ring, None);
}

#[test]
fn a_contained_button_uses_the_button_text_style() {
    // design-system.md section 1.2: button = 14 / 500 / 1.313 / 0.1px.
    let style = rest(Contained, Primary);
    assert_eq!(style.font, TYPE_BUTTON);
    assert_eq!(style.font.font_size, 14.0);
    assert_eq!(style.font.font_weight, 500);
}

#[test]
fn hovering_a_contained_button_darkens_it() {
    let style = resolve(Contained, Primary, Medium, Hover, &light());
    assert_eq!(style.background, Rgba8::new(0, 107, 216, 255));
    assert_eq!(style.foreground, Rgba8::new(255, 255, 255, 255));
}

#[test]
fn pressing_a_contained_button_looks_like_hovering_it() {
    let pressed = resolve(Contained, Primary, Medium, Pressed, &light());
    let hovered = resolve(Contained, Primary, Medium, Hover, &light());
    assert_eq!(pressed, hovered);
}

#[test]
fn a_contained_secondary_button_takes_the_secondary_tone() {
    let tones = LIGHT.secondary;
    let at_rest = rest(Contained, Secondary);
    assert_eq!(at_rest.background, tones.main);
    assert_eq!(at_rest.foreground, tones.contrast_text);
    let hovered = resolve(Contained, Secondary, Medium, Hover, &light());
    assert_eq!(hovered.background, tones.dark);
}

#[test]
fn a_contained_error_button_takes_the_error_tone() {
    let at_rest = rest(Contained, Error);
    assert_eq!(at_rest.background, Rgba8::new(255, 51, 71, 255));
    let hovered = resolve(Contained, Error, Medium, Hover, &light());
    assert_eq!(hovered.background, Rgba8::new(211, 44, 60, 255));
}

#[test]
fn a_contained_button_in_the_dark_scheme_uses_the_dark_tones() {
    let style = resolve(Contained, Primary, Medium, Rest, &dark());
    assert_eq!(style.background, DARK.primary.main);
    assert_eq!(style.background, Rgba8::new(123, 189, 255, 255));
    assert_eq!(style.foreground, DARK.primary.contrast_text);
    let hovered = resolve(Contained, Primary, Medium, Hover, &dark());
    assert_eq!(hovered.background, DARK.primary.dark);
}

#[test]
fn a_disabled_contained_button_is_grey_in_the_light_scheme() {
    // design-system.md lines 111 and 113: action-disabled .24, action-disabledBackground .12 (ink 66,66,68).
    let style = resolve(Contained, Primary, Medium, Disabled, &light());
    assert_eq!(style.background, Rgba8::new(66, 66, 68, 31));
    assert_eq!(style.foreground, Rgba8::new(66, 66, 68, 61));
}

#[test]
fn a_disabled_contained_button_is_grey_in_the_dark_scheme() {
    // design-system.md lines 111 and 113: action-disabled .32, action-disabledBackground .12 (white).
    let style = resolve(Contained, Primary, Medium, Disabled, &dark());
    assert_eq!(style.background, Rgba8::new(255, 255, 255, 31));
    assert_eq!(style.foreground, Rgba8::new(255, 255, 255, 82));
}

#[test]
fn a_disabled_button_ignores_its_colour() {
    let primary = resolve(Contained, Primary, Medium, Disabled, &light());
    let error = resolve(Contained, Error, Medium, Disabled, &light());
    assert_eq!(primary, error);
}

// ---- Focus (design-system.md 1.7: a 2 px ring in primary.main on every keyboard-reached control)

#[test]
fn a_focused_button_keeps_its_colours_and_gets_a_two_pixel_ring() {
    let focused = resolve(Contained, Primary, Medium, FocusVisible, &light());
    let at_rest = rest(Contained, Primary);
    assert_eq!(focused.background, at_rest.background);
    assert_eq!(focused.foreground, at_rest.foreground);
    let ring = FocusRing {
        width_px: 2.0,
        color: Rgba8::new(10, 132, 255, 255),
    };
    assert_eq!(focused.focus_ring, Some(ring));
}

#[test]
fn the_ring_of_an_error_button_is_still_primary() {
    let focused = resolve(Outlined, Error, Medium, FocusVisible, &light());
    let ring = focused.focus_ring.map(|ring| ring.color);
    assert_eq!(ring, Some(LIGHT.primary.main));
}

#[test]
fn the_ring_follows_the_dark_scheme() {
    let focused = resolve(Text, Primary, Medium, FocusVisible, &dark());
    let ring = focused.focus_ring.map(|ring| (ring.width_px, ring.color));
    assert_eq!(ring, Some((2.0, DARK.primary.main)));
}

#[test]
fn only_a_focused_button_has_a_ring() {
    for state in [Rest, Hover, Pressed, Disabled] {
        let style = resolve(Contained, Primary, Medium, state, &light());
        assert_eq!(style.focus_ring, None);
    }
}

// ---- Outlined (padding "5px 15px", borderColor currentColor)

#[test]
fn an_outlined_button_at_rest_is_transparent_with_a_coloured_border() {
    let style = rest(Outlined, Primary);
    let border = Border {
        width_px: 1.0,
        color: Rgba8::new(10, 132, 255, 255),
    };
    assert_eq!(style.background.a, 0);
    assert_eq!(style.foreground, Rgba8::new(10, 132, 255, 255));
    assert_eq!(style.border, Some(border));
    assert_eq!((style.padding_x, style.padding_y), (15.0, 5.0));
    assert_eq!(style.height_px, 40.0);
    assert_eq!(style.radius_px, 100.0);
}

#[test]
fn the_border_of_an_outlined_button_has_the_colour_of_its_text() {
    let style = rest(Outlined, Error);
    assert_eq!(style.foreground, LIGHT.error.main);
    assert_eq!(
        style.border.map(|border| border.color),
        Some(LIGHT.error.main)
    );
    let secondary = rest(Outlined, Secondary);
    assert_eq!(secondary.foreground, LIGHT.secondary.main);
    assert_eq!(
        secondary.border.map(|border| border.color),
        Some(LIGHT.secondary.main)
    );
}

#[test]
fn hovering_an_outlined_button_tints_it_by_four_percent_in_light() {
    // design-system.md line 108: action-hoverOpacity 0.04 (light) -> alpha 10.
    let style = resolve(Outlined, Primary, Medium, Hover, &light());
    assert_eq!(style.background, with_alpha(LIGHT.primary.main, 10));
    assert_eq!(style.foreground, LIGHT.primary.main);
}

#[test]
fn hovering_an_outlined_button_tints_it_by_eight_percent_in_dark() {
    // design-system.md line 108: action-hoverOpacity 0.08 (dark) -> alpha 20.
    let style = resolve(Outlined, Primary, Medium, Hover, &dark());
    assert_eq!(style.background, with_alpha(DARK.primary.main, 20));
}

#[test]
fn pressing_an_outlined_button_looks_like_hovering_it() {
    let pressed = resolve(Outlined, Secondary, Medium, Pressed, &light());
    let hovered = resolve(Outlined, Secondary, Medium, Hover, &light());
    assert_eq!(pressed, hovered);
}

#[test]
fn a_disabled_outlined_button_has_grey_text_and_no_fill() {
    let style = resolve(Outlined, Primary, Medium, Disabled, &light());
    assert_eq!(style.foreground, Rgba8::new(66, 66, 68, 61));
    assert_eq!(style.background.a, 0);
}

// ---- Text (padding "10px 8px", minWidth auto)

#[test]
fn a_text_button_at_rest_is_plain_coloured_text() {
    let style = rest(Text, Primary);
    assert_eq!(style.background.a, 0);
    assert_eq!(style.foreground, Rgba8::new(10, 132, 255, 255));
    assert_eq!(style.border, None);
    assert_eq!((style.padding_x, style.padding_y), (8.0, 10.0));
    assert_eq!(style.height_px, 40.0);
    assert_eq!(style.radius_px, 100.0);
}

#[test]
fn a_text_button_takes_the_colour_of_its_tone() {
    assert_eq!(rest(Text, Error).foreground, LIGHT.error.main);
    assert_eq!(rest(Text, Secondary).foreground, LIGHT.secondary.main);
}

#[test]
fn hovering_a_text_button_tints_it() {
    let in_light = resolve(Text, Error, Medium, Hover, &light());
    assert_eq!(in_light.background, with_alpha(LIGHT.error.main, 10));
    let in_dark = resolve(Text, Error, Medium, Hover, &dark());
    assert_eq!(in_dark.background, with_alpha(DARK.error.main, 20));
}

#[test]
fn a_disabled_text_button_has_grey_text() {
    let style = resolve(Text, Primary, Medium, Disabled, &dark());
    assert_eq!(style.foreground, Rgba8::new(255, 255, 255, 82));
    assert_eq!(style.background.a, 0);
}

#[test]
fn the_tone_of_a_button_is_the_one_of_the_active_scheme() {
    for theme in [light(), dark()] {
        let style = resolve(Text, Secondary, Medium, Rest, &theme);
        assert_eq!(style.foreground, tone(Secondary, &theme).main);
    }
}

// ---- Sizes (tokens.json MuiButton root.variants: size xsmall, small, large)

#[test]
fn the_heights_follow_the_size() {
    let heights: Vec<f32> = [XSmall, Small, Medium, Large]
        .into_iter()
        .map(|size| resolve(Contained, Primary, size, Rest, &light()).height_px)
        .collect();
    assert_eq!(heights, [24.0, 36.0, 40.0, 48.0]);
}

#[test]
fn the_contained_padding_follows_the_size() {
    assert_eq!(padding(Contained, XSmall), (8.0, 4.0));
    assert_eq!(padding(Contained, Small), (16.0, 9.0));
    assert_eq!(padding(Contained, Medium), (24.0, 10.0));
    assert_eq!(padding(Contained, Large), (32.0, 13.0));
}

#[test]
fn the_text_padding_follows_the_size() {
    assert_eq!(padding(Text, Small), (16.0, 8.0));
    assert_eq!(padding(Text, Large), (10.0, 14.0));
}

#[test]
fn the_outlined_padding_follows_the_size() {
    assert_eq!(padding(Outlined, XSmall), (7.0, 3.0));
    // Small and large outlined are not written in tokens.json: the size rule wins over the variant rule.
    assert_eq!(padding(Outlined, Small), (16.0, 9.0));
    assert_eq!(padding(Outlined, Large), (32.0, 13.0));
}

#[test]
fn an_extra_small_button_has_the_small_label_style() {
    // tokens.json: size xsmall -> fontSize 0.6875rem (11 px), lineHeight 16px, letterSpacing 0.5px.
    let style = resolve(Contained, Primary, XSmall, Rest, &light());
    let expected = TypeSpec {
        font_size: 11.0,
        font_weight: 500,
        line_height: LineHeight::Px(16.0),
        letter_spacing: 0.5,
    };
    assert_eq!(style.font, expected);
}

#[test]
fn the_other_sizes_keep_the_button_text_style() {
    for size in [Small, Medium, Large] {
        let style = resolve(Text, Primary, size, Rest, &light());
        assert_eq!(style.font, TYPE_BUTTON);
    }
}

// ---- Whatever the variant, size or state

#[test]
fn every_button_is_a_pill() {
    for variant in [Contained, Outlined, Text] {
        for size in [XSmall, Small, Medium, Large] {
            let style = resolve(variant, Primary, size, Rest, &light());
            assert_eq!(style.radius_px, 100.0);
        }
    }
}

#[test]
fn a_button_is_never_faded() {
    for state in [Rest, Hover, Pressed, FocusVisible, Disabled] {
        let style = resolve(Outlined, Primary, Medium, state, &light());
        assert_eq!(style.opacity, 1.0);
    }
}
