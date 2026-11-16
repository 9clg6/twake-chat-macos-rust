#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
//! Once registered, Inter is a family of the text system and each weight resolves to Inter itself,
//! not to a fallback. Split from `fonts.rs`: `current_platform` needs the main thread on macOS, which
//! the libtest threads are not, so this target has its own `main` (`harness = false`).

use twake_gpui::{Font, FontWeight, TextSystem, font};
use twake_ui_kit::fonts::{FAMILY, embedded};

const WEIGHTS: [u16; 4] = [400, 500, 600, 700];

fn registered_text_system() -> TextSystem {
    let platform = twake_gpui::gpui_platform::current_platform(true);
    let text = TextSystem::new(platform.text_system());
    let blobs: Vec<&'static [u8]> = embedded().iter().map(|f| f.data).collect();
    twake_gpui::register_fonts(&text, &blobs).expect("the embedded fonts register");
    text
}

fn inter(weight: u16) -> Font {
    let mut f = font(FAMILY);
    f.weight = FontWeight(f32::from(weight));
    f
}

fn inter_is_a_family_of_the_text_system_once_registered(text: &TextSystem) {
    assert!(text.all_font_names().iter().any(|n| n == "Inter"));
}

fn a_layout_in_inter_at_each_weight_resolves_to_inter_and_not_to_a_fallback(text: &TextSystem) {
    let mut ids = Vec::new();
    for weight in WEIGHTS {
        let id = text.resolve_font(&inter(weight));
        // `get_font_for_id` knows a font only when it was found directly, never for a fallback.
        let found = text
            .get_font_for_id(id)
            .unwrap_or_else(|| panic!("weight {weight} fell back to another font"));
        assert_eq!(found.family.as_ref(), "Inter", "weight {weight}");
        ids.push(id);
    }
    ids.sort_by_key(|id| id.0);
    ids.dedup();
    assert_eq!(
        ids.len(),
        4,
        "each weight is its own face, not one face stretched"
    );
}

fn main() {
    let text = registered_text_system();
    inter_is_a_family_of_the_text_system_once_registered(&text);
    println!("ok: inter_is_a_family_of_the_text_system_once_registered");
    a_layout_in_inter_at_each_weight_resolves_to_inter_and_not_to_a_fallback(&text);
    println!("ok: a_layout_in_inter_at_each_weight_resolves_to_inter_and_not_to_a_fallback");
}
