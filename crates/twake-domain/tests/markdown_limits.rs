//! Robustness: hostile or accidental input must neither overflow the stack, panic nor take long.
//!
//! Need (not the code): a message is typed by a person and pasted from anywhere; sending must
//! always finish quickly (< 2 s). Each case runs on a thread with the default 2 MiB stack, the
//! strictest the app gives to a worker; an abort (stack overflow) or a panic is a production bug.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::time::{Duration, Instant};

use twake_domain::markdown::to_message;

const KIB64: usize = 64 * 1024;

fn run(label: &str, text: String) {
    let started = Instant::now();
    let handle = std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let out = to_message(&text);
            assert_eq!(out.body, text);
        })
        .unwrap();
    handle
        .join()
        .unwrap_or_else(|_| panic!("{label}: panicked"));
    let took = started.elapsed();
    assert!(took < Duration::from_secs(2), "{label}: took {took:?}");
}

#[test]
fn ten_thousand_nested_stars() {
    run(
        "nested *",
        format!("{}x{}", "*".repeat(10_000), "*".repeat(10_000)),
    );
}

#[test]
fn ten_thousand_nested_double_stars() {
    run(
        "nested **",
        format!("{}x{}", "**".repeat(10_000), "**".repeat(10_000)),
    );
}

#[test]
fn ten_thousand_nested_brackets() {
    run(
        "nested [",
        format!(
            "{}x{}",
            "[".repeat(10_000),
            "](https://a.org)".repeat(10_000)
        ),
    );
}

#[test]
fn ten_thousand_nested_underline_tags() {
    run(
        "nested <u>",
        format!("{}x{}", "<u>".repeat(10_000), "</u>".repeat(10_000)),
    );
}

#[test]
fn ten_thousand_nested_quotes_and_lists() {
    run("nested >", format!("{}x", "> ".repeat(10_000)));
    run("nested list", format!("{}x", "- ".repeat(10_000)));
}

#[test]
fn sixty_four_kib_of_stars() {
    run("64k *", "*".repeat(KIB64));
}

#[test]
fn sixty_four_kib_of_open_brackets() {
    run("64k [", "[".repeat(KIB64));
}

#[test]
fn sixty_four_kib_of_link_openers() {
    run("64k ](", "](".repeat(KIB64 / 2));
}

#[test]
fn sixty_four_kib_of_bracket_paren_pairs() {
    run("64k [](", "[](".repeat(KIB64 / 3));
}

#[test]
fn sixty_four_kib_of_backticks_and_tildes() {
    run("64k `", "`".repeat(KIB64));
    run("64k ~", "~".repeat(KIB64));
}

#[test]
fn sixty_four_kib_of_newlines() {
    run("64k \\n", "\n".repeat(KIB64));
}
