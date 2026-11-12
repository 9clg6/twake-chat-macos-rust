//! What a room row shows: title, preview, time, unread badge, avatar, pin and mute marks.
//! Rules: D31, D175, D170, D190, D222, D247, D608, D615.

use twake_domain::room_list::{LastSender, RoomKind, RoomSummary};
use twake_domain::{Day, Timestamp};
use twake_i18n::{Arg, Locale};

use crate::avatar::AvatarView;
use crate::clock::{clock_text, local_day, short_date, weekday_key};

/// Largest count shown as is.
const BADGE_MAX: u32 = 99;
/// Days (before today) that show a weekday: "this week" is a rolling six days.
const WEEK_DAYS: i64 = 6;

/// How the badge is coloured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeStyle {
    /// A count.
    Normal,
    /// A mention.
    Mention,
    /// A muted room, grey.
    Muted,
}

/// The unread mark at the end of the row; an empty text is a dot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadgeView {
    /// `7`, `99+`, `@` or empty.
    pub text: String,
    /// Its colour.
    pub style: BadgeStyle,
}

/// A row of the room list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomRowView {
    /// The name of the room.
    pub title: String,
    /// The last message; none for a space or an empty room.
    pub preview: Option<String>,
    /// When; none while the badge shows.
    pub time: Option<String>,
    /// The unread mark.
    pub badge: Option<BadgeView>,
    /// Initials and colour.
    pub avatar: AvatarView,
    /// A favourite.
    pub is_pinned: bool,
    /// Notifications off.
    pub is_muted: bool,
}

impl RoomRowView {
    /// The row of `room` for a reader at `now`.
    pub fn from(room: &RoomSummary, now: Timestamp, locale: &Locale) -> Self {
        let badge = badge(room);
        let time = badge
            .is_none()
            .then(|| time_text(room.last_activity, now, locale));
        Self {
            title: room.name.clone(),
            preview: preview(room, locale),
            time,
            badge,
            avatar: AvatarView::of(&room.name, room.id.as_str()),
            is_pinned: room.is_favourite,
            is_muted: room.is_muted,
        }
    }
}

fn preview(room: &RoomSummary, locale: &Locale) -> Option<String> {
    let text = room.last_message_preview.as_deref()?;
    if matches!(room.kind, RoomKind::Space) {
        return None;
    }
    let shown = match (&room.kind, &room.last_sender) {
        (RoomKind::Direct, _) | (_, None) => text.to_owned(),
        (_, Some(LastSender::Me)) => locale.tr("room-preview-you", &[("text", Arg::Str(text))]),
        (_, Some(LastSender::Other(author))) => locale.tr(
            "room-preview-author",
            &[("author", Arg::Str(author)), ("text", Arg::Str(text))],
        ),
    };
    Some(shown)
}

fn badge(room: &RoomSummary) -> Option<BadgeView> {
    let unread = room.unread;
    if !unread.is_unread() {
        return None;
    }
    let text = if unread.mentions > 0 {
        "@".to_owned()
    } else if unread.unread > BADGE_MAX {
        format!("{BADGE_MAX}+")
    } else if unread.unread > 0 {
        unread.unread.to_string()
    } else {
        String::new()
    };
    let style = if room.is_muted {
        BadgeStyle::Muted
    } else if unread.mentions > 0 {
        BadgeStyle::Mention
    } else {
        BadgeStyle::Normal
    };
    Some(BadgeView { text, style })
}

fn time_text(activity: Timestamp, now: Timestamp, locale: &Locale) -> String {
    let age = local_day(now, locale) - local_day(activity, locale);
    match age {
        0 => clock_text(activity, locale),
        1 => locale.tr("time-yesterday", &[]),
        2..=WEEK_DAYS => locale.tr(weekday_key(local_day(activity, locale)), &[]),
        _ => short_date(Day::of(activity, locale.utc_offset_secs()), locale),
    }
}
