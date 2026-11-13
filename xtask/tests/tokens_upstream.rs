//! `cargo xtask tokens --check` also fails when `tokens.json` differs from what
//! `docs/spec/export-tokens.mjs` makes of the pinned twake-mui (lot 1 d: until now `--check` only saw
//! a stale `generated.rs`, never a twake-mui bump). Through the real binary.
//!
//! Contract: `--exporter PROGRAM` names a program run with no argument, whose stdout is the fresh
//! `tokens.json`; it is compared byte for byte with `--input`. Without `--exporter`, the check runs
//! the exporter only when the environment variable `TWAKE_SPACE_CHAT_DIR` is set (`node
//! docs/spec/export-tokens.mjs "$TWAKE_SPACE_CHAT_DIR"`), so a machine without the React checkout
//! keeps the old behaviour. Exit codes: 0 same, 1 differs (stderr names `tokens.json` and says how
//! to refresh it: `node docs/spec/export-tokens.mjs <twake-space-chat> > docs/spec/tokens.json`,
//! then `cargo xtask tokens`), 2 when the exporter cannot run or fails.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const SHADES: &str = "\"50\":\"#f6fbff\",\"100\":\"#e3f1ff\",\"200\":\"#d2e9ff\",\"300\":\"#bfdfff\",\"400\":\"#7bbdff\",\"500\":\"#46a2ff\",\"600\":\"#0a84ff\",\"700\":\"#006bd8\",\"800\":\"#0157ad\",\"900\":\"#082139\"";

fn tokens_json(version: &str) -> String {
    format!(
        "{{\"twakeMuiVersion\":\"{version}\",\"paletteRaw\":{{\"Primary\":{{{SHADES},\"ContrastText\":\"#fff\"}}}}}}\n"
    )
}

struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("twake-upstream-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        assert!(fs::create_dir_all(&path).is_ok());
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// An executable that prints `body` (and exits with `status`).
    fn exporter(&self, body: &str, status: i32) -> PathBuf {
        let path = self.file("exporter.sh");
        let script = format!(
            "#!/bin/sh\ncat <<'EOF_JSON'\n{body}EOF_JSON\necho BOOM-FROM-EXPORTER >&2\nexit {status}\n"
        );
        assert!(fs::write(&path, script).is_ok());
        assert!(fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).is_ok());
        path
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Generates `generated.rs` from the committed `tokens.json` (version 19.3.0), then checks with the exporter.
fn check(dir: &Dir, exporter: &Path) -> Option<Output> {
    let (input, output) = (dir.file("tokens.json"), dir.file("generated.rs"));
    assert!(fs::write(&input, tokens_json("19.3.0")).is_ok());
    let args = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_xtask"))
            .arg("tokens")
            .args([
                "--input",
                &input.to_string_lossy(),
                "--output",
                &output.to_string_lossy(),
            ])
            .args(extra)
            .env_remove("TWAKE_SPACE_CHAT_DIR")
            .output()
            .ok()
    };
    assert_eq!(args(&[]).and_then(|o| o.status.code()), Some(0));
    args(&["--check", "--exporter", &exporter.to_string_lossy()])
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

#[test]
fn check_passes_when_the_exporter_gives_the_committed_json() {
    let dir = Dir::new("same");
    let exporter = dir.exporter(&tokens_json("19.3.0"), 0);
    assert_eq!(code(&check(&dir, &exporter)), Some(0));
}

#[test]
fn check_fails_with_code_1_when_twake_mui_moved_on() {
    let dir = Dir::new("bump");
    let exporter = dir.exporter(&tokens_json("19.4.0"), 0);
    let result = check(&dir, &exporter);
    assert_eq!(code(&result), Some(1));
    let message = stderr(&result);
    assert!(message.contains("tokens.json"), "{message}");
    assert!(
        message.contains("export-tokens.mjs"),
        "tells how to refresh it: {message}"
    );
}

#[test]
fn check_fails_on_a_single_changed_byte() {
    let dir = Dir::new("byte");
    let exporter = dir.exporter(&tokens_json("19.3.0").replace("#0a84ff", "#0a84fe"), 0);
    assert_eq!(code(&check(&dir, &exporter)), Some(1));
}

#[test]
fn check_fails_with_code_2_when_the_exporter_fails_and_relays_what_it_said() {
    let dir = Dir::new("fails");
    let exporter = dir.exporter("", 3);
    let result = check(&dir, &exporter);
    assert_eq!(code(&result), Some(2));
    let message = stderr(&result);
    assert!(message.contains("BOOM-FROM-EXPORTER"), "{message}");
    assert!(
        !message.contains("unexpected"),
        "the flag must be known: {message}"
    );
}

#[test]
fn check_fails_with_code_2_when_the_exporter_does_not_exist() {
    let dir = Dir::new("missing");
    let result = check(&dir, &dir.file("nowhere.sh"));
    assert_eq!(code(&result), Some(2));
    let message = stderr(&result);
    assert!(
        message.contains("nowhere.sh"),
        "names the program: {message}"
    );
    assert!(
        !message.contains("unexpected"),
        "the flag must be known: {message}"
    );
}

#[test]
fn without_an_exporter_and_without_the_react_checkout_only_generated_rs_is_checked() {
    let dir = Dir::new("plain");
    let (input, output) = (dir.file("tokens.json"), dir.file("generated.rs"));
    assert!(fs::write(&input, tokens_json("19.3.0")).is_ok());
    let run = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_xtask"))
            .arg("tokens")
            .args([
                "--input",
                &input.to_string_lossy(),
                "--output",
                &output.to_string_lossy(),
            ])
            .args(extra)
            .env_remove("TWAKE_SPACE_CHAT_DIR")
            .output()
            .ok()
    };
    assert_eq!(run(&[]).and_then(|o| o.status.code()), Some(0));
    assert_eq!(run(&["--check"]).and_then(|o| o.status.code()), Some(0));
}

#[test]
fn the_exporter_is_not_run_to_write_the_file() {
    // Plain `cargo xtask tokens --exporter X` never calls X: only `--check` compares with upstream.
    let dir = Dir::new("write");
    let exporter = dir.exporter(&tokens_json("19.4.0"), 3);
    let (input, output) = (dir.file("tokens.json"), dir.file("generated.rs"));
    assert!(fs::write(&input, tokens_json("19.3.0")).is_ok());
    let result = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("tokens")
        .args([
            "--input",
            &input.to_string_lossy(),
            "--output",
            &output.to_string_lossy(),
        ])
        .args(["--exporter", &exporter.to_string_lossy()])
        .output()
        .ok();
    assert_eq!(code(&result), Some(0));
}
