//! The guard of `cargo xtask tokens --check`: `tokens.json` must be what the exporter makes of the
//! pinned twake-mui today, so that a bump of the package cannot go unseen.

use std::path::Path;
use std::process::Command;

/// The stdout of `command`, or what went wrong (the exporter's own stderr is relayed).
pub fn run(command: &mut Command, name: &str) -> Result<Vec<u8>, String> {
    match command.output() {
        Err(error) => Err(format!("cannot run {name}: {error}")),
        Ok(output) if output.status.success() => Ok(output.stdout),
        Ok(output) => Err(format!(
            "{name} failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}

/// The command that exports the tokens: the given program, else `node <script> <React checkout>`
/// when `TWAKE_SPACE_CHAT_DIR` is set, else none (a machine without the React checkout).
pub fn exporter(
    program: Option<&Path>,
    script: Option<&Path>,
    checkout: Option<String>,
) -> Option<(Command, String)> {
    if let Some(program) = program {
        return Some((Command::new(program), program.display().to_string()));
    }
    let (script, checkout) = (script?, checkout?);
    let mut command = Command::new("node");
    command.arg(script).arg(checkout);
    Some((command, format!("node {}", script.display())))
}
