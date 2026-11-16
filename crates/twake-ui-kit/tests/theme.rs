//! The theme: which colour scheme of `twake-tokens` is active.
//! Need: docs/spec/design-system.md section 1.7 ("Theme: light / dark / system"), docs/dev/ui-kit-api-sketch.md section 1.

use twake_tokens::{DARK, LIGHT};
use twake_ui_kit::theme::{ColorScheme, Theme};

#[test]
fn a_light_theme_exposes_the_light_scheme() {
    let theme = Theme::new(ColorScheme::Light);
    assert_eq!(theme.colors(), &LIGHT);
}

#[test]
fn a_dark_theme_exposes_the_dark_scheme() {
    let theme = Theme::new(ColorScheme::Dark);
    assert_eq!(theme.colors(), &DARK);
}

#[test]
fn the_theme_is_built_from_its_public_field() {
    let theme = Theme {
        scheme: ColorScheme::Dark,
    };
    assert_eq!(theme.colors(), &DARK);
}

#[test]
fn the_system_appearance_dark_gives_a_dark_theme() {
    assert_eq!(Theme::system(true).scheme, ColorScheme::Dark);
}

#[test]
fn the_system_appearance_light_gives_a_light_theme() {
    assert_eq!(Theme::system(false).scheme, ColorScheme::Light);
}

#[test]
fn toggling_a_light_theme_makes_it_dark() {
    let mut theme = Theme::new(ColorScheme::Light);
    theme.toggle();
    assert_eq!(theme.scheme, ColorScheme::Dark);
    assert_eq!(theme.colors(), &DARK);
}

#[test]
fn toggling_twice_comes_back() {
    let mut theme = Theme::new(ColorScheme::Dark);
    theme.toggle();
    assert_eq!(theme.scheme, ColorScheme::Light);
    theme.toggle();
    assert_eq!(theme.scheme, ColorScheme::Dark);
}

#[test]
fn the_two_schemes_really_differ() {
    assert_ne!(LIGHT, DARK);
}
