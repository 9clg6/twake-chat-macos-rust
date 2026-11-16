//! What a bubble shows and how consecutive messages group; day dividers and the new messages marker.
//! Rules: D138, D297, D470, D495, D500, D519, D523.

use std::collections::HashMap;

use twake_domain::timeline::{Message, SendState, TimelineItem};
use twake_domain::{Day, EventId, Timestamp, UserId};
use twake_i18n::Locale;

use crate::avatar::AvatarView;
use crate::clock::{age_in_days, clock_text, short_date};

/// A message within this delay after the one before joins its run (5 minutes: the spec is silent).
pub const GROUP_GAP_MS: u64 = 300_000;
/// Longest quote, in characters, the ellipsis included.
pub const QUOTE_MAX_CHARS: usize = 120;

/// What the rows need to know besides the items.
pub struct TimelineContext<'a> {
    /// The user.
    pub me: &'a UserId,
    /// A direct room shows no author name.
    pub is_direct: bool,
    /// Display names; an unknown person shows as the Matrix id.
    pub names: &'a HashMap<UserId, String>,
    /// The newest event any other member has read.
    pub read_by_others_up_to: Option<&'a EventId>,
}

/// Which side a bubble sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The user's, on the right.
    Own,
    /// Someone else's, on the left.
    Other,
}

/// The state of an own message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ticks {
    /// In flight.
    Clock,
    /// The server has it.
    Sent,
    /// Someone read it.
    Read,
    /// Not sent.
    Failed,
}

/// The message a reply answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuoteView {
    /// The answered event.
    pub target: EventId,
    /// Its author.
    pub author: String,
    /// Its text, cut when long.
    pub text: String,
}

/// A message bubble.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BubbleView {
    /// Right for the user, left for the others.
    pub side: Side,
    /// Display name of the author.
    pub author: String,
    /// Initials and colour of the author.
    pub avatar: AvatarView,
    /// The name stands above the first bubble of a run of someone else, not in a direct room.
    pub shows_name: bool,
    /// The avatar stands beside the last bubble of a run of someone else.
    pub shows_avatar: bool,
    /// The text.
    pub body: String,
    /// The clock of the reader.
    pub time: String,
    /// Own messages only.
    pub ticks: Option<Ticks>,
    /// `(edited)` once edited.
    pub edited: Option<String>,
    /// The answered message, when loaded.
    pub quote: Option<QuoteView>,
}

/// The line above the messages of a day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayDividerView {
    /// `Today`, `Yesterday` or a short date.
    pub label: String,
}

impl DayDividerView {
    /// The divider of `day` for a reader at `now`.
    pub fn from(day: Day, now: Timestamp, locale: &Locale) -> Self {
        let label = match age_in_days(day, now, locale) {
            0 => locale.tr("day-today", &[]),
            1 => locale.tr("day-yesterday", &[]),
            _ => short_date(day, locale),
        };
        Self { label }
    }
}

/// The "New messages" marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMessagesView {
    /// The text.
    pub label: String,
}

impl NewMessagesView {
    /// The marker in the language of the reader.
    pub fn from(locale: &Locale) -> Self {
        Self {
            label: locale.tr("new-messages", &[]),
        }
    }
}

/// A row of the conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowView {
    /// A day divider.
    Day(DayDividerView),
    /// The new messages marker.
    NewMessages(NewMessagesView),
    /// A message.
    Bubble(BubbleView),
}

/// One row per item, in the same order.
pub fn timeline_rows(
    items: &[TimelineItem],
    ctx: &TimelineContext,
    now: Timestamp,
    locale: &Locale,
) -> Vec<RowView> {
    let read_index = ctx
        .read_by_others_up_to
        .and_then(|read| items.iter().position(|item| event_of(item) == Some(read)));
    items
        .iter()
        .enumerate()
        .map(|(index, item)| match item {
            TimelineItem::DayDivider(day) => RowView::Day(DayDividerView::from(*day, now, locale)),
            TimelineItem::NewMessages => RowView::NewMessages(NewMessagesView::from(locale)),
            TimelineItem::Message(message) => {
                let placed = Placed {
                    items,
                    index,
                    read_index,
                };
                RowView::Bubble(bubble(message, &placed, ctx, locale))
            }
        })
        .collect()
}

fn event_of(item: &TimelineItem) -> Option<&EventId> {
    match item {
        TimelineItem::Message(message) => message.event_id(),
        _ => None,
    }
}

/// Where a message stands in the list.
struct Placed<'a> {
    items: &'a [TimelineItem],
    index: usize,
    read_index: Option<usize>,
}

fn name_of(ctx: &TimelineContext, user: &UserId) -> String {
    ctx.names
        .get(user)
        .cloned()
        .unwrap_or_else(|| user.as_str().to_owned())
}

/// `later` is in the run of `earlier`: same sender, at most `GROUP_GAP_MS` after it.
fn continues(earlier: Option<&TimelineItem>, later: Option<&TimelineItem>) -> bool {
    match (earlier, later) {
        (Some(TimelineItem::Message(a)), Some(TimelineItem::Message(b))) => {
            a.sender == b.sender
                && b.timestamp
                    .as_millis()
                    .saturating_sub(a.timestamp.as_millis())
                    <= GROUP_GAP_MS
        }
        _ => false,
    }
}

fn bubble(message: &Message, at: &Placed, ctx: &TimelineContext, locale: &Locale) -> BubbleView {
    let own = &message.sender == ctx.me;
    let before = at.index.checked_sub(1).and_then(|i| at.items.get(i));
    let current = at.items.get(at.index);
    let after = at.items.get(at.index + 1);
    let is_first = !continues(before, current);
    let is_last = !continues(current, after);
    let author = name_of(ctx, &message.sender);
    BubbleView {
        side: if own { Side::Own } else { Side::Other },
        avatar: AvatarView::of(&author, message.sender.as_str()),
        shows_name: is_first && !own && !ctx.is_direct,
        shows_avatar: is_last && !own,
        body: message.body.clone(),
        time: clock_text(message.timestamp, locale),
        ticks: own.then(|| ticks(message, at)),
        edited: message.edited.then(|| locale.tr("message-edited", &[])),
        quote: quote(message, at.items, ctx),
        author,
    }
}

fn ticks(message: &Message, at: &Placed) -> Ticks {
    match message.send_state {
        SendState::Sending => Ticks::Clock,
        SendState::Failed => Ticks::Failed,
        SendState::Sent if at.read_index.is_some_and(|read| at.index <= read) => Ticks::Read,
        SendState::Sent => Ticks::Sent,
    }
}

// ponytail: a linear search per reply; an index by event id if replies ever dominate a 10k timeline.
fn quote(message: &Message, items: &[TimelineItem], ctx: &TimelineContext) -> Option<QuoteView> {
    let target = message.reply_to.as_ref()?;
    let answered = items.iter().find_map(|item| match item {
        TimelineItem::Message(m) if m.event_id() == Some(target) => Some(m),
        _ => None,
    })?;
    Some(QuoteView {
        target: target.clone(),
        author: name_of(ctx, &answered.sender),
        text: cut(&answered.body),
    })
}

fn cut(text: &str) -> String {
    if text.chars().count() <= QUOTE_MAX_CHARS {
        return text.to_owned();
    }
    let mut cut: String = text.chars().take(QUOTE_MAX_CHARS - 1).collect();
    cut.push('…');
    cut
}
