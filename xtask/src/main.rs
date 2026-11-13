//! Repository tasks: `check-deps`, `check-size`, `tokens`, `icons`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use xtask::Error;
use xtask::check_deps::{Rules, check, load_graph};
use xtask::icons;
use xtask::tokens;
use xtask::upstream;

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

/// A generator task: its default files and the function that renders the output.
struct Task {
    name: &'static str,
    input: &'static str,
    output: &'static str,
    generate: fn(&str) -> Result<String, Error>,
    /// The script that exports `input` from the React checkout, for `--check`.
    script: Option<&'static str>,
}

const TOKENS: Task = Task {
    name: "tokens",
    input: "docs/spec/tokens.json",
    output: "crates/twake-tokens/src/generated.rs",
    generate: tokens::generate,
    script: Some("docs/spec/export-tokens.mjs"),
};

const ICONS: Task = Task {
    name: "icons",
    input: "docs/spec/icons.json",
    output: "crates/twake-icons/src/generated.rs",
    generate: icons::generate,
    script: None,
};

struct Options {
    input: PathBuf,
    output: PathBuf,
    check: bool,
    exporter: Option<PathBuf>,
}

/// A flag that takes a file: false when the flag is unknown or lacks its value.
fn set_path(options: &mut Options, flag: &str, path: Option<PathBuf>) -> bool {
    match (flag, path) {
        ("--input", Some(path)) => options.input = path,
        ("--output", Some(path)) => options.output = path,
        ("--exporter", Some(path)) => options.exporter = Some(path),
        _ => return false,
    }
    true
}

/// The flag that is unknown, or that lacks its value.
fn parse_options<'a>(task: &Task, args: &'a [String]) -> Result<Options, &'a String> {
    let mut options = Options {
        input: workspace_file(task.input),
        output: workspace_file(task.output),
        check: false,
        exporter: None,
    };
    let mut args = args.iter();
    while let Some(flag) = args.next() {
        if flag == "--check" {
            options.check = true;
        } else if !set_path(&mut options, flag, args.next().map(PathBuf::from)) {
            return Err(flag);
        }
    }
    Ok(options)
}

fn render(task: &Task, input: &Path) -> Result<String, String> {
    let json = std::fs::read_to_string(input)
        .map_err(|error| format!("cannot read {}: {error}", input.display()))?;
    (task.generate)(&json).map_err(|error| error.to_string())
}

/// 0 when `input` is what the exporter makes now (or no exporter can run here), 1 when it differs,
/// 2 when the exporter fails.
fn check_upstream(task: &Task, options: &Options) -> ExitCode {
    let script = task.script.map(workspace_file);
    let checkout = std::env::var("TWAKE_SPACE_CHAT_DIR").ok();
    let Some((mut command, program)) =
        upstream::exporter(options.exporter.as_deref(), script.as_deref(), checkout)
    else {
        return ExitCode::SUCCESS;
    };
    let name = task.name;
    let fresh = match upstream::run(&mut command, &program) {
        Ok(bytes) => bytes,
        Err(message) => {
            eprintln!("{name}: {message}");
            return ExitCode::from(2);
        }
    };
    if std::fs::read(&options.input).is_ok_and(|current| current == fresh) {
        return ExitCode::SUCCESS;
    }
    let (input, script) = (options.input.display(), task.script.unwrap_or(&program));
    eprintln!(
        "{name}: {input} differs from what {program} exports; refresh it with `node {script} <twake-space-chat> > {input}`, then `cargo xtask {name}`"
    );
    ExitCode::FAILURE
}

fn store(task: &Task, options: &Options, text: String) -> ExitCode {
    let (name, output) = (task.name, &options.output);
    if options.check {
        if std::fs::read_to_string(output).is_ok_and(|current| current == text) {
            return check_upstream(task, options);
        }
        eprintln!(
            "{name}: {} is stale; run `cargo xtask {name}`",
            output.display()
        );
        return ExitCode::FAILURE;
    }
    match std::fs::write(output, text) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{name}: cannot write {}: {error}", output.display());
            ExitCode::from(2)
        }
    }
}

/// `<task> [--input FILE] [--output FILE] [--check [--exporter PROGRAM]]`: 0 done, 1 stale (`--check`), 2 failure.
fn run(task: &Task, args: &[String]) -> ExitCode {
    let name = task.name;
    let options = match parse_options(task, args) {
        Ok(options) => options,
        Err(flag) => {
            eprintln!(
                "{name}: unexpected {flag}; usage: cargo xtask {name} [--input FILE] [--output FILE] [--check [--exporter PROGRAM]]"
            );
            return ExitCode::from(2);
        }
    };
    match render(task, &options.input) {
        Ok(text) => store(task, &options, text),
        Err(message) => {
            eprintln!("{name}: {message}");
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let task = args.first().map_or("", String::as_str);
    match task {
        "check-deps" => return check_deps(),
        "tokens" => return run(&TOKENS, &args[1..]),
        "icons" => return run(&ICONS, &args[1..]),
        "check-size" => eprintln!("xtask {task}: not implemented"),
        _ => eprintln!("usage: cargo xtask <check-deps|check-size|tokens|icons>"),
    }
    ExitCode::from(2)
}
