//! Messages of a conversation, their identity while being sent, and the diffs of the stream.
//!
//! Implements D495 (an echo and its confirmation are one bubble), D523 (new messages marker).

use crate::{Day, EventId, Timestamp, TxnId, UserId};

/// Where a message is on its way to the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendState {
    /// Local echo, not accepted yet.
    Sending,
    /// Accepted by the server.
    Sent,
    /// Could not be sent.
    Failed,
}

/// A message of a conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Who wrote it.
    pub sender: UserId,
    /// When.
    pub timestamp: Timestamp,
    /// Its text.
    pub body: String,
    /// The event it answers.
    pub reply_to: Option<EventId>,
    /// Edited after sending.
    pub edited: bool,
    /// Its way to the server.
    pub send_state: SendState,
    event_id: Option<EventId>,
    txn_id: Option<TxnId>,
}

impl Message {
    fn new(
        event_id: Option<EventId>,
        txn_id: Option<TxnId>,
        sender: UserId,
        timestamp: Timestamp,
        body: String,
        send_state: SendState,
    ) -> Self {
        Self {
            sender,
            timestamp,
            body,
            reply_to: None,
            edited: false,
            send_state,
            event_id,
            txn_id,
        }
    }

    /// A message the server holds.
    pub fn remote(event: EventId, sender: UserId, ts: Timestamp, body: impl Into<String>) -> Self {
        Self::new(Some(event), None, sender, ts, body.into(), SendState::Sent)
    }

    /// A local echo, known by its transaction id only.
    pub fn local(txn: TxnId, sender: UserId, ts: Timestamp, body: impl Into<String>) -> Self {
        Self::new(None, Some(txn), sender, ts, body.into(), SendState::Sending)
    }

    /// Records the transaction that made this server message.
    pub fn with_txn_id(mut self, txn: TxnId) -> Self {
        self.txn_id = Some(txn);
        self
    }

    /// Records the event it answers.
    pub fn with_reply_to(mut self, event: EventId) -> Self {
        self.reply_to = Some(event);
        self
    }

    /// Marks it edited.
    pub fn edited(mut self) -> Self {
        self.edited = true;
        self
    }

    /// Marks it failed.
    pub fn failed(mut self) -> Self {
        self.send_state = SendState::Failed;
        self
    }

    /// The event id, once the server has one.
    pub fn event_id(&self) -> Option<&EventId> {
        self.event_id.as_ref()
    }

    /// The transaction id of a send.
    pub fn txn_id(&self) -> Option<&TxnId> {
        self.txn_id.as_ref()
    }

    /// Whether `key` designates this message.
    pub fn matches(&self, key: &MessageKey) -> bool {
        match key {
            MessageKey::Event(event) => self.event_id.as_ref() == Some(event),
            MessageKey::Txn(txn) => self.txn_id.as_ref() == Some(txn),
        }
    }

    /// Whether `other` is this same message: same event id, or same transaction id.
    pub fn is_same_as(&self, other: &Self) -> bool {
        (self.event_id.is_some() && self.event_id == other.event_id)
            || (self.txn_id.is_some() && self.txn_id == other.txn_id)
    }
}

/// How a diff designates a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageKey {
    /// By event id.
    Event(EventId),
    /// By transaction id.
    Txn(TxnId),
}

/// A row of the conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineItem {
    /// The start of a calendar day.
    DayDivider(Day),
    /// The "New messages" marker.
    NewMessages,
    /// A message.
    Message(Message),
}

/// A change of the conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineDiff {
    /// The room was opened or switched.
    Reset(Vec<Message>),
    /// Newer messages and local echoes.
    Append(Vec<Message>),
    /// An older page, oldest first.
    Prepend {
        /// The page.
        messages: Vec<Message>,
        /// No message remains before it.
        reached_start: bool,
    },
    /// Replaces a message by event id or transaction id; an unknown one is ignored.
    Update(Message),
    /// Drops a message; an unknown one is ignored.
    Remove(MessageKey),
}
