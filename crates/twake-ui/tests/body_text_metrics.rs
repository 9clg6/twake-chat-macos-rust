#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
//! The height of a bubble is estimated before its text is measured, from an average character
//! width. With Inter embedded (15 px body text), that width must be Inter's, not the system
//! font's. Public API under test (to write): `twake_ui::bubble_style::BODY_CHAR_WIDTH_PX: f32`
//! (the former private `AVERAGE_CHAR_WIDTH` of `view/timeline.rs`, 7.0) and
//! `twake_ui_kit::fonts::embedded()` (see `twake-ui-kit/tests/fonts.rs`).

use twake_ui::bubble_style::BODY_CHAR_WIDTH_PX;
use twake_ui_kit::fonts::embedded;

const BODY_PX: f32 = 15.0;
/// Typical chat prose, Latin script, mixed case, digits and punctuation, English and French.
const SAMPLES: [&str; 4] = [
    "Hello, are you free for a quick call this afternoon? I pushed the fix to the branch.",
    "The meeting moved to 3 pm; could someone update the invitation and tell Ana?",
    "Salut, tu peux regarder la demande de fusion avant midi ? Merci beaucoup !",
    "Thanks! I will review it tonight and send my comments tomorrow morning.",
];

/// Mean advance, in pixels at 15 px, of the characters of `SAMPLES` in the face of `weight`.
fn measured_average(weight: u16) -> f32 {
    let font = embedded()
        .iter()
        .find(|f| f.weight == weight)
        .expect("the weight is embedded");
    let face = ttf_parser::Face::parse(font.data, 0).expect("valid TTF");
    let scale = BODY_PX / f32::from(face.units_per_em());
    let (mut total, mut count) = (0.0_f32, 0_u32);
    for ch in SAMPLES.iter().flat_map(|s| s.chars()) {
        let glyph = face
            .glyph_index(ch)
            .unwrap_or_else(|| panic!("no glyph for {ch:?}"));
        let advance = face.glyph_hor_advance(glyph).expect("advance");
        total += f32::from(advance) * scale;
        count += 1;
    }
    total / count as f32
}

#[test]
fn the_estimated_char_width_is_the_measured_one_of_inter_regular_15_px() {
    let measured = measured_average(400);
    let error = (BODY_CHAR_WIDTH_PX - measured).abs() / measured;
    assert!(
        error <= 0.03,
        "estimate {BODY_CHAR_WIDTH_PX} px, Inter Regular measures {measured:.2} px ({:.1} % off)",
        error * 100.0
    );
}
