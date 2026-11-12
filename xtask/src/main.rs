//! Repository tasks: `check-deps`, `check-size`, `tokens`, `icons`.

use std::path::Path;
use std::process::ExitCode;

use xtask::check_deps::{Rules, check, load_graph};

fn check_deps() -> ExitCode {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml");
    match load_graph(&manifest) {
        Err(error) => {
            eprintln!("check-deps: {error}");
            ExitCode::from(2)
        }
        Ok(graph) => {
            let violations = check(&Rules::twake(), &graph);
            for violation in &violations {
                println!("{violation}");
            }
            if violations.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    }
}

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    match task.as_str() {
        "check-deps" => return check_deps(),
        "check-size" | "tokens" | "icons" => eprintln!("xtask {task}: not implemented"),
        _ => eprintln!("usage: cargo xtask <check-deps|check-size|tokens|icons>"),
    }
    ExitCode::from(2)
}
