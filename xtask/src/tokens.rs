//! `cargo xtask tokens`: turns `docs/spec/tokens.json` into `crates/twake-tokens/src/generated.rs`.

use std::fmt;

use serde_json::{Map, Value};

/// Why the tokens could not be generated; the text says what is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

type Res<T> = Result<T, Error>;

fn fail<T>(message: String) -> Res<T> {
    Err(Error(message))
}

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
const GROUPS: [(&str, &[(&str, &str)]); 3] = [
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
    let alpha = match parts.get(3) {
        Some(a) => (a.parse::<f64>().ok()? * 255.0).round() as u8,
        None => 255,
    };
    Some([channel(0)?, channel(1)?, channel(2)?, alpha])
}

fn color(text: &str) -> Res<String> {
    match hex(text).or_else(|| functional(text)) {
        Some([r, g, b, a]) => Ok(format!("Rgba8::new({r}, {g}, {b}, {a})")),
        None => fail(format!("unsupported colour {text:?}")),
    }
}

fn string<'a>(value: &'a Value, what: &str) -> Res<&'a str> {
    match value.as_str() {
        Some(text) => Ok(text),
        None => fail(format!("{what} is not a string")),
    }
}

fn object<'a>(value: &'a Value, what: &str) -> Res<&'a Map<String, Value>> {
    match value.as_object() {
        Some(map) => Ok(map),
        None => fail(format!("{what} is not an object")),
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
        _ => "Tone",
    }
}

fn scheme(mode: &str, raw: &Value) -> Res<String> {
    let map = object(raw, &format!("scheme {mode}"))?;
    let palette = match map.get("palette") {
        Some(value) => object(value, &format!("scheme {mode} palette"))?,
        None => return fail(format!("scheme {mode} has no palette")),
    };
    let mut text = format!("pub const {}: Scheme = Scheme {{\n", mode.to_uppercase());
    for tone in TONES {
        text.push_str(&group(palette, tone, &TONE_KEYS, mode)?);
    }
    for (name, keys) in GROUPS {
        text.push_str(&group(palette, name, keys, mode)?);
    }
    let divider = colour_of(palette, "divider", &format!("scheme {mode}"))?;
    text.push_str(&format!("    divider: {divider},\n}};\n\n"));
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

fn type_spec(style: &str, raw: &Value) -> Res<String> {
    let what = format!("typography {style}");
    let map = object(raw, &what)?;
    let field = |key: &str| match map.get(key) {
        Some(value) => Ok(value),
        None => fail(format!("{what} has no {key}")),
    };
    let size = px(field("fontSize")?, &what)?;
    let weight = match field("fontWeight")?.as_u64() {
        Some(weight) => weight,
        None => return fail(format!("{what}: fontWeight is not a whole number")),
    };
    let height = field("lineHeight")?;
    let kind = if height.is_string() { "Px" } else { "Ratio" };
    let line = format!("LineHeight::{kind}({:?})", px(height, &what)?);
    let spacing = match map.get("letterSpacing") {
        Some(value) => px(value, &what)?,
        None => 0.0,
    };
    Ok(format!(
        "pub const TYPE_{}: TypeSpec = TypeSpec {{\n    font_size: {size:?},\n    font_weight: {weight},\n    line_height: {line},\n    letter_spacing: {spacing:?},\n}};\n\n",
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

/// The text of `generated.rs` for the text of `tokens.json`.
pub fn generate(json: &str) -> Result<String, Error> {
    let value: Value = match serde_json::from_str(json) {
        Ok(value) => value,
        Err(error) => return fail(format!("invalid JSON: {error}")),
    };
    let root = object(&value, "tokens.json")?;
    let version = match root.get("twakeMuiVersion") {
        Some(version) => string(version, "twakeMuiVersion")?,
        None => return fail("tokens.json has no twakeMuiVersion".to_string()),
    };
    let body = [
        palettes(root)?,
        schemes(root)?,
        single(root, "spacing", "unit", "SPACING")?,
        single(root, "shape", "borderRadius", "SHAPE_RADIUS")?,
        lengths(root, "radius", &RADII, "RADIUS_")?,
        lengths(root, "breakpoints", &BREAKPOINTS, "BREAKPOINT_")?,
        typography(root)?,
    ]
    .concat();
    Ok(format!(
        "// @generated by `cargo xtask tokens` from docs/spec/tokens.json (twake-mui {version}). Do not edit.\n#![allow(missing_docs)]\n\nuse super::*;\n\n{}\n",
        body.trim_end()
    ))
}
