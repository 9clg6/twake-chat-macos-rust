//! `cargo xtask tokens [--input FILE] [--output FILE] [--check]`, through the real binary.
//! Need: CI runs `cargo xtask tokens --check` and fails (exit 1) when `generated.rs` is stale;
//! bad input is a different failure (exit 2, message on stderr, nothing written).
//! Public items used are listed in `docs/dev/tokens-api-sketch.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SHADES: &str = "\"50\":\"#f6fbff\",\"100\":\"#e3f1ff\",\"200\":\"#d2e9ff\",\"300\":\"#bfdfff\",\"400\":\"#7bbdff\",\"500\":\"#46a2ff\",\"600\":\"#0a84ff\",\"700\":\"#006bd8\",\"800\":\"#0157ad\",\"900\":\"#082139\"";

fn fixture() -> String {
    format!(
        "{{\"twakeMuiVersion\":\"9.9.9\",\"paletteRaw\":{{\"Primary\":{{{SHADES},\"ContrastText\":\"#fff\"}}}}}}"
    )
}

/// A fresh directory under the system temp dir, removed on drop.
struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("twake-xtask-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        let made = fs::create_dir_all(&path);
        assert!(made.is_ok(), "cannot create {path:?}");
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `None` when the binary cannot even be started (every assertion below then fails).
fn tokens(args: &[&str]) -> Option<Output> {
    let run = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("tokens")
        .args(args)
        .output();
    run.ok()
}

fn run(input: &Path, output: &Path, extra: &[&str]) -> Option<Output> {
    let (i, o) = (input.to_string_lossy(), output.to_string_lossy());
    let mut args = vec!["--input", i.as_ref(), "--output", o.as_ref()];
    args.extend_from_slice(extra);
    tokens(&args)
}

fn code(output: &Option<Output>) -> Option<i32> {
    output.as_ref().and_then(|o| o.status.code())
}

fn stderr(output: &Option<Output>) -> String {
    output
        .as_ref()
        .map(|o| String::from_utf8_lossy(&o.stderr).into_owned())
        .unwrap_or_default()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// A directory holding a valid `tokens.json`, and the generated file already written.
fn generated(name: &str) -> (Dir, PathBuf, PathBuf) {
    let dir = Dir::new(name);
    let (input, output) = (dir.file("tokens.json"), dir.file("generated.rs"));
    assert!(fs::write(&input, fixture()).is_ok());
    assert_eq!(code(&run(&input, &output, &[])), Some(0));
    (dir, input, output)
}

#[test]
fn it_writes_the_generated_file_and_exits_zero() {
    let (_dir, _input, output) = generated("write");
    let text = read(&output);
    assert!(text.starts_with("// @generated"), "{text}");
    assert!(text.contains("PRIMARY"));
}

#[test]
fn running_twice_changes_nothing() {
    let (_dir, input, output) = generated("twice");
    let first = read(&output);
    assert_eq!(code(&run(&input, &output, &[])), Some(0));
    assert_eq!(read(&output), first);
}

#[test]
fn it_overwrites_a_stale_file() {
    let (_dir, input, output) = generated("overwrite");
    let good = read(&output);
    assert!(fs::write(&output, "// hand edited\n").is_ok());
    assert_eq!(code(&run(&input, &output, &[])), Some(0));
    assert_eq!(read(&output), good);
}

#[test]
fn check_passes_when_the_file_is_up_to_date() {
    let (_dir, input, output) = generated("check-ok");
    assert_eq!(code(&run(&input, &output, &["--check"])), Some(0));
}

#[test]
fn check_fails_with_code_1_when_the_file_was_edited_and_leaves_it_alone() {
    let (_dir, input, output) = generated("check-edited");
    assert!(fs::write(&output, "// hand edited\n").is_ok());
    let result = run(&input, &output, &["--check"]);
    assert_eq!(code(&result), Some(1));
    assert!(
        stderr(&result).contains("cargo xtask tokens"),
        "tells how to fix it"
    );
    assert_eq!(read(&output), "// hand edited\n");
}

#[test]
fn check_fails_when_the_json_changed_after_generation() {
    let (_dir, input, output) = generated("check-json");
    assert!(fs::write(&input, fixture().replace("#0a84ff", "#0a84fe")).is_ok());
    assert_eq!(code(&run(&input, &output, &["--check"])), Some(1));
}

#[test]
fn check_fails_when_the_file_is_missing_and_does_not_create_it() {
    let dir = Dir::new("check-missing");
    let (input, output) = (dir.file("tokens.json"), dir.file("generated.rs"));
    assert!(fs::write(&input, fixture()).is_ok());
    assert_eq!(code(&run(&input, &output, &["--check"])), Some(1));
    assert!(!output.exists());
}

#[test]
fn invalid_json_exits_2_with_a_message_and_writes_nothing() {
    let dir = Dir::new("bad-json");
    let (input, output) = (dir.file("tokens.json"), dir.file("generated.rs"));
    assert!(fs::write(&input, "{ not json").is_ok());
    let result = run(&input, &output, &[]);
    assert_eq!(code(&result), Some(2));
    assert!(
        stderr(&result).contains("invalid JSON"),
        "{}",
        stderr(&result)
    );
    assert!(!output.exists());
}

#[test]
fn invalid_json_is_also_exit_2_with_check() {
    let dir = Dir::new("bad-json-check");
    let (input, output) = (dir.file("tokens.json"), dir.file("generated.rs"));
    assert!(fs::write(&input, "{ not json").is_ok());
    let result = run(&input, &output, &["--check"]);
    assert_eq!(code(&result), Some(2));
    assert!(
        stderr(&result).contains("invalid JSON"),
        "{}",
        stderr(&result)
    );
}

#[test]
fn a_missing_input_file_exits_2_and_names_the_file() {
    let dir = Dir::new("no-input");
    let (input, output) = (dir.file("absent.json"), dir.file("generated.rs"));
    let result = run(&input, &output, &[]);
    assert_eq!(code(&result), Some(2));
    assert!(
        stderr(&result).contains("absent.json"),
        "{}",
        stderr(&result)
    );
}

#[test]
fn an_unknown_flag_exits_2() {
    let result = tokens(&["--frobnicate"]);
    assert_eq!(code(&result), Some(2));
    assert!(
        stderr(&result).contains("--frobnicate"),
        "{}",
        stderr(&result)
    );
}

#[test]
fn without_flags_the_committed_file_is_checked_against_the_committed_json() {
    // CI runs exactly this: `cargo xtask tokens --check` from anywhere in the repo.
    assert_eq!(code(&tokens(&["--check"])), Some(0));
}
