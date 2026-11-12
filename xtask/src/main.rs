//! Repository tasks: `check-deps`, `check-size`, `tokens`, `icons`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use xtask::check_deps::{Rules, check, load_graph};
use xtask::tokens::generate;

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

fn workspace_file(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(relative)
}

/// `tokens [--input FILE] [--output FILE] [--check]`: 0 done, 1 stale (`--check`), 2 failure.
fn tokens(args: &[String]) -> ExitCode {
    let mut input = workspace_file("docs/spec/tokens.json");
    let mut output = workspace_file("crates/twake-tokens/src/generated.rs");
    let mut check = false;
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--check" => check = true,
            "--input" | "--output" if args.len() > 0 => {
                let path = args.next().map(PathBuf::from).unwrap_or_default();
                if flag == "--input" {
                    input = path;
                } else {
                    output = path;
                }
            }
            _ => {
                eprintln!(
                    "tokens: unexpected {flag}; usage: cargo xtask tokens [--input FILE] [--output FILE] [--check]"
                );
                return ExitCode::from(2);
            }
        }
    }
    let json = match std::fs::read_to_string(&input) {
        Ok(json) => json,
        Err(error) => {
            eprintln!("tokens: cannot read {}: {error}", input.display());
            return ExitCode::from(2);
        }
    };
    let text = match generate(&json) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("tokens: {error}");
            return ExitCode::from(2);
        }
    };
    if check {
        if std::fs::read_to_string(&output).is_ok_and(|current| current == text) {
            return ExitCode::SUCCESS;
        }
        eprintln!(
            "tokens: {} is stale; run `cargo xtask tokens`",
            output.display()
        );
        return ExitCode::FAILURE;
    }
    match std::fs::write(&output, text) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tokens: cannot write {}: {error}", output.display());
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let task = args.first().map_or("", String::as_str);
    match task {
        "check-deps" => return check_deps(),
        "tokens" => return tokens(&args[1..]),
        "check-size" | "icons" => eprintln!("xtask {task}: not implemented"),
        _ => eprintln!("usage: cargo xtask <check-deps|check-size|tokens|icons>"),
    }
    ExitCode::from(2)
}
