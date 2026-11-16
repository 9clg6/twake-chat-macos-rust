#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]
//! The app embeds Inter so that it looks the same on macOS, Windows and Linux.
//! Need: the design system's `FONT_FAMILY` starts with "Inter" (docs/spec/tokens.json); Inter 4.1,
//! SIL OFL 1.1, from https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip.
//! Public API under test (to write): `twake_ui_kit::fonts::{FAMILY, LICENSE, EmbeddedFont, embedded}`
//! with `EmbeddedFont { file_name: &'static str, weight: u16, data: &'static [u8] }`, and
//! `twake_gpui::register_fonts(&TextSystem, &[&'static [u8]]) -> Result<(), impl Debug>`.

use std::fs;
use std::path::PathBuf;

use sha2::{Digest, Sha256};
use twake_tokens::FONT_FAMILY;
use twake_ui_kit::fonts::{FAMILY, LICENSE, embedded};

const WEIGHTS: [u16; 4] = [400, 500, 600, 700];
const FILES: [&str; 4] = [
    "Inter-Regular.ttf",
    "Inter-Medium.ttf",
    "Inter-SemiBold.ttf",
    "Inter-Bold.ttf",
];
/// Hash of `Inter-4.1.zip`, recorded in `assets/fonts/SOURCE.txt` next to its URL.
const ZIP_SHA256: &str = "9883fdd4a49d4fb66bd8177ba6625ef9a64aa45899767dde3d36aa425756b11e";

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/fonts")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn first_family(css: &str) -> String {
    css.split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches('"')
        .to_owned()
}

#[test]
fn the_embedded_family_is_the_first_family_of_the_design_system() {
    assert_eq!(FAMILY, first_family(FONT_FAMILY));
    assert_eq!(FAMILY, "Inter");
}

#[test]
fn four_weights_are_embedded_400_500_600_700() {
    let weights: Vec<u16> = embedded().iter().map(|f| f.weight).collect();
    assert_eq!(weights, WEIGHTS);
}

#[test]
fn each_embedded_font_is_a_real_truetype_file_of_its_weight() {
    for font in embedded() {
        assert!(!font.data.is_empty(), "{} is empty", font.file_name);
        // 0x00010000: a TrueType (glyf) outline font, what Inter's static TTF are.
        assert_eq!(&font.data[..4], &[0, 1, 0, 0], "{} header", font.file_name);
        let face = ttf_parser::Face::parse(font.data, 0)
            .unwrap_or_else(|e| panic!("{} does not parse: {e}", font.file_name));
        assert_eq!(face.weight().to_number(), font.weight, "{}", font.file_name);
        assert!(
            face.number_of_glyphs() > 1000,
            "{} is a stub",
            font.file_name
        );
    }
}

#[test]
fn the_licence_text_is_embedded_and_is_the_sil_ofl() {
    assert!(LICENSE.contains("SIL Open Font License"));
    assert!(LICENSE.contains("Version 1.1"));
}

#[test]
fn the_licence_and_the_provenance_ship_next_to_the_fonts() {
    let dir = fonts_dir();
    let licence = fs::read_to_string(dir.join("LICENSE.txt")).expect("LICENSE.txt next to fonts");
    assert_eq!(licence, LICENSE);
    let source = fs::read_to_string(dir.join("SOURCE.txt")).expect("SOURCE.txt next to fonts");
    assert!(source.contains("https://github.com/rsms/inter/releases/download/v4.1/Inter-4.1.zip"));
    assert!(source.contains(ZIP_SHA256));
}

#[test]
fn the_font_files_match_the_pinned_sha256_list() {
    // `SHA256SUMS` is in `sha256sum` format: "<64 hex>  <file>" for the four fonts and LICENSE.txt.
    let dir = fonts_dir();
    let sums = fs::read_to_string(dir.join("SHA256SUMS")).expect("SHA256SUMS next to fonts");
    let listed: Vec<(&str, &str)> = sums
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let (hash, name) = l.split_once("  ").expect("`hash  name` lines");
            (name.trim(), hash.trim())
        })
        .collect();
    let mut expected: Vec<&str> = FILES.to_vec();
    expected.push("LICENSE.txt");
    let mut names: Vec<&str> = listed.iter().map(|(n, _)| *n).collect();
    names.sort_unstable();
    expected.sort_unstable();
    assert_eq!(
        names, expected,
        "SHA256SUMS lists exactly the shipped files"
    );
    for (name, hash) in listed {
        assert_eq!(hash.len(), 64, "{name}");
        let bytes = fs::read(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            sha256_hex(&bytes),
            hash,
            "{name} differs from the pinned hash"
        );
    }
}

#[test]
fn what_is_embedded_is_what_is_on_disk() {
    let dir = fonts_dir();
    let fonts = embedded();
    assert_eq!(fonts.len(), FILES.len());
    for (font, file) in fonts.iter().zip(FILES) {
        assert_eq!(font.file_name, file);
        let on_disk = fs::read(dir.join(file)).expect(file);
        assert_eq!(sha256_hex(font.data), sha256_hex(&on_disk), "{file}");
    }
}
