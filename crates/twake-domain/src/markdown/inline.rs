//! The marks inside a line: bold, italic, strike, underline, code, link.

/// Schemes a link may have (A020); anything else stays text.
const LINK_SCHEMES: [&str; 4] = ["http://", "https://", "mailto:", "matrix:"];

/// `& < > " '` as entities.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric()
}

fn solid(c: Option<char>) -> bool {
    c.is_some_and(|c| !c.is_whitespace())
}

/// The first closing `delim` of `rest` (an opening of `open` bytes aside) that has a non-space
/// before it, an inner text, and that `fits(before, after)`; its offset.
fn find_close(
    rest: &str,
    open: usize,
    delim: &str,
    fits: impl Fn(Option<char>, Option<char>) -> bool,
) -> Option<usize> {
    rest[open..]
        .match_indices(delim)
        .map(|(at, _)| at + open)
        .filter(|&at| at > open)
        .find(|&at| {
            let before = rest[..at].chars().next_back();
            let after = rest[at + delim.len()..].chars().next();
            solid(before) && fits(before, after)
        })
}

/// `delim inner delim` as `<tag>inner</tag>`, and the bytes it takes.
fn pair(rest: &str, delim: &str, tag: &str) -> Option<(String, usize)> {
    let open = delim.len();
    if !solid(rest.strip_prefix(delim)?.chars().next()) {
        return None;
    }
    let close = find_close(rest, open, delim, |_, _| true)?;
    let html = format!("<{tag}>{}</{tag}>", inline(&rest[open..close]));
    Some((html, close + open))
}

/// `*i*` or `_i_`, not read inside a word nor as part of a doubled mark.
fn emphasis(rest: &str, mark: char, prev: Option<char>) -> Option<(String, usize)> {
    let after_open = rest[1..].chars().next();
    if prev.is_some_and(is_word) || !solid(after_open) || after_open == Some(mark) {
        return None;
    }
    let delim = mark.to_string();
    let lone = |before: Option<char>, after: Option<char>| {
        before != Some(mark) && after != Some(mark) && !after.is_some_and(is_word)
    };
    let close = find_close(rest, 1, &delim, lone)?;
    Some((format!("<em>{}</em>", inline(&rest[1..close])), close + 1))
}

/// `` `code` ``: escaped, never read as Markdown.
fn code(rest: &str) -> Option<(String, usize)> {
    let close = rest[1..].find('`')? + 1;
    (close > 1).then(|| {
        (
            format!("<code>{}</code>", escape(&rest[1..close])),
            close + 1,
        )
    })
}

/// `<u>under</u>`, the tag the toolbar writes.
fn underline(rest: &str) -> Option<(String, usize)> {
    let inner = rest.strip_prefix("<u>")?;
    let close = inner.find("</u>").filter(|&at| at > 0)?;
    Some((format!("<u>{}</u>", inline(&inner[..close])), close + 7))
}

fn allowed_link(url: &str) -> bool {
    let lower = url.to_lowercase();
    !url.chars().any(|c| c.is_whitespace() || c.is_control())
        && LINK_SCHEMES
            .iter()
            .any(|scheme| lower.strip_prefix(scheme).is_some_and(|r| !r.is_empty()))
}

/// `[text](address)` with an address of an allowed scheme.
fn link(rest: &str) -> Option<(String, usize)> {
    let label_end = rest.find("](")?;
    let url_end = label_end + 2 + rest[label_end + 2..].find(')')?;
    let (label, url) = (&rest[1..label_end], &rest[label_end + 2..url_end]);
    if label.is_empty() || label.contains('[') || !allowed_link(url) {
        return None;
    }
    let html = format!("<a href=\"{}\">{}</a>", escape(url), inline(label));
    Some((html, url_end + 1))
}

/// `**b**` or `*i*`, `_i_`.
fn emphasized(rest: &str, prev: Option<char>) -> Option<(String, usize)> {
    let mark = rest.chars().next()?;
    pair(rest, "**", "strong").or_else(|| emphasis(rest, mark, prev))
}

/// The mark that starts `rest`, as HTML, and the bytes it takes.
fn mark_at(rest: &str, prev: Option<char>) -> Option<(String, usize)> {
    match rest.chars().next()? {
        '*' | '_' => emphasized(rest, prev),
        '~' => pair(rest, "~~", "del"),
        '`' => code(rest),
        '[' => link(rest),
        '<' => underline(rest),
        _ => None,
    }
}

/// A line as HTML: its marks built, the rest escaped.
pub(super) fn inline(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut at = 0;
    let mut prev = None;
    while at < src.len() {
        let rest = &src[at..];
        let (html, used) = mark_at(rest, prev).unwrap_or_else(|| {
            let c = rest.chars().next().unwrap_or_default();
            (escape(c.encode_utf8(&mut [0; 4])), c.len_utf8())
        });
        out.push_str(&html);
        at += used;
        prev = src[..at].chars().next_back();
    }
    out
}
