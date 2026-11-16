//! Notification levels of a room and their effect.
//!
//! Need (not the code): notifications.md D22 (a room is at every message, mentions only or
//! nothing; "nothing" silences mentions too; a room without a choice follows the account default
//! of its kind: one for direct messages, one for groups), D85 ("a room at nothing stays silent").
//! Owner's rulings: a room set to "All" notifies whatever the account default ("it should get
//! the notifications it asks for"); "muted" means nothing at all, never mentions only (D615,
//! the grey badge).
//!
//! Public items used are listed in `docs/dev/settings-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use twake_domain::notifications::{
    NotificationLevel, RoomKind, RoomNotificationLevel, effective_level,
};
use twake_domain::preferences::GlobalNotificationLevel as Global;
use twake_domain::preferences::NotificationDefaults;

fn defaults(direct_messages: Global, groups: Global) -> NotificationDefaults {
    NotificationDefaults {
        direct_messages,
        groups,
    }
}

const KINDS: [RoomKind; 2] = [RoomKind::Direct, RoomKind::Group];
const LEVELS: [Global; 2] = [Global::All, Global::Mentions];

#[test]
fn a_direct_room_without_a_choice_follows_the_direct_default() {
    assert_eq!(
        effective_level(
            RoomNotificationLevel::Default,
            RoomKind::Direct,
            defaults(Global::Mentions, Global::All)
        ),
        NotificationLevel::Mentions
    );
    assert_eq!(
        effective_level(
            RoomNotificationLevel::Default,
            RoomKind::Direct,
            defaults(Global::All, Global::Mentions)
        ),
        NotificationLevel::All
    );
}

#[test]
fn a_group_without_a_choice_follows_the_group_default() {
    assert_eq!(
        effective_level(
            RoomNotificationLevel::Default,
            RoomKind::Group,
            defaults(Global::All, Global::Mentions)
        ),
        NotificationLevel::Mentions
    );
    assert_eq!(
        effective_level(
            RoomNotificationLevel::Default,
            RoomKind::Group,
            defaults(Global::Mentions, Global::All)
        ),
        NotificationLevel::All
    );
}

#[test]
fn a_room_with_a_choice_ignores_both_defaults() {
    for kind in KINDS {
        for direct in LEVELS {
            for group in LEVELS {
                let d = defaults(direct, group);
                assert_eq!(
                    effective_level(RoomNotificationLevel::Mentions, kind, d),
                    NotificationLevel::Mentions
                );
                assert_eq!(
                    effective_level(RoomNotificationLevel::None, kind, d),
                    NotificationLevel::None
                );
            }
        }
    }
}

#[test]
fn a_room_set_to_all_notifies_whatever_the_account_default() {
    for kind in KINDS {
        for direct in LEVELS {
            for group in LEVELS {
                let level =
                    effective_level(RoomNotificationLevel::All, kind, defaults(direct, group));
                assert_eq!(level, NotificationLevel::All);
                assert!(level.notifies(false));
            }
        }
    }
}

#[test]
fn every_message_notifies_at_all() {
    assert!(NotificationLevel::All.notifies(false));
    assert!(NotificationLevel::All.notifies(true));
}

#[test]
fn mentions_only_notifies_a_mention_alone() {
    assert!(NotificationLevel::Mentions.notifies(true));
    assert!(!NotificationLevel::Mentions.notifies(false));
}

#[test]
fn nothing_silences_even_a_mention() {
    assert!(!NotificationLevel::None.notifies(true));
    assert!(!NotificationLevel::None.notifies(false));
}

#[test]
fn only_nothing_is_muted() {
    assert!(!NotificationLevel::All.is_muted());
    assert!(!NotificationLevel::Mentions.is_muted());
    assert!(NotificationLevel::None.is_muted());
}

#[test]
fn a_room_at_mentions_only_is_not_muted_and_still_tells_mentions() {
    let level = effective_level(
        RoomNotificationLevel::Default,
        RoomKind::Group,
        defaults(Global::All, Global::Mentions),
    );
    assert!(!level.is_muted());
    assert!(level.notifies(true));
    assert!(!level.notifies(false));
}
