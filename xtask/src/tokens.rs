//! `cargo xtask tokens`: turns `docs/spec/tokens.json` into `crates/twake-tokens/src/generated.rs`.

use serde_json::{Map, Value};

use crate::common::{Res, fail, object, root as parse_root, string};
use crate::tokens_design::{avatars, opacities, shadows, theme};

pub use crate::common::Error;

const SHADES: [&str; 10] = [
    "50", "100", "200", "300", "400", "500", "600", "700", "800", "900",
];
const PALETTES: [&str; 7] = [
    "Primary",
    "Secondary",
    "Error",
    "Warning",
    "Success",
    "Info",
    "Grey",
];
const TONES: [&str; 6] = [
    "primary",
    "secondary",
    "error",
    "warning",
    "success",
    "info",
];
const TONE_KEYS: [(&str, &str); 4] = [
    ("light", "light"),
    ("main", "main"),
    ("dark", "dark"),
    ("contrastText", "contrast_text"),
];
const GROUPS: [(&str, &[(&str, &str)]); 4] = [
    (
        "action",
        &[
            ("active", "active"),
            ("hover", "hover"),
            ("selected", "selected"),
            ("disabled", "disabled"),
            ("disabledBackground", "disabled_background"),
            ("focus", "focus"),
        ],
    ),
    (
        "text",
        &[
            ("primary", "primary"),
            ("secondary", "secondary"),
            ("disabled", "disabled"),
            ("icon", "icon"),
        ],
    ),
    (
        "background",
        &[
            ("default", "default"),
            ("paper", "paper"),
            ("contrast", "contrast"),
        ],
    ),
    (
        "border",
        &[
            ("main", "main"),
            ("disabled", "disabled"),
            ("ghost", "ghost"),
            ("ghostDisabled", "ghost_disabled"),
        ],
    ),
];
const RADII: [&str; 5] = ["none", "sm", "md", "lg", "pill"];
const BREAKPOINTS: [&str; 5] = ["xs", "sm", "md", "lg", "xl"];
const STYLES: [&str; 14] = [
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "subtitle1",
    "subtitle2",
    "body1",
    "body2",
    "body3",
    "button",
    "caption",
    "overline",
];

fn hex(text: &str) -> Option<[u8; 4]> {
    let digits: Vec<u8> = text
        .strip_prefix('#')?
        .chars()
        .map(|c| c.to_digit(16).map(|d| d as u8))
        .collect::<Option<_>>()?;
    let pair = |i: usize| digits[i] * 16 + digits[i + 1];
    match digits.len() {
        3 => Some([digits[0] * 17, digits[1] * 17, digits[2] * 17, 255]),
        6 => Some([pair(0), pair(2), pair(4), 255]),
        8 => Some([pair(0), pair(2), pair(4), pair(6)]),
        _ => None,
    }
}

fn functional(text: &str) -> Option<[u8; 4]> {
    let inner = text
        .strip_prefix("rgba(")
        .or_else(|| text.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if !(3..=4).contains(&parts.len()) {
        return None;
    }
    let channel = |i: usize| parts[i].parse::<u8>().ok();
    Some([
        channel(0)?,
        channel(1)?,
        channel(2)?,
        alpha(parts.get(3).copied())?,
    ])
}

fn alpha(text: Option<&str>) -> Option<u8> {
    match text {
        Some(a) => Some((a.parse::<f64>().ok()? * 255.0).round() as u8),
        None => Some(255),
    }
}

pub(crate) fn color(text: &str) -> Res<String> {
    match hex(text).or_else(|| functional(text)) {
        Some([r, g, b, a]) => Ok(format!("Rgba8::new({r}, {g}, {b}, {a})")),
        None => fail(format!("unsupported colour {text:?}")),
    }
}

fn colour_of(map: &Map<String, Value>, key: &str, what: &str) -> Res<String> {
    match map.get(key) {
        Some(value) => color(string(value, &format!("{what} {key}"))?),
        None => fail(format!("{what} has no {key}")),
    }
}

fn palette(name: &str, raw: &Value) -> Res<String> {
    let map = object(raw, name)?;
    let mut text = format!(
        "pub const {}: Palette = Palette {{\n    shades: [\n",
        name.to_uppercase()
    );
    for shade in SHADES {
        let line = colour_of(map, shade, &format!("palette {name} shade"))?;
        text.push_str(&format!("        {line},\n"));
    }
    let contrast = match map.get("ContrastText") {
        Some(value) => format!("Some({})", color(string(value, "ContrastText")?)?),
        None => "None".to_string(),
    };
    text.push_str(&format!("    ],\n    contrast_text: {contrast},\n}};\n\n"));
    Ok(text)
}

fn common(raw: &Value) -> Res<String> {
    let map = object(raw, "Common")?;
    let mut text = String::new();
    for name in ["white", "black"] {
        let line = colour_of(map, name, "Common")?;
        text.push_str(&format!(
            "pub const {}: Rgba8 = {line};\n\n",
            name.to_uppercase()
        ));
    }
    Ok(text)
}

fn group(map: &Map<String, Value>, name: &str, keys: &[(&str, &str)], mode: &str) -> Res<String> {
    let inner = match map.get(name) {
        Some(value) => object(value, &format!("scheme {mode} {name}"))?,
        None => return fail(format!("scheme {mode} has no {name}")),
    };
    let mut text = format!("    {name}: {} {{\n", struct_of(name));
    for (json, field) in keys {
        let line = colour_of(inner, json, &format!("scheme {mode} {name}"))?;
        text.push_str(&format!("        {field}: {line},\n"));
    }
    text.push_str("    },\n");
    Ok(text)
}

fn struct_of(name: &str) -> &'static str {
    match name {
        "text" => "TextColors",
        "background" => "BackgroundColors",
        "border" => "BorderColors",
        "action" => "ActionColors",
        _ => "Tone",
    }
}

fn scheme_palette<'a>(mode: &str, raw: &'a Value) -> Res<&'a Map<String, Value>> {
    match object(raw, &format!("scheme {mode}"))?.get("palette") {
        Some(value) => object(value, &format!("scheme {mode} palette")),
        None => fail(format!("scheme {mode} has no palette")),
    }
}

fn scheme(mode: &str, raw: &Value) -> Res<String> {
    let palette = scheme_palette(mode, raw)?;
    let mut text = format!("pub const {}: Scheme = Scheme {{\n", mode.to_uppercase());
    for tone in TONES {
        text.push_str(&group(palette, tone, &TONE_KEYS, mode)?);
    }
    for (name, keys) in GROUPS {
        text.push_str(&group(palette, name, keys, mode)?);
    }
    let divider = colour_of(palette, "divider", &format!("scheme {mode}"))?;
    text.push_str(&format!("    divider: {divider},\n}};\n\n"));
    text.push_str(&opacities(mode, palette)?);
    text.push_str(&shadows(mode, object(raw, "scheme")?)?);
    Ok(text)
}

/// A length in pixels: a number, `"4px"`, `"0"` or `"var(--x, 8px)"`.
fn px(value: &Value, what: &str) -> Res<f64> {
    if let Some(number) = value.as_f64() {
        return Ok(number);
    }
    let text = string(value, what)?;
    let tail = match text.strip_prefix("var(") {
        Some(rest) => rest
            .rsplit(',')
            .next()
            .unwrap_or(rest)
            .trim_end_matches(')'),
        None => text,
    };
    match tail.trim().trim_end_matches("px").trim().parse() {
        Ok(number) => Ok(number),
        Err(_) => fail(format!("{what}: unsupported length {text:?}")),
    }
}

fn constant(name: &str, value: f64) -> String {
    format!("pub const {name}: f32 = {value:?};\n\n")
}

fn lengths(root: &Map<String, Value>, section: &str, keys: &[&str], prefix: &str) -> Res<String> {
    let Some(raw) = root.get(section) else {
        return Ok(String::new());
    };
    let map = object(raw, section)?;
    let mut text = String::new();
    for key in keys {
        if let Some(value) = map.get(*key) {
            let value = px(value, &format!("{section} {key}"))?;
            text.push_str(&constant(&format!("{prefix}{}", key.to_uppercase()), value));
        }
    }
    Ok(text)
}

fn single(root: &Map<String, Value>, section: &str, key: &str, name: &str) -> Res<String> {
    let Some(raw) = root.get(section) else {
        return Ok(String::new());
    };
    match object(raw, section)?.get(key) {
        Some(value) => Ok(constant(name, px(value, &format!("{section} {key}"))?)),
        None => fail(format!("{section} has no {key}")),
    }
}

fn need<'a>(map: &'a Map<String, Value>, key: &str, what: &str) -> Res<&'a Value> {
    match map.get(key) {
        Some(value) => Ok(value),
        None => fail(format!("{what} has no {key}")),
    }
}

fn line_height(height: &Value, what: &str) -> Res<String> {
    let kind = if height.is_string() { "Px" } else { "Ratio" };
    Ok(format!("LineHeight::{kind}({:?})", px(height, what)?))
}

fn weight(value: &Value, what: &str) -> Res<u64> {
    match value.as_u64() {
        Some(weight) => Ok(weight),
        None => fail(format!("{what}: fontWeight is not a whole number")),
    }
}

fn spacing(value: Option<&Value>, what: &str) -> Res<f64> {
    match value {
        Some(value) => px(value, what),
        None => Ok(0.0),
    }
}

fn type_fields(map: &Map<String, Value>, what: &str) -> Res<String> {
    let size = px(need(map, "fontSize", what)?, what)?;
    let weight = weight(need(map, "fontWeight", what)?, what)?;
    let line = line_height(need(map, "lineHeight", what)?, what)?;
    let spacing = spacing(map.get("letterSpacing"), what)?;
    Ok(format!(
        "    font_size: {size:?},\n    font_weight: {weight},\n    line_height: {line},\n    letter_spacing: {spacing:?},\n"
    ))
}

fn type_spec(style: &str, raw: &Value) -> Res<String> {
    let what = format!("typography {style}");
    let fields = type_fields(object(raw, &what)?, &what)?;
    Ok(format!(
        "pub const TYPE_{}: TypeSpec = TypeSpec {{\n{fields}}};\n\n",
        style.to_uppercase()
    ))
}

fn typography(root: &Map<String, Value>) -> Res<String> {
    let Some(raw) = root.get("typography") else {
        return Ok(String::new());
    };
    let map = object(raw, "typography")?;
    let mut text = String::new();
    for style in STYLES {
        if let Some(value) = map.get(style) {
            text.push_str(&type_spec(style, value)?);
        }
    }
    Ok(text)
}

fn palettes(root: &Map<String, Value>) -> Res<String> {
    let Some(raw) = root.get("paletteRaw") else {
        return fail("tokens.json has no paletteRaw".to_string());
    };
    let map = object(raw, "paletteRaw")?;
    let mut text = String::new();
    for name in PALETTES {
        if let Some(value) = map.get(name) {
            text.push_str(&palette(name, value)?);
        }
    }
    if let Some(value) = map.get("Common") {
        text.push_str(&common(value)?);
    }
    Ok(text)
}

fn schemes(root: &Map<String, Value>) -> Res<String> {
    let Some(raw) = root.get("schemes") else {
        return Ok(String::new());
    };
    let map = object(raw, "schemes")?;
    let mut text = String::new();
    for mode in ["light", "dark"] {
        if let Some(value) = map.get(mode) {
            text.push_str(&scheme(mode, value)?);
        }
    }
    Ok(text)
}

fn version(root: &Map<String, Value>) -> Res<&str> {
    match root.get("twakeMuiVersion") {
        Some(version) => string(version, "twakeMuiVersion"),
        None => fail("tokens.json has no twakeMuiVersion".to_string()),
    }
}

/// Lengths: spacing, shape, radii and breakpoints.
fn sizes(root: &Map<String, Value>) -> Res<String> {
    Ok([
        single(root, "spacing", "unit", "SPACING")?,
        single(root, "shape", "borderRadius", "SHAPE_RADIUS")?,
        lengths(root, "radius", &RADII, "RADIUS_")?,
        lengths(root, "breakpoints", &BREAKPOINTS, "BREAKPOINT_")?,
    ]
    .concat())
}

fn body(root: &Map<String, Value>) -> Res<String> {
    Ok([
        palettes(root)?,
        schemes(root)?,
        sizes(root)?,
        typography(root)?,
        theme(root)?,
        avatars(root)?,
    ]
    .concat())
}

/// The text of `generated.rs` for the text of `tokens.json`.
pub fn generate(json: &str) -> Result<String, Error> {
    let root = &parse_root(json, "tokens.json")?;
    let version = version(root)?;
    let body = body(root)?;
    Ok(format!(
        "// @generated by `cargo xtask tokens` from docs/spec/tokens.json (twake-mui {version}). Do not edit.\n#![allow(missing_docs)]\n\nuse super::*;\n\n{}\n",
        body.trim_end()
    ))
}
