//! What a room does with its notifications, from its own choice and the account default.

use crate::preferences::{GlobalNotificationLevel, NotificationDefaults};

/// What a room chooses for itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomNotificationLevel {
    /// Follow the account default.
    Default,
    /// Every message.
    All,
    /// Mentions only.
    Mentions,
    /// Nothing, a mention neither.
    None,
}

/// The level that applies to a room.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    /// Every message.
    All,
    /// Mentions only.
    Mentions,
    /// Nothing.
    None,
}

/// Direct room or group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomKind {
    /// A direct room.
    Direct,
    /// A group.
    Group,
}

/// The level of a room: its own choice, or the account default of its kind.
pub fn effective_level(
    room: RoomNotificationLevel,
    kind: RoomKind,
    defaults: NotificationDefaults,
) -> NotificationLevel {
    let global = match (room, kind) {
        (RoomNotificationLevel::All, _) => return NotificationLevel::All,
        (RoomNotificationLevel::Mentions, _) => return NotificationLevel::Mentions,
        (RoomNotificationLevel::None, _) => return NotificationLevel::None,
        (RoomNotificationLevel::Default, RoomKind::Direct) => defaults.direct_messages,
        (RoomNotificationLevel::Default, RoomKind::Group) => defaults.groups,
    };
    match global {
        GlobalNotificationLevel::All => NotificationLevel::All,
        GlobalNotificationLevel::Mentions => NotificationLevel::Mentions,
    }
}

impl NotificationLevel {
    /// Whether a message notifies; `None` never does, a mention neither (D22).
    pub fn notifies(self, is_mention: bool) -> bool {
        match self {
            Self::All => true,
            Self::Mentions => is_mention,
            Self::None => false,
        }
    }

    /// Muted is nothing at all (D615).
    pub fn is_muted(self) -> bool {
        self == Self::None
    }
}
