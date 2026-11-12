//! Chat Markdown as the HTML of Matrix, and the link written by the link dialog (D35, D97, A020).
//!
//! A small parser of our own, line by line: the text is escaped first, so no tag the user typed
//! ever comes out, only the tags built here.

mod block;
mod inline;

pub(crate) use inline::escape;

/// A message as typed, and as Matrix wants it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutgoingBody {
    /// The text exactly as typed (the plain fallback).
    pub body: String,
    /// `org.matrix.custom.html`; `None` when the text holds no Markdown.
    pub formatted_body: Option<String>,
}

/// The text as HTML with nothing but line breaks: what a text without Markdown comes to.
pub(crate) fn plain_html(text: &str) -> String {
    escape(text).replace('\n', "<br>")
}

/// Chat Markdown as Matrix HTML (A020). `formatted_body` is `None` when the HTML would be the
/// escaped text with `<br>` for the newlines.
pub fn to_message(text: &str) -> OutgoingBody {
    let lines: Vec<&str> = text.split('\n').collect();
    let html = block::render(&lines);
    let formatted_body = (html != plain_html(text)).then_some(html);
    OutgoingBody {
        body: text.to_string(),
        formatted_body,
    }
}

/// D97: the Markdown the link dialog writes. `http://` and `https://` are kept, `www.` gets
/// `https://`, anything else (other scheme, spaces inside, empty) is `None`. An empty `label`
/// becomes the address. Parentheses of the address are percent-encoded so that they cannot end
/// the link.
pub fn link_markdown(label: &str, address: &str) -> Option<String> {
    let address = address.trim();
    if address.chars().any(char::is_whitespace) {
        return None;
    }
    let address = if address.starts_with("www.") {
        format!("https://{address}")
    } else {
        address.to_string()
    };
    let lower = address.to_lowercase();
    let host = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))?;
    if host.is_empty() {
        return None;
    }
    let address = address.replace('(', "%28").replace(')', "%29");
    let label = label.trim();
    let label = if label.is_empty() { &address } else { label };
    Some(format!("[{label}]({address})"))
}
