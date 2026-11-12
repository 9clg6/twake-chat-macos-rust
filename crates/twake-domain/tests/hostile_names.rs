//! Trust boundary: the name of another person or room (a display name, a room id) is text, never
//! markup. Whatever it holds, the HTML written around it keeps it as escaped text, and the href
//! of a mention link never carries a raw `"`, `<` or `>`.
//!
//! Need (not the code): Matrix spec (D418), `formatted_body` is HTML read by other clients;
//! D502 permalinks `https://matrix.to/#/<percent-encoded id>`; the module doc of
//! `twake_domain::markdown`: "no tag the user typed ever comes out".
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use twake_domain::markdown::to_message;
use twake_domain::mention::{Mention, link_names};
use twake_domain::{RoomAlias, UserId};

/// Names that carry markup (HTML), none that Markdown would turn into a tag.
const HTML_NAMES: [&str; 6] = [
    "<img src=x onerror=alert(1)>",
    "\"><script>alert(1)</script>",
    "<a href=\"https://evil.example\">click</a>",
    "&amp;",
    "&lt;b&gt;",
    "x' onmouseover='alert(1)",
];

/// Only the tags the sender's own text and our links make. A `<` starts one of these or the
/// output is hostile.
const ALLOWED: [&str; 3] = ["<a href=\"https://matrix.to/#/", "</a>", "<br>"];

fn only_allowed_tags(html: &str) -> bool {
    html.match_indices('<')
        .all(|(at, _)| ALLOWED.iter().any(|tag| html[at..].starts_with(tag)))
}

/// Every href of the output is a matrix.to permalink without a raw quote, `<` or `>`.
fn hrefs_are_safe(html: &str) -> bool {
    html.match_indices("href=\"").all(|(at, _)| {
        let rest = &html[at + 6..];
        let Some(end) = rest.find('"') else {
            return false;
        };
        let href = &rest[..end];
        href.starts_with("https://matrix.to/#/") && !href.contains(['<', '>', '\'', ' '])
    })
}

fn user(local: &str) -> UserId {
    UserId::parse(&format!("@{local}:localhost")).unwrap()
}

fn linked(text: &str, mention: Mention) -> String {
    link_names(to_message(text), &[mention])
        .formatted_body
        .unwrap_or_default()
}

#[test]
fn a_display_name_with_markup_comes_out_as_text() {
    for name in HTML_NAMES {
        let html = linked(&format!("hi {name} bye"), Mention::user(user("eve"), name));
        assert!(only_allowed_tags(&html), "{name}: {html}");
        assert!(hrefs_are_safe(&html), "{name}: {html}");
    }
}

#[test]
fn a_display_name_that_is_a_tag_is_not_a_tag_in_the_link_text() {
    let name = "<img src=x onerror=alert(1)>";
    let html = linked(&format!("hi {name}"), Mention::user(user("eve"), name));
    assert!(html.contains("&lt;img"), "{html}");
    assert!(!html.contains("<img"), "{html}");
}

#[test]
fn a_display_name_that_looks_like_an_entity_stays_literal() {
    // "&amp;" typed by a person must read "&amp;", not "&": the ampersand is escaped again.
    let html = linked("hi &amp; bye", Mention::user(user("eve"), "&amp;"));
    assert!(!html.contains(">&amp;<"), "{html}");
    assert!(html.contains("&amp;amp;"), "{html}");
}

#[test]
fn a_display_name_with_a_newline_adds_no_markup() {
    let name = "Eve\n<b>Admin</b>";
    let html = linked(&format!("hi {name}"), Mention::user(user("eve"), name));
    assert!(only_allowed_tags(&html), "{html}");
}

#[test]
fn a_user_id_with_quotes_and_tags_never_reaches_the_href_raw() {
    for local in [
        "a\"><script>alert(1)</script>",
        "x<img src=x onerror=alert(1)>",
        "a'b",
    ] {
        let Ok(id) = UserId::parse(&format!("@{local}:localhost")) else {
            continue; // refused at the door: nothing to link
        };
        let html = linked(
            &format!("hi {}", id.as_str()),
            Mention::user(id.clone(), id.as_str()),
        );
        assert!(only_allowed_tags(&html), "{local}: {html}");
        assert!(hrefs_are_safe(&html), "{local}: {html}");
    }
}

#[test]
fn a_room_alias_with_quotes_and_tags_never_reaches_the_href_raw() {
    for local in [
        "a\"><script>alert(1)</script>",
        "x<img src=x onerror=alert(1)>",
        "a'b",
    ] {
        let Ok(alias) = RoomAlias::parse(&format!("#{local}:localhost")) else {
            continue;
        };
        let name = alias.as_str().to_string();
        let html = linked(&format!("go {name}"), Mention::room(alias, &name));
        assert!(only_allowed_tags(&html), "{local}: {html}");
        assert!(hrefs_are_safe(&html), "{local}: {html}");
    }
}

#[test]
fn the_plain_body_is_never_changed_by_a_hostile_name() {
    for name in HTML_NAMES {
        let text = format!("hi {name} bye");
        let out = link_names(to_message(&text), &[Mention::user(user("eve"), name)]);
        assert_eq!(out.body, text);
    }
}
