//! The pure mapping from matrix-sdk / ruma values to the domain (room list and conversation).
//! No client, no server: plain values in, plain values out. Nothing here keeps an error's text,
//! which can hold an access token (an error is mapped to a kind, never to its message).

use matrix_sdk::notification_settings::RoomNotificationMode;
use matrix_sdk::ruma::MilliSecondsSinceUnixEpoch;
use matrix_sdk::ruma::api::error::ErrorKind;
use matrix_sdk::ruma::events::room::message::MessageType;
use matrix_sdk_ui::timeline::EventSendState;
use twake_app::ports::{RoomListError, TimelineError};
use twake_domain::Timestamp;
use twake_domain::UserId;
use twake_domain::markdown::escape_markdown;
use twake_domain::read_state::UnreadCount;
use twake_domain::room_list::{LastSender, RoomKind};
use twake_domain::timeline::SendState;

/// A space is a space even when flagged direct; else Direct for a direct room, else Group.
/// Broadcast and Voice need custom `m.room.create` fields and are not read yet.
pub fn room_kind(is_space: bool, is_direct: bool) -> RoomKind {
    match (is_space, is_direct) {
        (true, _) => RoomKind::Space,
        (false, true) => RoomKind::Direct,
        (false, false) => RoomKind::Group,
    }
}

/// D31: a room on "mentions" or "none" is muted.
pub fn is_muted(mode: RoomNotificationMode) -> bool {
    !matches!(mode, RoomNotificationMode::AllMessages)
}

/// The counts of the server, each stopped at the largest value the domain holds.
pub fn unread_count(messages: u64, mentions: u64, marked_unread: bool) -> UnreadCount {
    let clamp = |n: u64| u32::try_from(n).unwrap_or(u32::MAX);
    UnreadCount {
        unread: clamp(messages),
        mentions: clamp(mentions),
        marked_unread,
    }
}

/// Who wrote the last message: the user, or a person by display name (else the local part of
/// their id). The name is plain text, never escaped.
pub fn last_sender(me: &UserId, sender: &UserId, display_name: Option<&str>) -> LastSender {
    if sender == me {
        return LastSender::Me;
    }
    LastSender::Other(name_of(sender, display_name))
}

pub(crate) fn name_of(user: &UserId, display_name: Option<&str>) -> String {
    match display_name.filter(|n| !n.trim().is_empty()) {
        Some(name) => name.to_string(),
        None => user.localpart().to_string(),
    }
}

/// The same instant, in the domain.
pub fn timestamp(at: MilliSecondsSinceUnixEpoch) -> Timestamp {
    Timestamp::from_millis(u64::from(at.0))
}

/// Where a local echo is on its way to the server.
pub fn send_state(state: &EventSendState) -> SendState {
    match state {
        EventSendState::NotSentYet { .. } => SendState::Sending,
        EventSendState::SendingFailed { .. } => SendState::Failed,
        EventSendState::Sent { .. } => SendState::Sent,
    }
}

/// The row of the room list (D31, D175): a code block shows its first line of code, an emote
/// reads the name then the words, a file without a caption says what it is.
pub fn preview(content: &MessageType, sender_name: &str) -> String {
    match content {
        MessageType::Text(text) => first_code_line(&text.body)
            .unwrap_or(&text.body)
            .to_string(),
        MessageType::Emote(emote) => format!("{sender_name} {}", emote.body),
        other => plain(other),
    }
}

/// The bubble: the text as is, an emote as `* name words` (the name escaped, since the bubble
/// is rendered as Markdown), a file without a caption says what it is.
pub fn message_body(content: &MessageType, sender_name: &str) -> String {
    match content {
        MessageType::Emote(emote) => {
            format!("* {} {}", escape_markdown(sender_name), emote.body)
        }
        other => plain(other),
    }
}

fn plain(content: &MessageType) -> String {
    match content {
        MessageType::Image(m) => caption_or(m.caption(), "Image"),
        MessageType::Video(m) => caption_or(m.caption(), "Video"),
        MessageType::Audio(m) => caption_or(m.caption(), "Audio"),
        MessageType::File(m) => caption_or(m.caption(), "File"),
        other => other.body().to_string(),
    }
}

fn caption_or(caption: Option<&str>, kind: &str) -> String {
    caption.unwrap_or(kind).to_string()
}

/// The first line of code of a fenced block, when `body` is one.
fn first_code_line(body: &str) -> Option<&str> {
    let mut lines = body.trim_start().lines();
    lines.next().filter(|l| l.starts_with("```"))?;
    lines
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("```"))
}

/// The text of a reply without the quote some clients still put before it (the fallback).
pub fn strip_reply_fallback(body: &str) -> &str {
    let quoted = body.lines().take_while(|l| l.starts_with('>')).count();
    if quoted == 0 {
        return body;
    }
    let rest = body.splitn(quoted + 1, '\n').nth(quoted).unwrap_or("");
    rest.strip_prefix('\n').unwrap_or(rest)
}

fn is_unknown_token(error: &matrix_sdk::Error) -> bool {
    matches!(
        error.client_api_error_kind(),
        Some(ErrorKind::UnknownToken { .. })
    )
}

/// A failure of the room list: no session, or a token the server no longer knows, is
/// `NotSignedIn`; anything else is `Unavailable`.
pub fn room_list_error(error: &matrix_sdk::Error) -> RoomListError {
    match error {
        matrix_sdk::Error::AuthenticationRequired => RoomListError::NotSignedIn,
        e if is_unknown_token(e) => RoomListError::NotSignedIn,
        _ => RoomListError::Unavailable,
    }
}

/// A failure of the conversation is `Unavailable`, whatever the cause.
pub fn timeline_error(_error: &matrix_sdk::Error) -> TimelineError {
    TimelineError::Unavailable
}
