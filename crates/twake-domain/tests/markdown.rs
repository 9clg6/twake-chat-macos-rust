//! Chat Markdown as the HTML of Matrix, and the link written by the link dialog.
//!
//! Need (not the code): `docs/spec/features/composer.md`
//! - D35: bold, italic, struck, underline `<u>`, link, lists, quote, code, code block; sent as
//!   `org.matrix.custom.html`; e2e "Formatting": `**loud**` + a code block gives
//!   `<strong>loud</strong><pre><code>def f():\n    return 1</code></pre>`.
//! - D97: link dialog: http/https only, `www.` gets `https://`, anything else is refused,
//!   empty text becomes the address; `[docs](https://www.x.org)`.
//! - D584: a fence on one line or across lines is a code block; an unclosed fence stays text.
//! - Matrix spec (D418): `body` is the plain text, `formatted_body` the HTML; nothing the user
//!   typed as a tag may reach the HTML (the text is escaped first).
//!
//! Public items used are listed in `docs/dev/composer-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_domain::markdown::{OutgoingBody, link_markdown, to_message};

fn html(text: &str) -> Option<String> {
    to_message(text).formatted_body
}

fn some(s: &str) -> Option<String> {
    Some(s.to_string())
}

// ------------------------------------------------------------ plain text

#[test]
fn plain_text_has_no_formatted_body() {
    let out = to_message("hello there");
    assert_eq!(
        out,
        OutgoingBody {
            body: "hello there".into(),
            formatted_body: None
        }
    );
}

#[test]
fn the_body_keeps_the_markdown_as_typed() {
    assert_eq!(to_message("**loud** and `x`").body, "**loud** and `x`");
}

#[test]
fn empty_text_is_plain() {
    assert_eq!(html(""), None);
}

#[test]
fn newlines_alone_are_not_markdown() {
    assert_eq!(html("one\ntwo\nthree"), None);
}

#[test]
fn a_bare_www_address_is_left_to_the_display() {
    assert_eq!(html("see www.x.org now"), None);
}

// ------------------------------------------------------------ inline marks

#[test]
fn bold() {
    assert_eq!(html("**loud**"), some("<strong>loud</strong>"));
}

#[test]
fn bold_inside_a_sentence() {
    assert_eq!(
        html("say **loud** please"),
        some("say <strong>loud</strong> please")
    );
}

#[test]
fn italic_with_stars() {
    assert_eq!(html("*soft*"), some("<em>soft</em>"));
}

#[test]
fn italic_with_underscores() {
    assert_eq!(html("a _soft_ b"), some("a <em>soft</em> b"));
}

#[test]
fn strike() {
    assert_eq!(html("~~gone~~"), some("<del>gone</del>"));
}

#[test]
fn underline_is_the_tag_the_toolbar_writes() {
    assert_eq!(html("<u>under</u>"), some("<u>under</u>"));
}

#[test]
fn inline_code() {
    assert_eq!(
        html("run `npm test` then"),
        some("run <code>npm test</code> then")
    );
}

#[test]
fn inline_code_is_escaped_and_not_read_as_markdown() {
    assert_eq!(html("`a<b **x**`"), some("<code>a&lt;b **x**</code>"));
}

#[test]
fn a_mark_around_spaces_is_text() {
    assert_eq!(html("~~ gone ~~"), None);
}

#[test]
fn stars_between_spaces_are_text() {
    assert_eq!(html("2 * 3 * 4"), None);
}

#[test]
fn italic_is_not_read_inside_a_word() {
    assert_eq!(html("a*b*c"), None);
}

#[test]
fn snake_case_stays_as_typed() {
    assert_eq!(html("use snake_case_name here"), None);
}

#[test]
fn bold_and_code_in_one_line() {
    assert_eq!(html("**a** `b`"), some("<strong>a</strong> <code>b</code>"));
}

// ------------------------------------------------------------ links

#[test]
fn a_https_link() {
    assert_eq!(
        html("[docs](https://www.x.org)"),
        some("<a href=\"https://www.x.org\">docs</a>")
    );
}

#[test]
fn an_http_link_and_a_mailto_link() {
    assert_eq!(
        html("[a](http://a.org) [m](mailto:me@a.org)"),
        some("<a href=\"http://a.org\">a</a> <a href=\"mailto:me@a.org\">m</a>")
    );
}

#[test]
fn a_javascript_link_is_never_a_link() {
    assert_eq!(html("[x](javascript:alert1)"), None);
}

#[test]
fn a_data_link_is_never_a_link() {
    let out = html("[x](data:text/html,hi) **b**").unwrap();
    assert!(!out.contains("<a"), "{out}");
    assert!(out.contains("<strong>b</strong>"), "{out}");
}

// ------------------------------------------------------------ blocks

#[test]
fn a_quote() {
    assert_eq!(
        html("> wise words"),
        some("<blockquote>wise words</blockquote>")
    );
}

#[test]
fn a_quote_of_two_lines_is_one_block() {
    assert_eq!(
        html("> one\n> two"),
        some("<blockquote>one<br>two</blockquote>")
    );
}

#[test]
fn a_bulleted_list() {
    assert_eq!(html("- a\n- b"), some("<ul><li>a</li><li>b</li></ul>"));
}

#[test]
fn a_list_with_stars() {
    assert_eq!(html("* a\n* b"), some("<ul><li>a</li><li>b</li></ul>"));
}

#[test]
fn a_numbered_list() {
    assert_eq!(html("1. a\n2) b"), some("<ol><li>a</li><li>b</li></ol>"));
}

#[test]
fn text_then_list_has_no_break_between() {
    assert_eq!(html("intro\n- a"), some("intro<ul><li>a</li></ul>"));
}

#[test]
fn text_lines_around_a_mark_are_joined_by_br() {
    assert_eq!(html("one\n**two**"), some("one<br><strong>two</strong>"));
}

#[test]
fn list_items_hold_marks() {
    assert_eq!(
        html("- **a**"),
        some("<ul><li><strong>a</strong></li></ul>")
    );
}

// ------------------------------------------------------------ code blocks

#[test]
fn a_fenced_block_across_lines() {
    assert_eq!(
        html("```\ndef f():\n    return 1\n```"),
        some("<pre><code>def f():\n    return 1</code></pre>")
    );
}

#[test]
fn bold_then_a_block() {
    assert_eq!(
        html("**loud**\n```\nx\n```"),
        some("<strong>loud</strong><pre><code>x</code></pre>")
    );
}

#[test]
fn a_fence_closed_on_its_own_line() {
    assert_eq!(html("```code```"), some("<pre><code>code</code></pre>"));
}

#[test]
fn a_block_is_escaped_and_not_read_as_markdown() {
    assert_eq!(
        html("```\n<b>**x**</b>\n```"),
        some("<pre><code>&lt;b&gt;**x**&lt;/b&gt;</code></pre>")
    );
}

#[test]
fn an_unclosed_fence_is_text() {
    assert_eq!(html("```code"), None);
}

#[test]
fn text_after_a_block_follows_it() {
    assert_eq!(
        html("```\na\n```\nafter"),
        some("<pre><code>a</code></pre>after")
    );
}

// ------------------------------------------------------------ escaping

#[test]
fn raw_html_is_escaped_when_there_is_markdown() {
    assert_eq!(
        html("<b>hi</b> **x**"),
        some("&lt;b&gt;hi&lt;/b&gt; <strong>x</strong>")
    );
}

#[test]
fn a_script_is_escaped() {
    let out = html("**a** <script>alert(1)</script>").unwrap();
    assert!(!out.contains("<script"), "{out}");
    assert!(out.contains("&lt;script&gt;"), "{out}");
}

#[test]
fn ampersand_quotes_and_apostrophes_are_escaped() {
    assert_eq!(
        html("**a** & \"b\" 'c'"),
        some("<strong>a</strong> &amp; &quot;b&quot; &#39;c&#39;")
    );
}

#[test]
fn a_quote_sign_is_escaped_inside_a_quote() {
    assert_eq!(html("> a > b"), some("<blockquote>a &gt; b</blockquote>"));
}

#[test]
fn an_attribute_cannot_leave_the_href() {
    let out = html("[x](https://a.org/\"onmouseover=\"alert1)").unwrap_or_default();
    assert!(!out.contains("\"onmouseover"), "{out}");
}

// ------------------------------------------------------------ the link dialog (D97)

#[test]
fn the_dialog_writes_a_markdown_link() {
    assert_eq!(
        link_markdown("docs", "https://x.org"),
        some("[docs](https://x.org)")
    );
}

#[test]
fn www_gets_https() {
    assert_eq!(
        link_markdown("docs", "www.x.org"),
        some("[docs](https://www.x.org)")
    );
}

#[test]
fn http_is_kept() {
    assert_eq!(
        link_markdown("a", "http://a.org"),
        some("[a](http://a.org)")
    );
}

#[test]
fn the_address_is_trimmed() {
    assert_eq!(
        link_markdown("a", "  https://a.org \n"),
        some("[a](https://a.org)")
    );
}

#[test]
fn an_empty_text_becomes_the_address() {
    assert_eq!(
        link_markdown("", "https://a.org"),
        some("[https://a.org](https://a.org)")
    );
}

#[test]
fn other_schemes_and_junk_are_refused() {
    for bad in [
        "javascript:alert1",
        "ftp://a.org",
        "mailto:a@b.org",
        "not a link",
        "",
        "  ",
    ] {
        assert_eq!(link_markdown("t", bad), None, "{bad}");
    }
}

#[test]
fn a_link_written_by_the_dialog_is_read_back_as_a_link() {
    let md = link_markdown("docs", "www.x.org").unwrap();
    assert_eq!(html(&md), some("<a href=\"https://www.x.org\">docs</a>"));
}

// ------------------------------------------------------------ properties

/// Strips every tag this module may build; whatever still holds a `<` is a tag the user made.
fn strip_known_tags(html: &str) -> String {
    const TAGS: [&str; 20] = [
        "<strong>",
        "</strong>",
        "<em>",
        "</em>",
        "<del>",
        "</del>",
        "<u>",
        "</u>",
        "<code>",
        "</code>",
        "<pre>",
        "</pre>",
        "<blockquote>",
        "</blockquote>",
        "<ul>",
        "</ul>",
        "<ol>",
        "</ol>",
        "<li>",
        "</li>",
    ];
    let mut out = String::new();
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(tag) = TAGS.iter().find(|t| rest.starts_with(**t)) {
            rest = &rest[tag.len()..];
        } else if let Some(tail) = rest.strip_prefix("<br>") {
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("</a>") {
            rest = tail;
        } else if let Some(tail) = rest.strip_prefix("<a href=\"") {
            let end = tail.find("\">").expect("an <a> closes its href");
            let href = &tail[..end];
            assert!(
                ["http://", "https://", "mailto:"]
                    .iter()
                    .any(|s| href.starts_with(s)),
                "href {href}"
            );
            assert!(!href.contains(['<', '"']), "href {href}");
            rest = &tail[end + 2..];
        } else {
            let c = rest.chars().next().unwrap();
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

proptest! {
    #[test]
    fn no_tag_of_the_user_survives(text in "(?s:.{0,80})") {
        if let Some(out) = to_message(&text).formatted_body {
            let left = strip_known_tags(&out);
            prop_assert!(!left.contains('<'), "{out}");
            prop_assert!(!out.contains("<script"), "{out}");
        }
    }

    #[test]
    fn a_hostile_mix_never_lets_a_script_out(
        parts in proptest::collection::vec(
            prop_oneof![
                Just("<script>".to_string()), Just("**".to_string()), Just("`".to_string()),
                Just("[x](".to_string()), Just(")".to_string()), Just("\n".to_string()),
                Just("> ".to_string()), Just("- ".to_string()), Just("```".to_string()),
                Just("<img src=x onerror=1>".to_string()), Just("\"".to_string()),
                "[a-z ]{0,5}",
            ],
            0..14,
        )
    ) {
        let text = parts.concat();
        if let Some(out) = to_message(&text).formatted_body {
            prop_assert!(!out.contains("<script"), "{out}");
            prop_assert!(!out.contains("<img"), "{out}");
            prop_assert!(!strip_known_tags(&out).contains('<'), "{out}");
        }
    }

    #[test]
    fn the_body_is_always_the_text(text in "(?s:.{0,80})") {
        prop_assert_eq!(to_message(&text).body, text);
    }

    #[test]
    fn words_alone_have_no_formatted_body(text in "[a-zA-Z ,.]{0,60}") {
        prop_assert_eq!(to_message(&text).formatted_body, None);
    }

    #[test]
    fn a_bold_word_is_always_strong(word in "[a-z]{1,10}") {
        prop_assert_eq!(
            to_message(&format!("**{word}**")).formatted_body,
            Some(format!("<strong>{word}</strong>"))
        );
    }
}

// ------------------------------------------------------------ link and underline edge cases

#[test]
fn underline_of_one_character_is_exact() {
    assert_eq!(html("<u>x</u>"), some("<u>x</u>"));
}

#[test]
fn empty_underline_stays_text() {
    let out = to_message("<u></u>");
    assert_eq!(out.body, "<u></u>");
    assert_eq!(out.formatted_body, None);
}

#[test]
fn link_with_a_space_in_the_address_stays_text() {
    let out = html("[x](https://a.org/a b)").unwrap_or_default();
    assert!(!out.contains("<a "), "{out}");
}

#[test]
fn link_with_a_control_character_stays_text() {
    let out = html("[x](https://a.org/\u{7}a)").unwrap_or_default();
    assert!(!out.contains("<a "), "{out}");
}

#[test]
fn link_with_an_empty_label_stays_text() {
    let out = html("[](https://a.org)").unwrap_or_default();
    assert!(!out.contains("<a "), "{out}");
}

#[test]
fn link_dialog_refuses_a_scheme_without_host() {
    assert_eq!(link_markdown("t", "https://"), None);
}

#[test]
fn javascript_address_stays_text() {
    let out = html("[a](javascript:alert(1))").unwrap_or_default();
    assert!(!out.contains("<a "), "{out}");
    assert!(!out.contains("href"), "{out}");
}

#[test]
fn quote_in_an_address_cannot_add_an_attribute() {
    let out = html("[a](https://a.org/\"onmouseover=\"x)").unwrap_or_default();
    assert!(
        out.contains("href=\"https://a.org/&quot;onmouseover=&quot;x\""),
        "{out}"
    );
    assert!(!out.contains("\" onmouseover"), "{out}");
}
