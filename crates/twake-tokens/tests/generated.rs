//! The constants generated from `docs/spec/tokens.json` by `cargo xtask tokens`.
//! Need: the design system is data, not hand-copied constants (`design-system.md` section 4):
//! every value below is read from the JSON at run time and compared with the crate, plus a few
//! literals from the brief (primary 600 = `#0a84ff`, white contrast text).
//! Public items used are listed in `docs/dev/tokens-api-sketch.md`.

use std::error::Error;

use serde_json::Value;
use twake_tokens::{
    BLACK, BREAKPOINT_LG, BREAKPOINT_MD, BREAKPOINT_SM, BREAKPOINT_XL, BREAKPOINT_XS, DARK, ERROR,
    GREY, INFO, LIGHT, LineHeight, PRIMARY, Palette, RADIUS_LG, RADIUS_MD, RADIUS_NONE,
    RADIUS_PILL, RADIUS_SM, Rgba8, SECONDARY, SHAPE_RADIUS, SPACING, SUCCESS, TYPE_BODY1,
    TYPE_BODY2, TYPE_BODY3, TYPE_BUTTON, TYPE_CAPTION, TYPE_H1, TYPE_H2, TYPE_H3, TYPE_H4, TYPE_H5,
    TYPE_H6, TYPE_OVERLINE, TYPE_SUBTITLE1, TYPE_SUBTITLE2, TypeSpec, WARNING, WHITE,
};

type Res = Result<(), Box<dyn Error>>;

const SHADES: [u16; 10] = [50, 100, 200, 300, 400, 500, 600, 700, 800, 900];

fn tokens() -> Result<Value, Box<dyn Error>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/spec/tokens.json");
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("{key} is not a string in tokens.json").into())
}

fn number(value: &Value, key: &str) -> Result<f32, Box<dyn Error>> {
    let raw = value[key]
        .as_f64()
        .ok_or_else(|| format!("{key} is not a number in tokens.json"))?;
    Ok(raw as f32)
}

fn channel(text: &str) -> Result<u8, Box<dyn Error>> {
    Ok(text.trim().parse()?)
}

/// `#hex`, or `rgb(r, g, b)` / `rgba(r, g, b, a)` with `a` in 0..=1 (alpha rounded to a byte).
fn css(color: &str) -> Result<Rgba8, Box<dyn Error>> {
    if color.starts_with('#') {
        return Ok(Rgba8::from_hex(color)?);
    }
    let inner = color
        .strip_prefix("rgba(")
        .or_else(|| color.strip_prefix("rgb("))
        .and_then(|rest| rest.strip_suffix(')'))
        .ok_or_else(|| format!("unsupported colour {color:?}"))?;
    let parts: Vec<&str> = inner.split(',').collect();
    let alpha = match parts.get(3) {
        Some(a) => (a.trim().parse::<f64>()? * 255.0).round() as u8,
        None => 255,
    };
    Ok(Rgba8::new(
        channel(parts[0])?,
        channel(parts[1])?,
        channel(parts[2])?,
        alpha,
    ))
}

fn assert_palette(json: &Value, name: &str, palette: &Palette) -> Res {
    let raw = &json["paletteRaw"][name];
    for shade in SHADES {
        let expected = css(text(raw, &shade.to_string())?)?;
        assert_eq!(palette.shade(shade), Some(expected), "{name} {shade}");
    }
    let contrast = raw["ContrastText"].as_str().map(css).transpose()?;
    assert_eq!(palette.contrast_text, contrast, "{name} ContrastText");
    Ok(())
}

// ------------------------------------------------------------------ raw palettes

#[test]
fn primary_600_is_the_brand_blue() {
    assert_eq!(PRIMARY.shade(600), Some(Rgba8::rgb(0x0a84ff)));
}

#[test]
fn contrast_text_of_primary_is_white() {
    assert_eq!(PRIMARY.contrast_text, Some(Rgba8::rgb(0xffffff)));
    assert_eq!(PRIMARY.contrast_text, Some(WHITE));
}

#[test]
fn palettes_without_a_contrast_text_have_none() {
    assert_eq!(INFO.contrast_text, None);
    assert_eq!(GREY.contrast_text, None);
}

#[test]
fn a_shade_that_does_not_exist_is_none() {
    assert_eq!(PRIMARY.shade(0), None);
    assert_eq!(PRIMARY.shade(550), None);
    assert_eq!(PRIMARY.shade(1000), None);
}

#[test]
fn shades_run_from_lightest_to_darkest_in_the_array() {
    assert_eq!(PRIMARY.shades[0], Rgba8::rgb(0xf6fbff));
    assert_eq!(PRIMARY.shades[9], Rgba8::rgb(0x082139));
}

#[test]
fn common_colours_are_white_and_black() {
    assert_eq!(WHITE, Rgba8::new(255, 255, 255, 255));
    assert_eq!(BLACK, Rgba8::new(0, 0, 0, 255));
}

#[test]
fn every_raw_palette_matches_tokens_json() -> Res {
    let json = tokens()?;
    let all = [
        ("Primary", &PRIMARY),
        ("Secondary", &SECONDARY),
        ("Error", &ERROR),
        ("Warning", &WARNING),
        ("Success", &SUCCESS),
        ("Info", &INFO),
        ("Grey", &GREY),
    ];
    for (name, palette) in all {
        assert_palette(&json, name, palette)?;
    }
    Ok(())
}

// ------------------------------------------------------------------ semantic schemes

#[test]
fn light_and_dark_backgrounds_have_their_spec_values() {
    assert_eq!(LIGHT.background.default, Rgba8::rgb(0xF3F6F9));
    assert_eq!(LIGHT.background.paper, WHITE);
    assert_eq!(DARK.background.default, Rgba8::rgb(0x2C3039));
    assert_eq!(DARK.background.paper, Rgba8::rgb(0x4E5052));
}

#[test]
fn translucent_text_keeps_its_alpha() {
    // rgba(66, 66, 68, 0.64): alpha 0.64 * 255 = 163.2, a byte of 163.
    assert_eq!(LIGHT.text.secondary, Rgba8::new(66, 66, 68, 163));
    assert_eq!(DARK.text.primary, WHITE);
}

#[test]
fn dividers_are_translucent_in_both_schemes() {
    assert_eq!(LIGHT.divider, Rgba8::new(66, 66, 68, 31));
    assert_eq!(DARK.divider, Rgba8::new(255, 255, 255, 41));
}

#[test]
fn primary_tone_is_the_same_blue_in_the_light_scheme() {
    assert_eq!(LIGHT.primary.main, Rgba8::rgb(0x0a84ff));
    assert_eq!(LIGHT.primary.contrast_text, WHITE);
}

#[test]
fn schemes_match_tokens_json() -> Res {
    let json = tokens()?;
    for (name, scheme) in [("light", &LIGHT), ("dark", &DARK)] {
        let palette = &json["schemes"][name]["palette"];
        let tones = [
            ("primary", &scheme.primary),
            ("secondary", &scheme.secondary),
            ("error", &scheme.error),
            ("warning", &scheme.warning),
            ("success", &scheme.success),
            ("info", &scheme.info),
        ];
        for (tone, got) in tones {
            let want = &palette[tone];
            assert_eq!(got.light, css(text(want, "light")?)?, "{name} {tone}");
            assert_eq!(got.main, css(text(want, "main")?)?, "{name} {tone}");
            assert_eq!(got.dark, css(text(want, "dark")?)?, "{name} {tone}");
            let contrast = css(text(want, "contrastText")?)?;
            assert_eq!(got.contrast_text, contrast, "{name} {tone}");
        }
        assert_eq!(scheme.divider, css(text(palette, "divider")?)?, "{name}");
    }
    Ok(())
}

#[test]
fn text_background_and_border_colours_match_tokens_json() -> Res {
    let json = tokens()?;
    for (name, scheme) in [("light", &LIGHT), ("dark", &DARK)] {
        let palette = &json["schemes"][name]["palette"];
        let pairs = [
            (&palette["text"], "primary", scheme.text.primary),
            (&palette["text"], "secondary", scheme.text.secondary),
            (&palette["text"], "disabled", scheme.text.disabled),
            (&palette["text"], "icon", scheme.text.icon),
            (&palette["background"], "default", scheme.background.default),
            (&palette["background"], "paper", scheme.background.paper),
            (
                &palette["background"],
                "contrast",
                scheme.background.contrast,
            ),
            (&palette["border"], "main", scheme.border.main),
            (&palette["border"], "disabled", scheme.border.disabled),
            (&palette["border"], "ghost", scheme.border.ghost),
            (
                &palette["border"],
                "ghostDisabled",
                scheme.border.ghost_disabled,
            ),
        ];
        for (group, key, got) in pairs {
            assert_eq!(got, css(text(group, key)?)?, "{name} {key}");
        }
    }
    Ok(())
}

// ------------------------------------------------------------------ geometry

#[test]
fn spacing_unit_is_8_px() {
    assert_eq!(SPACING, 8.0);
}

#[test]
fn radii_follow_the_scale_of_twake_mui() {
    let got = [RADIUS_NONE, RADIUS_SM, RADIUS_MD, RADIUS_LG, RADIUS_PILL];
    assert_eq!(got, [0.0, 4.0, 8.0, 12.0, 100.0]);
    assert_eq!(SHAPE_RADIUS, 6.0);
}

#[test]
fn breakpoints_put_the_two_pane_threshold_at_769_px() {
    let got = [
        BREAKPOINT_XS,
        BREAKPOINT_SM,
        BREAKPOINT_MD,
        BREAKPOINT_LG,
        BREAKPOINT_XL,
    ];
    assert_eq!(got, [0.0, 544.0, 769.0, 1024.0, 1201.0]);
}

#[test]
fn geometry_matches_tokens_json() -> Res {
    let json = tokens()?;
    assert_eq!(number(&json["shape"], "borderRadius")?, SHAPE_RADIUS);
    let radii = [("none", RADIUS_NONE), ("sm", RADIUS_SM), ("md", RADIUS_MD)];
    for (key, got) in radii
        .into_iter()
        .chain([("lg", RADIUS_LG), ("pill", RADIUS_PILL)])
    {
        let px: f32 = text(&json["radius"], key)?.trim_end_matches("px").parse()?;
        assert_eq!(got, px, "radius {key}");
    }
    let marks = [
        ("xs", BREAKPOINT_XS),
        ("sm", BREAKPOINT_SM),
        ("md", BREAKPOINT_MD),
    ];
    for (key, got) in marks
        .into_iter()
        .chain([("lg", BREAKPOINT_LG), ("xl", BREAKPOINT_XL)])
    {
        assert_eq!(number(&json["breakpoints"], key)?, got, "breakpoint {key}");
    }
    Ok(())
}

// ------------------------------------------------------------------ typography

#[test]
fn body1_is_16_px_regular_with_a_ratio_line_height() {
    assert_eq!(TYPE_BODY1.font_size, 16.0);
    assert_eq!(TYPE_BODY1.font_weight, 400);
    assert_eq!(TYPE_BODY1.line_height, LineHeight::Ratio(1.313));
    assert_eq!(TYPE_BODY1.letter_spacing, -0.15);
}

#[test]
fn the_largest_heading_is_45_px_semi_bold() {
    assert_eq!(TYPE_H1.font_size, 45.0);
    assert_eq!(TYPE_H1.font_weight, 600);
}

#[test]
fn a_style_without_letter_spacing_has_zero() {
    assert_eq!(TYPE_H1.letter_spacing, 0.0);
}

#[test]
fn overline_has_a_pixel_line_height() {
    assert_eq!(TYPE_OVERLINE.line_height, LineHeight::Px(13.0));
    assert_eq!(TYPE_OVERLINE.font_size, 11.0);
}

fn assert_style(json: &Value, key: &str, got: &TypeSpec) -> Res {
    let want = &json["typography"][key];
    assert_eq!(got.font_size, number(want, "fontSize")?, "{key} size");
    assert_eq!(
        f64::from(got.font_weight),
        want["fontWeight"].as_f64().unwrap_or(-1.0)
    );
    let line_height = match want["lineHeight"].as_str() {
        Some(px) => LineHeight::Px(px.trim_end_matches("px").parse()?),
        None => LineHeight::Ratio(number(want, "lineHeight")?),
    };
    assert_eq!(got.line_height, line_height, "{key} line height");
    let spacing: f32 = match want["letterSpacing"].as_str() {
        Some(px) => px.trim_end_matches("px").parse()?,
        None => 0.0,
    };
    assert!(
        (got.letter_spacing - spacing).abs() < 1e-6,
        "{key} letter spacing"
    );
    Ok(())
}

#[test]
fn every_type_style_matches_tokens_json() -> Res {
    let json = tokens()?;
    let all = [
        ("h1", &TYPE_H1),
        ("h2", &TYPE_H2),
        ("h3", &TYPE_H3),
        ("h4", &TYPE_H4),
        ("h5", &TYPE_H5),
        ("h6", &TYPE_H6),
        ("subtitle1", &TYPE_SUBTITLE1),
        ("subtitle2", &TYPE_SUBTITLE2),
        ("body1", &TYPE_BODY1),
        ("body2", &TYPE_BODY2),
        ("body3", &TYPE_BODY3),
        ("button", &TYPE_BUTTON),
        ("caption", &TYPE_CAPTION),
        ("overline", &TYPE_OVERLINE),
    ];
    for (key, style) in all {
        assert_style(&json, key, style)?;
    }
    Ok(())
}
