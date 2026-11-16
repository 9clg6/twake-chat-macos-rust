//! The names of the picked mentions linked in the HTML of a message (D502, Matrix permalinks).

use super::{Mention, MentionTarget};
use crate::markdown::{OutgoingBody, escape, plain_html};

/// A name to link, as it stands in the HTML, and where it points.
struct Link {
    name: String,
    href: String,
}

/// `encodeURIComponent`: everything but letters, digits and `-_.!~*'()` is percent-encoded.
fn encode_component(id: &str) -> String {
    let mut out = String::with_capacity(id.len() * 2);
    for byte in id.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn permalink(mention: &Mention) -> Option<String> {
    let id = match &mention.target {
        MentionTarget::User(id) => id.as_str(),
        MentionTarget::Room(alias) => alias.as_str(),
        MentionTarget::Everyone => return None,
    };
    Some(format!("https://matrix.to/#/{}", encode_component(id)))
}

/// The links of the picked mentions the text names, the longest name first (it wins), a name
/// once.
fn links(text: &str, picked: &[Mention]) -> Vec<Link> {
    let mut found: Vec<Link> = Vec::new();
    for mention in picked.iter().filter(|m| text.contains(&m.name)) {
        let name = escape(&mention.name);
        let Some(href) = permalink(mention) else {
            continue;
        };
        if !name.is_empty() && found.iter().all(|l| l.name != name) {
            found.push(Link { name, href });
        }
    }
    found.sort_by_key(|l| std::cmp::Reverse(l.name.len()));
    found
}

fn glued(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// The length of an entity (`&lt;`, `&#39;`) at the start of `text`.
fn entity_len(text: &str) -> Option<usize> {
    let end = text.find(';')?;
    let name = &text[1..end];
    let shaped = (1..=6).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'#');
    shaped.then_some(end + 1)
}

/// Links the names in a stretch of text that holds no tag; the count of links made.
fn link_text(text: &str, links: &[Link]) -> (String, usize) {
    let (mut out, mut count, mut at) = (String::new(), 0, 0);
    while at < text.len() {
        let rest = &text[at..];
        let before = text[..at].chars().next_back();
        let hit = links.iter().find(|l| {
            rest.strip_prefix(l.name.as_str())
                .is_some_and(|after| !glued(before) && !glued(after.chars().next()))
        });
        let used = if let Some(link) = hit {
            out.push_str(&format!(
                "<a href=\"{}\">{}</a>",
                escape(&link.href),
                link.name
            ));
            count += 1;
            link.name.len()
        } else {
            let used = if rest.starts_with('&') {
                entity_len(rest).unwrap_or(1)
            } else {
                rest.chars().next().map_or(1, char::len_utf8)
            };
            out.push_str(&rest[..used]);
            used
        };
        at += used;
    }
    (out, count)
}

/// Links the names in the text of `html`, but not inside `<a>`, `<code>` or `<pre>`.
fn link_html(html: &str, links: &[Link]) -> (String, usize) {
    let (mut out, mut count, mut skipping) = (String::new(), 0, 0_i32);
    let mut rest = html;
    while !rest.is_empty() {
        let end = rest.find('<').unwrap_or(rest.len());
        let (text, tail) = rest.split_at(end);
        if skipping > 0 {
            out.push_str(text);
        } else {
            let (linked, made) = link_text(text, links);
            out.push_str(&linked);
            count += made;
        }
        let tag_end = tail.find('>').map_or(tail.len(), |at| at + 1);
        let (tag, after) = tail.split_at(tag_end);
        skipping += depth_change(tag);
        out.push_str(tag);
        rest = after;
    }
    (out, count)
}

/// +1 for the opening of `<a>`, `<code>`, `<pre>`; -1 for their closing; else 0.
fn depth_change(tag: &str) -> i32 {
    let inner = tag.trim_start_matches('<').trim_end_matches('>');
    let (closing, name) = match inner.strip_prefix('/') {
        Some(name) => (true, name),
        None => (false, inner.split(' ').next().unwrap_or_default()),
    };
    match (["a", "code", "pre"].contains(&name), closing) {
        (true, true) => -1,
        (true, false) => 1,
        _ => 0,
    }
}

/// Each present name of a person or a room becomes a matrix.to link, in the HTML of `body` (the
/// escaped text with `<br>` when it has none). With nothing to link, `body` comes back unchanged.
pub fn link_names(body: OutgoingBody, picked: &[Mention]) -> OutgoingBody {
    let links = links(&body.body, picked);
    if links.is_empty() {
        return body;
    }
    let base = body
        .formatted_body
        .clone()
        .unwrap_or_else(|| plain_html(&body.body));
    match link_html(&base, &links) {
        (_, 0) => body,
        (html, _) => OutgoingBody {
            formatted_body: Some(html),
            ..body
        },
    }
}
