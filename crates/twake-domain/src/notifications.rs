//! What a room does with its notifications, from its own choice and the account default.

use crate::preferences::GlobalNotificationLevel;

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

/// The level of a room: its own choice, or the account default.
pub fn effective_level(
    room: RoomNotificationLevel,
    global: GlobalNotificationLevel,
) -> NotificationLevel {
    match (room, global) {
        (RoomNotificationLevel::All, _) => NotificationLevel::All,
        (RoomNotificationLevel::Mentions, _) => NotificationLevel::Mentions,
        (RoomNotificationLevel::None, _) => NotificationLevel::None,
        (RoomNotificationLevel::Default, GlobalNotificationLevel::All) => NotificationLevel::All,
        (RoomNotificationLevel::Default, GlobalNotificationLevel::Mentions) => {
            NotificationLevel::Mentions
        }
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

    /// The badge is grey for `Mentions` and `None` (D615).
    pub fn is_muted(self) -> bool {
        self != Self::All
    }
}
