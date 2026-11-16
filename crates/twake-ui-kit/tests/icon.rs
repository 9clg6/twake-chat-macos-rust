//! The SVG an icon draws at a size, in a colour.
//! Need: docs/spec/design-system.md section 1.6 (`<Icon icon size>` renders width and height; single-colour
//! glyphs follow the colour of the host, illustrations keep their own), decision A013 (hard-coded colours stay),
//! docs/dev/ui-kit-api-sketch.md section 3.

use twake_icons::{ASSISTANT_COLOR, CHECK, SEND};
use twake_tokens::Rgba8;
use twake_ui_kit::icon::{IconColor, IconSpec, svg_for};

fn spec(icon: &'static twake_icons::Icon, size_px: f32, color: IconColor) -> IconSpec {
    IconSpec {
        icon,
        size_px,
        color,
    }
}

#[test]
fn the_size_sets_width_and_height() {
    let svg = svg_for(&spec(&SEND, 20.0, IconColor::Current));
    assert!(svg.contains("width=\"20\""), "{svg}");
    assert!(svg.contains("height=\"20\""), "{svg}");
}

#[test]
fn a_fractional_size_keeps_its_fraction() {
    let svg = svg_for(&spec(&SEND, 14.5, IconColor::Current));
    assert!(
        svg.contains("width=\"14.5\"") && svg.contains("height=\"14.5\""),
        "{svg}"
    );
}

#[test]
fn a_wide_icon_is_still_drawn_in_a_square_box() {
    // `check` is 17 x 13: the box scales with its aspect kept, as `<Icon size={16}>` does.
    let svg = svg_for(&spec(&CHECK, 16.0, IconColor::Current));
    assert!(
        svg.contains("width=\"16\"") && svg.contains("height=\"16\""),
        "{svg}"
    );
    assert!(svg.contains("viewBox=\"0 0 17 13\""), "{svg}");
}

#[test]
fn the_current_colour_follows_the_host() {
    let svg = svg_for(&spec(&SEND, 16.0, IconColor::Current));
    assert!(svg.contains("fill=\"currentColor\""), "{svg}");
}

#[test]
fn a_fixed_opaque_colour_is_written_as_six_hex_digits() {
    let color = IconColor::Fixed(Rgba8::new(10, 132, 255, 255));
    let svg = svg_for(&spec(&SEND, 16.0, color));
    assert!(svg.contains("fill=\"#0a84ff\""), "{svg}");
    assert!(!svg.contains("currentColor"), "{svg}");
}

#[test]
fn a_fixed_translucent_colour_is_written_as_eight_hex_digits() {
    let color = IconColor::Fixed(Rgba8::new(66, 66, 68, 163));
    let svg = svg_for(&spec(&SEND, 16.0, color));
    assert!(svg.contains("fill=\"#424244a3\""), "{svg}");
}

#[test]
fn the_body_of_the_icon_is_never_touched() {
    let color = IconColor::Fixed(Rgba8::new(1, 2, 3, 255));
    let svg = svg_for(&spec(&SEND, 16.0, color));
    assert!(svg.starts_with("<svg "), "{svg}");
    assert!(svg.ends_with(&format!("{}</svg>", SEND.body)), "{svg}");
}

#[test]
fn an_icon_with_hard_coded_colours_is_not_recoloured() {
    assert!(
        ASSISTANT_COLOR.body.contains("fill=\"#"),
        "the fixture must hold a hard-coded colour"
    );
    let tinted = svg_for(&spec(
        &ASSISTANT_COLOR,
        24.0,
        IconColor::Fixed(Rgba8::new(10, 132, 255, 255)),
    ));
    let plain = svg_for(&spec(&ASSISTANT_COLOR, 24.0, IconColor::Current));
    assert_eq!(tinted, plain);
    assert!(!tinted.contains("#0a84ff"), "{tinted}");
}
