//! Edge cases of the inline converter, pinned from the need: a name escaped by `escape_markdown`
//! reads back as typed (D418 literal names), a backslash escapes only what it may, and a mark
//! that is not closed, or a link that is not safe, stays text.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use twake_domain::markdown::{escape_markdown, to_message};

/// The HTML sent, `None` when the text holds no Markdown (it stays text).
fn html(text: &str) -> Option<String> {
    to_message(text).formatted_body
}

fn check_escaped(cases: &[(&str, &str)]) {
    for (name, expected) in cases {
        assert_eq!(escape_markdown(name), *expected, "name {name:?}");
    }
}

fn check_html(cases: &[(&str, Option<&str>)]) {
    for (text, expected) in cases {
        assert_eq!(html(text).as_deref(), *expected, "text {text:?}");
    }
}

/// What a reader sees: the tags dropped, the entities read.
fn visible(text: &str) -> String {
    let Some(html) = html(text) else {
        return text.to_string();
    };
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[test]
fn an_underscore_between_two_letters_is_not_escaped() {
    check_escaped(&[("john_doe", "john_doe"), ("a1_2b", "a1_2b"), ("é_è", "é_è")]);
}

#[test]
fn an_underscore_that_could_open_or_close_a_mark_is_escaped() {
    check_escaped(&[
        ("_x_", "\\_x\\_"),
        ("_b", "\\_b"),
        ("a_", "a\\_"),
        (" _b", " \\_b"),
        ("a_ ", "a\\_ "),
        ("__", "\\_\\_"),
    ]);
}

#[test]
fn a_dash_or_a_quote_mark_is_escaped_at_the_start_only() {
    check_escaped(&[
        ("-x", "\\-x"),
        ("a-x", "a-x"),
        (">x", "\\>x"),
        ("a>x", "a>x"),
    ]);
}

#[test]
fn the_chars_that_start_a_mark_are_escaped_anywhere() {
    check_escaped(&[
        ("\\", "\\\\"),
        ("a*b", "a\\*b"),
        ("[x](y)", "\\[x\\](y)"),
        ("a<b", "a\\<b"),
        ("a~b", "a\\~b"),
        ("a`b", "a\\`b"),
        ("a#b", "a#b"),
        ("#a", "#a"),
    ]);
}

#[test]
fn an_escaped_name_reads_back_as_typed() {
    let names = [
        "a*b*",
        "[x](y)",
        "_a_",
        "\\*",
        "**",
        "john_doe",
        "-x",
        ">x",
        "# t",
        "~~x~~",
        "`c`",
        "<u>x</u>",
        "é_è",
        "a\\",
        "*",
        "[a",
        "x_",
        "__",
        "***a***",
        "[a](https://x.org)",
    ];
    for name in names {
        let sent = escape_markdown(name);
        assert_eq!(to_message(&sent).body, sent);
        assert_eq!(visible(&sent), name, "name {name:?}");
    }
}

#[test]
fn a_backslash_before_a_multibyte_char_stays_text_and_does_not_panic() {
    check_html(&[
        ("\\é", None),
        ("\\😀", None),
        ("\\\u{a0}", None),
        ("a\\", None),
    ]);
}

#[test]
fn a_backslash_before_a_mark_char_shows_the_char() {
    check_html(&[
        ("\\*", Some("*")),
        ("\\\\", Some("\\")),
        ("\\<", Some("&lt;")),
    ]);
}

#[test]
fn an_address_with_a_paren_a_space_or_a_control_char_is_no_link() {
    check_html(&[
        ("[a](https://x.org/(y))", None),
        ("[a](https://x.org/a b)", None),
        ("[a](https://x.org/a\u{1}b)", None),
        ("[a](https://x.org/a\tb)", None),
    ]);
}

#[test]
fn an_address_ends_at_its_first_closing_paren() {
    check_html(&[(
        "[a](https://x.org)tail)",
        Some("<a href=\"https://x.org\">a</a>tail)"),
    )]);
}

#[test]
fn only_the_listed_schemes_with_something_after_make_a_link() {
    check_html(&[
        ("[a](ftp://x.org)", None),
        ("[a](javascript:alert)", None),
        ("[a](https://)", None),
        ("[a](mailto:)", None),
        ("[a](x.org)", None),
        (
            "[a](HTTPS://x.org)",
            Some("<a href=\"HTTPS://x.org\">a</a>"),
        ),
        (
            "[a](mailto:b@x.org)",
            Some("<a href=\"mailto:b@x.org\">a</a>"),
        ),
        ("[a](matrix:r/x)", Some("<a href=\"matrix:r/x\">a</a>")),
    ]);
}

#[test]
fn an_escaped_closing_star_does_not_close() {
    check_html(&[
        ("*a\\*b*", Some("<em>a*b</em>")),
        ("*a\\*", Some("*a*")),
        ("**a\\**b**", Some("<strong>a**b</strong>")),
        ("_a\\_b_", Some("<em>a_b</em>")),
    ]);
}

#[test]
fn a_mark_not_closed_stays_text() {
    check_html(&[
        ("*abc", None),
        ("**abc", None),
        ("_abc", None),
        ("****", None),
    ]);
}

#[test]
fn a_space_before_the_closing_mark_does_not_close() {
    check_html(&[
        ("*a *", None),
        ("**a **", None),
        ("_a _", None),
        ("*a *b*", Some("<em>a *b</em>")),
    ]);
}

#[test]
fn an_emphasis_is_not_read_inside_a_word_nor_closed_inside_one() {
    check_html(&[
        ("snake_case_name", None),
        ("a*b*", None),
        ("_a_b", None),
        ("*a*b", None),
        ("x_a_", None),
        ("_a_ b", Some("<em>a</em> b")),
        ("_a_.", Some("<em>a</em>.")),
        ("(_a_)", Some("(<em>a</em>)")),
    ]);
}

#[test]
fn a_doubled_mark_or_a_mark_before_a_space_is_no_emphasis() {
    check_html(&[
        ("__a__", None),
        ("*a**", None),
        ("x * a*", None),
        ("**a*", Some("*<em>a</em>")),
    ]);
}
