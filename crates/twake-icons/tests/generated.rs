//! The icons generated from `docs/spec/icons.json` by `cargo xtask icons`.
//! Need: the icon set is data, not hand-copied SVG: every icon of the JSON is in the crate with
//! the same id, viewBox and body, in an order `by_id` can search.
//! Public items used: `docs/dev/icons-api-sketch.md`.

use std::error::Error;

use serde_json::{Map, Value};
use twake_icons::{ALL, ViewBox, by_id};

type Res = Result<(), Box<dyn Error>>;

fn icons() -> Result<Map<String, Value>, Box<dyn Error>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/spec/icons.json");
    let json: Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let found = json["icons"].as_object().ok_or("icons is not an object")?;
    Ok(found.clone())
}

fn view_box(text: &str) -> Result<ViewBox, Box<dyn Error>> {
    let numbers = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|part| !part.is_empty())
        .map(str::parse::<f32>)
        .collect::<Result<Vec<_>, _>>()?;
    match numbers[..] {
        [min_x, min_y, width, height] => Ok(ViewBox {
            min_x,
            min_y,
            width,
            height,
        }),
        _ => Err(format!("viewBox {text:?} is not four numbers").into()),
    }
}

fn field<'a>(icon: &'a Value, key: &str) -> Result<&'a str, Box<dyn Error>> {
    icon[key]
        .as_str()
        .ok_or_else(|| format!("{key} is not a string in icons.json").into())
}

#[test]
fn there_is_one_icon_per_entry_of_the_json() -> Res {
    let json = icons()?;
    assert!(!json.is_empty());
    assert_eq!(ALL.len(), json.len());
    Ok(())
}

#[test]
fn every_icon_of_the_json_is_in_the_crate_with_its_viewbox() -> Res {
    for (id, icon) in icons()? {
        let found = by_id(&id).ok_or_else(|| format!("{id} is missing"))?;
        assert_eq!(found.id, id);
        assert_eq!(found.view_box, view_box(field(&icon, "viewBox")?)?, "{id}");
    }
    Ok(())
}

#[test]
fn every_icon_of_the_json_keeps_its_body_byte_for_byte() -> Res {
    for (id, icon) in icons()? {
        let found = by_id(&id).ok_or_else(|| format!("{id} is missing"))?;
        assert_eq!(found.body, field(&icon, "body")?, "{id}");
    }
    Ok(())
}

#[test]
fn the_crate_holds_no_icon_the_json_does_not() -> Res {
    let json = icons()?;
    for icon in ALL {
        assert!(
            json.contains_key(icon.id),
            "{} is not in icons.json",
            icon.id
        );
    }
    Ok(())
}

#[test]
fn all_is_sorted_by_id_without_duplicates() {
    for pair in ALL.windows(2) {
        assert!(
            pair[0].id < pair[1].id,
            "{} then {}",
            pair[0].id,
            pair[1].id
        );
    }
}

#[test]
fn ids_are_kebab_case() {
    for icon in ALL {
        let ok = icon.id.split('-').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        });
        assert!(ok, "{}", icon.id);
    }
}

#[test]
fn every_icon_has_a_drawable_viewbox_and_a_body() {
    for icon in ALL {
        assert!(
            icon.view_box.width > 0.0 && icon.view_box.height > 0.0,
            "{}",
            icon.id
        );
        assert!(!icon.body.is_empty(), "{}", icon.id);
    }
}
