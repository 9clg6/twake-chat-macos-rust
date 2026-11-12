//! The committed `crates/twake-icons/src/generated.rs` is what the generator makes of the
//! committed `docs/spec/icons.json` (the guard CI relies on, as a plain test).

use std::fs;

use xtask::icons::generate;

fn read(relative: &str) -> String {
    let path = format!("{}/../{relative}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path).unwrap_or_default()
}

#[test]
fn the_real_icons_json_generates() {
    let text = generate(&read("docs/spec/icons.json")).unwrap_or_default();
    assert!(!read("docs/spec/icons.json").is_empty());
    assert!(text.contains("pub static ALL: &[Icon]"));
    assert!(
        text.contains("@linagora/twake-icons@"),
        "source in the header"
    );
}

#[test]
fn the_committed_generated_file_is_up_to_date() {
    let want = generate(&read("docs/spec/icons.json")).unwrap_or_default();
    assert!(!want.is_empty());
    assert_eq!(read("crates/twake-icons/src/generated.rs"), want);
}
