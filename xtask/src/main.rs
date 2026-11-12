//! Repository tasks: `check-deps`, `check-size`, `tokens`, `icons`. Not implemented yet.

use std::process::ExitCode;

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    match task.as_str() {
        "check-deps" | "check-size" | "tokens" | "icons" => {
            eprintln!("xtask {task}: not implemented");
        }
        _ => eprintln!("usage: cargo xtask <check-deps|check-size|tokens|icons>"),
    }
    ExitCode::from(2)
}
