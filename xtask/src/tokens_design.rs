//! The design tokens beyond the colours of `cargo xtask tokens`: shadows, `zIndex`, transitions, the
//! font family and the avatar gradients (A012, lifted by A026).

use serde_json::{Map, Value};

use crate::common::{Res, fail, object, string};
use crate::tokens::color;

/// `mobileStepper` reads as `MOBILE_STEPPER`.
fn upper_snake(key: &str) -> String {
    let mut text = String::new();
    for c in key.chars() {
        if c.is_uppercase() {
            text.push('_');
        }
        text.extend(c.to_uppercase());
    }
    text
}

/// The entries by key: `serde_json` keeps the file order when another crate of the workspace turns on
/// `preserve_order`, and `generated.rs` must not depend on that.
fn sorted(map: &Map<String, Value>) -> Vec<(&String, &Value)> {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_by_key(|(key, _)| *key);
    entries
}

fn section<'a>(root: &'a Map<String, Value>, name: &str) -> Res<Option<&'a Map<String, Value>>> {
    root.get(name).map(|value| object(value, name)).transpose()
}

fn whole(value: &Value, what: &str) -> Res<u32> {
    match value.as_u64().and_then(|n| u32::try_from(n).ok()) {
        Some(number) => Ok(number),
        None => fail(format!("{what} is not a whole number")),
    }
}

fn integers(map: &Map<String, Value>, prefix: &str, what: &str) -> Res<String> {
    let mut text = String::new();
    for (key, value) in sorted(map) {
        let number = whole(value, &format!("{what} {key}"))?;
        text.push_str(&format!(
            "pub const {prefix}{}: u32 = {number};\n\n",
            upper_snake(key)
        ));
    }
    Ok(text)
}

/// `cubic-bezier(0.4, 0, 0.2, 1)` as four numbers.
fn bezier(text: &str) -> Option<[f64; 4]> {
    let inner = text.strip_prefix("cubic-bezier(")?.strip_suffix(')')?;
    let numbers: Vec<f64> = inner
        .split(',')
        .map(|part| part.trim().parse().ok())
        .collect::<Option<_>>()?;
    numbers.try_into().ok()
}

fn easings(map: &Map<String, Value>) -> Res<String> {
    let mut text = String::new();
    for (key, value) in sorted(map) {
        let raw = string(value, &format!("easing {key}"))?;
        let Some([x1, y1, x2, y2]) = bezier(raw) else {
            return fail(format!("easing {key}: unsupported curve {raw:?}"));
        };
        text.push_str(&format!(
            "pub const {}: [f32; 4] = [{x1:?}, {y1:?}, {x2:?}, {y2:?}];\n\n",
            upper_snake(key)
        ));
    }
    Ok(text)
}

/// The CSS text of each elevation of one scheme.
pub(crate) fn shadows(mode: &str, raw: &Map<String, Value>) -> Res<String> {
    let Some(list) = raw.get("shadows") else {
        return Ok(String::new());
    };
    let Some(items) = list.as_array() else {
        return fail(format!("scheme {mode} shadows is not a list"));
    };
    let mut lines = String::new();
    for item in items {
        lines.push_str(&format!(
            "    {:?},\n",
            string(item, &format!("scheme {mode} shadow"))?
        ));
    }
    Ok(format!(
        "pub const SHADOWS_{}: [&str; {}] = [\n{lines}];\n\n",
        mode.to_uppercase(),
        items.len()
    ))
}

fn z_indexes(root: &Map<String, Value>) -> Res<String> {
    let map = section(root, "zIndex")?;
    map.map_or(Ok(String::new()), |map| integers(map, "Z_", "zIndex"))
}

fn transitions(root: &Map<String, Value>) -> Res<String> {
    let Some(map) = section(root, "transitions")? else {
        return Ok(String::new());
    };
    let part = |key: &str| map.get(key).map(|value| object(value, key)).transpose();
    let durations =
        part("duration")?.map_or(Ok(String::new()), |m| integers(m, "DURATION_", "duration"))?;
    let curves = part("easing")?.map_or(Ok(String::new()), easings)?;
    Ok(durations + &curves)
}

fn font_family(root: &Map<String, Value>) -> Res<String> {
    let Some(family) = section(root, "typographyBase")?.and_then(|map| map.get("fontFamily"))
    else {
        return Ok(String::new());
    };
    Ok(format!(
        "pub const FONT_FAMILY: &str = {:?};\n\n",
        string(family, "fontFamily")?
    ))
}

/// `zIndex`, `transitions` and the font family.
pub(crate) fn theme(root: &Map<String, Value>) -> Res<String> {
    Ok([z_indexes(root)?, transitions(root)?, font_family(root)?].concat())
}

/// `linear-gradient(136deg, #F8D280 14.84%, #F2AC69 96.03%)` as a `Gradient`.
fn gradient(text: &str) -> Option<String> {
    let inner = text.strip_prefix("linear-gradient(")?.strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    let [angle, from, to] = parts.as_slice() else {
        return None;
    };
    let angle: f64 = angle.strip_suffix("deg")?.parse().ok()?;
    let stop = |part: &str| -> Option<String> {
        let (hex, at) = part.split_once(' ')?;
        let at: f64 = at.trim().strip_suffix('%')?.parse().ok()?;
        Some(format!(
            "GradientStop {{ color: {}, at_percent: {at:?} }}",
            color(hex).ok()?
        ))
    };
    Some(format!(
        "Gradient {{ angle_deg: {angle:?}, from: {}, to: {} }}",
        stop(from)?,
        stop(to)?
    ))
}

/// `avatarColors`: the names in order of `supportedColors` and their gradients.
pub(crate) fn avatars(root: &Map<String, Value>) -> Res<String> {
    let Some(map) = section(root, "avatarColors")? else {
        return Ok(String::new());
    };
    let (Some(names), Some(mapping)) = (map.get("supportedColors"), map.get("colorMapping")) else {
        return fail("avatarColors needs supportedColors and colorMapping".to_string());
    };
    let (mapping, Some(names)) = (object(mapping, "colorMapping")?, names.as_array()) else {
        return fail("supportedColors is not a list".to_string());
    };
    let (mut quoted, mut gradients) = (Vec::new(), String::new());
    for name in names {
        let name = string(name, "supportedColors item")?;
        let Some(value) = mapping.get(name) else {
            return fail(format!("colorMapping has no {name}"));
        };
        let raw = string(value, &format!("colorMapping {name}"))?;
        let Some(parsed) = gradient(raw) else {
            return fail(format!("colorMapping {name}: unsupported gradient {raw:?}"));
        };
        quoted.push(format!("{name:?}"));
        gradients.push_str(&format!("    {parsed},\n"));
    }
    let count = quoted.len();
    Ok(format!(
        "pub const AVATAR_COLOR_NAMES: [&str; {count}] = [{}];\n\npub const AVATAR_GRADIENTS: [Gradient; {count}] = [\n{gradients}];\n\n",
        quoted.join(", ")
    ))
}
