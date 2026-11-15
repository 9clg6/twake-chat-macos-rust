//! CSS `box-shadow` text of the tokens as layers.
//! Need: CSS Backgrounds 3, `box-shadow` (`<x> <y> <blur>? <spread>? <color>`, comma-separated layers, `none`);
//! values of `SHADOWS_LIGHT` / `SHADOWS_DARK` in twake-tokens (makeShadows.js).

use twake_tokens::{Rgba8, SHADOWS_DARK, SHADOWS_LIGHT};
use twake_ui_kit::shadow::{ShadowLayer, parse_shadow};

fn layer(x: f32, y: f32, blur: f32, spread: f32, color: (u8, u8, u8, u8)) -> ShadowLayer {
    ShadowLayer {
        x_px: x,
        y_px: y,
        blur_px: blur,
        spread_px: spread,
        color: Rgba8::new(color.0, color.1, color.2, color.3),
    }
}

#[test]
fn none_has_no_layers() {
    assert!(parse_shadow("none").is_empty());
    assert!(parse_shadow(SHADOWS_LIGHT[0]).is_empty());
    assert!(parse_shadow(SHADOWS_DARK[0]).is_empty());
}

#[test]
fn one_layer() {
    assert_eq!(
        parse_shadow("0px 2px 4px 1px rgba(10, 20, 30, 1)"),
        vec![layer(0.0, 2.0, 4.0, 1.0, (10, 20, 30, 255))]
    );
}

#[test]
fn elevation_8_of_the_light_tokens_has_three_layers_in_order() {
    // "0px 5px 8px -2px rgba(66, 66, 68, 0.13), 0px 6px 26px 2px rgba(66, 66, 68, 0.11), 0px 0px 0px 0.5px rgba(66, 66, 68, 0.12)"
    assert_eq!(
        parse_shadow(SHADOWS_LIGHT[8]),
        vec![
            layer(0.0, 5.0, 8.0, -2.0, (66, 66, 68, 33)),
            layer(0.0, 6.0, 26.0, 2.0, (66, 66, 68, 28)),
            layer(0.0, 0.0, 0.0, 0.5, (66, 66, 68, 31)),
        ]
    );
}

#[test]
fn elevation_24_of_the_light_tokens_has_three_layers_in_order() {
    // "0px 12px 16px -8px rgba(66, 66, 68, 0.24), 0px 12px 48px 8px rgba(66, 66, 68, 0.24), 0px 0px 0px 0.5px rgba(66, 66, 68, 0.12)"
    assert_eq!(
        parse_shadow(SHADOWS_LIGHT[24]),
        vec![
            layer(0.0, 12.0, 16.0, -8.0, (66, 66, 68, 61)),
            layer(0.0, 12.0, 48.0, 8.0, (66, 66, 68, 61)),
            layer(0.0, 0.0, 0.0, 0.5, (66, 66, 68, 31)),
        ]
    );
}

#[test]
fn every_elevation_of_both_schemes_that_is_not_none_gives_a_layer_per_comma_group() {
    for (scheme, tokens) in [("light", SHADOWS_LIGHT), ("dark", SHADOWS_DARK)] {
        for (i, css) in tokens.iter().enumerate().skip(1) {
            let expected = css.matches("rgba(").count();
            let layers = parse_shadow(css);
            assert_eq!(layers.len(), expected, "{scheme}[{i}] = {css}");
            assert!(
                layers.iter().all(|l| l.color.a > 0),
                "{scheme}[{i}] has a transparent layer"
            );
        }
    }
}

#[test]
fn alpha_is_rounded_to_a_byte() {
    let a = |css: &str| parse_shadow(css)[0].color.a;
    assert_eq!(a("0px 0px 0px 0px rgba(0, 0, 0, 0.08)"), 20);
    assert_eq!(a("0px 0px 0px 0px rgba(0, 0, 0, 0.12)"), 31);
    assert_eq!(a("0px 0px 0px 0px rgba(0, 0, 0, 0)"), 0);
    assert_eq!(a("0px 0px 0px 0px rgba(0, 0, 0, 1)"), 255);
}

#[test]
fn offsets_and_blur_may_be_negative_or_decimal() {
    assert_eq!(
        parse_shadow("-1.5px -2px 0.25px -0.5px rgba(1, 2, 3, 1)"),
        vec![layer(-1.5, -2.0, 0.25, -0.5, (1, 2, 3, 255))]
    );
}

#[test]
fn an_absent_spread_is_zero() {
    // The last token, SHADOWS_LIGHT[25], is written without spread.
    assert_eq!(
        parse_shadow("0px 1px 4px rgba(66, 66, 68, 0.04)"),
        vec![layer(0.0, 1.0, 4.0, 0.0, (66, 66, 68, 10))]
    );
    assert_eq!(parse_shadow(SHADOWS_LIGHT[25]).len(), 1);
}

#[test]
fn malformed_input_gives_no_layer_and_no_panic() {
    for css in [
        "",
        "   ",
        "1px",
        "garbage text",
        "0px 0px 0px 0px rgba",
        "0px 0px 0px 0px rgba(1, 2, 3, 0.5",
        "0px 0px 0px 0px rgba(1, 2, 3)",
        "0px 0px 0px 0px rgba(a, b, c, d)",
        "0px 0px 0px 0px rgba()",
        ",",
        ")(",
    ] {
        assert!(parse_shadow(css).is_empty(), "{css:?}");
    }
}

#[test]
fn a_malformed_layer_is_skipped_and_the_others_stay() {
    assert_eq!(
        parse_shadow(
            "0px 1px 2px 3px rgba(1, 2, 3, 1), nonsense, 4px 5px 6px 7px rgba(8, 9, 10, 1)"
        ),
        vec![
            layer(0.0, 1.0, 2.0, 3.0, (1, 2, 3, 255)),
            layer(4.0, 5.0, 6.0, 7.0, (8, 9, 10, 255)),
        ]
    );
}

#[test]
fn extra_whitespace_changes_nothing() {
    assert_eq!(
        parse_shadow(
            "  0px   1px\t2px  3px   rgba(  1 ,2,   3 ,  1 )  ,   4px 5px 6px 7px rgba(8,9,10,1)  "
        ),
        vec![
            layer(0.0, 1.0, 2.0, 3.0, (1, 2, 3, 255)),
            layer(4.0, 5.0, 6.0, 7.0, (8, 9, 10, 255)),
        ]
    );
}
