//! Notification levels of a room and their effect.
//!
//! Need (not the code): notifications.md D22 (a room is at every message, mentions only
//! ("muted") or nothing; "nothing" silences mentions too; a room without a choice follows the
//! account default), D85 ("a room at nothing stays silent"), D615 (muted badges are grey).
//! React `shouldNotify`: a mention notifies unless the room says nothing at all.
//!
//! Public items used are listed in `docs/dev/settings-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use twake_domain::notifications::{NotificationLevel, RoomNotificationLevel, effective_level};
use twake_domain::preferences::GlobalNotificationLevel as Global;

#[test]
fn a_room_without_a_choice_follows_the_account_default() {
    assert_eq!(
        effective_level(RoomNotificationLevel::Default, Global::All),
        NotificationLevel::All
    );
    assert_eq!(
        effective_level(RoomNotificationLevel::Default, Global::Mentions),
        NotificationLevel::Mentions
    );
}

#[test]
fn a_room_with_a_choice_ignores_the_account_default() {
    for global in [Global::All, Global::Mentions] {
        assert_eq!(
            effective_level(RoomNotificationLevel::All, global),
            NotificationLevel::All
        );
        assert_eq!(
            effective_level(RoomNotificationLevel::Mentions, global),
            NotificationLevel::Mentions
        );
        assert_eq!(
            effective_level(RoomNotificationLevel::None, global),
            NotificationLevel::None
        );
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
fn only_all_is_not_muted() {
    assert!(!NotificationLevel::All.is_muted());
    assert!(NotificationLevel::Mentions.is_muted());
    assert!(NotificationLevel::None.is_muted());
}

#[test]
fn a_default_room_under_a_mentions_account_is_muted_but_still_tells_mentions() {
    let level = effective_level(RoomNotificationLevel::Default, Global::Mentions);
    assert!(level.is_muted());
    assert!(level.notifies(true));
    assert!(!level.notifies(false));
}
