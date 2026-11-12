//! The committed `crates/twake-tokens/src/generated.rs` is what the generator makes of the
//! committed `docs/spec/tokens.json` (the guard CI relies on, as a plain test).

use std::fs;

use xtask::tokens::generate;

fn read(relative: &str) -> String {
    let path = format!("{}/../{relative}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path).unwrap_or_default()
}

#[test]
fn the_real_tokens_json_generates() {
    let text = generate(&read("docs/spec/tokens.json")).unwrap_or_default();
    assert!(!read("docs/spec/tokens.json").is_empty());
    assert!(text.contains("pub const PRIMARY: Palette"));
    assert!(text.contains("pub const LIGHT: Scheme"));
    assert!(text.contains("pub const DARK: Scheme"));
    assert!(text.contains("19.3.0"), "twake-mui version in the header");
}

#[test]
fn the_committed_generated_file_is_up_to_date() {
    let want = generate(&read("docs/spec/tokens.json")).unwrap_or_default();
    assert!(!want.is_empty());
    assert_eq!(read("crates/twake-tokens/src/generated.rs"), want);
}
