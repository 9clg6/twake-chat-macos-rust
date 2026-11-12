//! What the conversation shows: the diffs of the stream, with day dividers and the marker of new messages.

use twake_domain::timeline::{Message, TimelineDiff, TimelineItem};
use twake_domain::{Day, EventId};

use crate::ports::TimelineError;

/// What the screen says about the conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineStatus {
    /// Nothing received yet.
    Loading,
    /// No message.
    Empty,
    /// Messages to show.
    Ready,
    /// The stream failed; the next `Reset` recovers.
    Failed(TimelineError),
}

/// The conversation model of a screen.
#[derive(Debug)]
pub struct TimelineState {
    utc_offset_secs: i32,
    messages: Vec<Message>,
    loaded: bool,
    reached_start: bool,
    first_unread: Option<EventId>,
    failure: Option<TimelineError>,
}

/// Replaces the same message in place, else puts it last.
fn upsert(list: &mut Vec<Message>, message: Message) {
    match list.iter_mut().find(|m| m.is_same_as(&message)) {
        Some(known) => *known = message,
        None => list.push(message),
    }
}

impl TimelineState {
    /// Loading and empty; the day dividers follow `utc_offset_secs`.
    pub fn new(utc_offset_secs: i32) -> Self {
        Self {
            utc_offset_secs,
            messages: Vec::new(),
            loaded: false,
            reached_start: false,
            first_unread: None,
            failure: None,
        }
    }

    /// Applies one diff.
    pub fn apply(&mut self, diff: TimelineDiff) {
        match diff {
            TimelineDiff::Reset(all) => self.reset(all),
            TimelineDiff::Append(list) => {
                list.into_iter().for_each(|m| upsert(&mut self.messages, m))
            }
            TimelineDiff::Prepend {
                messages,
                reached_start,
            } => self.prepend(messages, reached_start),
            TimelineDiff::Update(message) => {
                if let Some(known) = self.messages.iter_mut().find(|m| m.is_same_as(&message)) {
                    *known = message;
                }
            }
            TimelineDiff::Remove(key) => self.messages.retain(|m| !m.matches(&key)),
        }
    }

    fn reset(&mut self, all: Vec<Message>) {
        self.messages.clear();
        all.into_iter().for_each(|m| upsert(&mut self.messages, m));
        self.loaded = true;
        self.reached_start = false;
        self.first_unread = None;
        self.failure = None;
    }

    fn prepend(&mut self, page: Vec<Message>, reached_start: bool) {
        let mut older: Vec<Message> = Vec::new();
        for message in page {
            let held = self
                .messages
                .iter()
                .chain(&older)
                .any(|m| m.is_same_as(&message));
            if !held {
                older.push(message);
            }
        }
        self.messages.splice(0..0, older);
        self.reached_start |= reached_start;
    }

    /// Puts the "New messages" marker before this event, once it is held.
    pub fn mark_new_messages_from(&mut self, first_unread: EventId) {
        self.first_unread = Some(first_unread);
    }

    /// The rows to draw: derived, never stored.
    pub fn items(&self) -> Vec<TimelineItem> {
        let mut items = Vec::new();
        let mut current: Option<Day> = None;
        for message in &self.messages {
            let day = Day::of(message.timestamp, self.utc_offset_secs);
            if current != Some(day) {
                items.push(TimelineItem::DayDivider(day));
                current = Some(day);
            }
            if self.first_unread.is_some() && message.event_id() == self.first_unread.as_ref() {
                items.push(TimelineItem::NewMessages);
            }
            items.push(TimelineItem::Message(message.clone()));
        }
        items
    }

    /// The start of the history was reached.
    pub fn reached_start(&self) -> bool {
        self.reached_start
    }

    /// Records a failure; the messages are kept.
    pub fn fail(&mut self, error: TimelineError) {
        self.failure = Some(error);
    }

    /// What the screen says about the conversation.
    pub fn status(&self) -> TimelineStatus {
        if let Some(error) = &self.failure {
            TimelineStatus::Failed(error.clone())
        } else if !self.loaded {
            TimelineStatus::Loading
        } else if self.messages.is_empty() {
            TimelineStatus::Empty
        } else {
            TimelineStatus::Ready
        }
    }
}
