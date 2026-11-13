//! A message of the SDK timeline as a domain `Message` (A031).

use matrix_sdk_ui::timeline::{
    EventTimelineItem, MsgLikeKind, TimelineDetails, TimelineItemContent,
};
use twake_domain::timeline::{Message, MessageKey, SendState};
use twake_domain::{EventId, TxnId, UserId};

use crate::map;

/// The message an event item shows; none for what is not a message of the conversation yet
/// (a deleted message, a state event, a sticker, a poll, a message that cannot be read).
pub(crate) fn to_message(item: &EventTimelineItem) -> Option<Message> {
    let TimelineItemContent::MsgLike(like) = item.content() else {
        return None;
    };
    let MsgLikeKind::Message(message) = &like.kind else {
        return None;
    };
    let sender = UserId::parse(item.sender().as_str()).ok()?;
    let reply = like
        .in_reply_to
        .as_ref()
        .and_then(|r| EventId::parse(r.event_id.as_str()).ok());
    let name = map::name_of(&sender, display_name(item));
    let mut body = map::message_body(message.msgtype(), &name);
    if reply.is_some() {
        body = map::strip_reply_fallback(&body).to_string();
    }
    let mut out = identified(item, sender, body)?;
    out.send_state = item.send_state().map_or(SendState::Sent, map::send_state);
    out.edited = message.is_edited();
    out.reply_to = reply;
    Some(out)
}

fn display_name(item: &EventTimelineItem) -> Option<&str> {
    match item.sender_profile() {
        TimelineDetails::Ready(profile) => profile.display_name.as_deref(),
        _ => None,
    }
}

/// The message with its identity: the event id once the server has one, the transaction id of
/// a send (kept beside the event id until the remote echo comes).
fn identified(item: &EventTimelineItem, sender: UserId, body: String) -> Option<Message> {
    let ts = map::timestamp(item.timestamp());
    let event = item
        .event_id()
        .and_then(|e| EventId::parse(e.as_str()).ok());
    let txn = item
        .transaction_id()
        .and_then(|t| TxnId::parse(t.as_str()).ok());
    match (event, txn) {
        (Some(event), Some(txn)) => Some(Message::remote(event, sender, ts, body).with_txn_id(txn)),
        (Some(event), None) => Some(Message::remote(event, sender, ts, body)),
        (None, Some(txn)) => Some(Message::local(txn, sender, ts, body)),
        (None, None) => None,
    }
}

/// How a diff designates `message`: by event id, else by transaction id.
pub(crate) fn key_of(message: &Message) -> Option<MessageKey> {
    match (message.event_id(), message.txn_id()) {
        (Some(event), _) => Some(MessageKey::Event(event.clone())),
        (None, Some(txn)) => Some(MessageKey::Txn(txn.clone())),
        (None, None) => None,
    }
}
