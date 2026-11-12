//! `PowerLevels`: who may redact, pin, react and write in a room.
//!
//! Need: D77 (Delete offered on others' messages when power levels allow `redact`, 50 by
//! default), D260 (pin only when the user may send that state), D90 and D109 (a broadcast room:
//! `events_default` 50, only moderators post), Matrix `m.room.power_levels` defaults (D418:
//! `users_default` 0, `events_default` 0, `state_default` 50, `redact` 50, `ban` 50; a user
//! may always redact their own event). Specified by `docs/dev/message-actions-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_domain::UserId;
use twake_domain::power_levels::{PowerLevels, can_redact};

fn user(name: &str) -> UserId {
    UserId::parse(&format!("@{name}:example.org")).unwrap()
}

#[test]
fn a_room_with_no_power_levels_event_follows_the_spec_defaults() {
    let p = PowerLevels::new();
    let nobody = user("nobody");
    assert_eq!(p.level_of(&nobody), 0);
    assert!(p.can_send_message(&nobody));
    assert!(p.can_send_reaction(&nobody));
    assert!(!p.can_pin(&nobody));
    assert!(!p.can_ban(&nobody));
}

#[test]
fn a_user_has_the_level_listed_for_them_else_the_default() {
    let p = PowerLevels::new()
        .with_user(user("admin"), 100)
        .with_users_default(10);
    assert_eq!(p.level_of(&user("admin")), 100);
    assert_eq!(p.level_of(&user("bob")), 10);
}

#[test]
fn a_negative_level_is_kept() {
    let p = PowerLevels::new().with_user(user("muted"), -1);
    assert_eq!(p.level_of(&user("muted")), -1);
    assert!(!p.can_send_message(&user("muted")));
}

#[test]
fn a_moderator_pins_and_a_member_does_not() {
    let p = PowerLevels::new().with_user(user("mod"), 50);
    assert!(p.can_pin(&user("mod")));
    assert!(!p.can_pin(&user("bob")));
}

#[test]
fn the_pin_level_follows_the_event_entry_before_state_default() {
    let p = PowerLevels::new().with_event("m.room.pinned_events", 0);
    assert!(p.can_pin(&user("bob")));
    let strict = PowerLevels::new()
        .with_state_default(10)
        .with_event("m.room.pinned_events", 75)
        .with_user(user("mod"), 50);
    assert!(!strict.can_pin(&user("mod")));
}

#[test]
fn without_an_event_entry_pinning_follows_state_default() {
    let p = PowerLevels::new()
        .with_state_default(20)
        .with_user(user("a"), 20)
        .with_user(user("b"), 19);
    assert!(p.can_pin(&user("a")));
    assert!(!p.can_pin(&user("b")));
}

#[test]
fn a_broadcast_room_lets_only_moderators_write() {
    let p = PowerLevels::new()
        .with_events_default(50)
        .with_user(user("mod"), 50);
    assert!(p.can_send_message(&user("mod")));
    assert!(!p.can_send_message(&user("bob")));
    assert!(!p.can_send_reaction(&user("bob")));
}

#[test]
fn reactions_can_be_restricted_on_their_own() {
    let p = PowerLevels::new()
        .with_event("m.reaction", 50)
        .with_user(user("mod"), 50);
    assert!(p.can_send_message(&user("bob")));
    assert!(!p.can_send_reaction(&user("bob")));
    assert!(p.can_send_reaction(&user("mod")));
}

#[test]
fn reactions_can_stay_open_in_a_room_where_writing_is_not() {
    let p = PowerLevels::new()
        .with_events_default(50)
        .with_event("m.reaction", 0);
    assert!(!p.can_send_message(&user("bob")));
    assert!(p.can_send_reaction(&user("bob")));
}

#[test]
fn an_event_type_not_listed_uses_events_default_and_a_state_type_state_default() {
    let p = PowerLevels::new()
        .with_events_default(5)
        .with_state_default(60)
        .with_user(user("a"), 5)
        .with_user(user("b"), 60);
    assert!(p.can_send_event(&user("a"), "m.sticker"));
    assert!(!p.can_send_state(&user("a"), "m.room.topic"));
    assert!(p.can_send_state(&user("b"), "m.room.topic"));
}

#[test]
fn banning_needs_the_ban_level() {
    let p = PowerLevels::new().with_ban(75).with_user(user("mod"), 50);
    assert!(!p.can_ban(&user("mod")));
    assert!(!p.can_ban(&user("admin")));
    let p = p.with_user(user("admin"), 75);
    assert!(p.can_ban(&user("admin")));
}

#[test]
fn one_may_always_redact_their_own_event() {
    let p = PowerLevels::new().with_redact(100);
    assert!(p.can_redact(&user("bob"), &user("bob")));
}

#[test]
fn redacting_someone_else_needs_the_redact_level() {
    let p = PowerLevels::new().with_user(user("mod"), 50);
    assert!(p.can_redact(&user("mod"), &user("bob")));
    assert!(!p.can_redact(&user("bob"), &user("mod")));
    let strict = p.with_redact(51);
    assert!(!strict.can_redact(&user("mod"), &user("bob")));
}

#[test]
fn in_a_v12_room_the_creator_has_infinite_power() {
    let p = PowerLevels::new()
        .with_events_default(100)
        .with_state_default(100)
        .with_redact(100)
        .with_ban(100)
        .with_event("m.reaction", 1000)
        .with_creator(user("boss"))
        .with_creator_infinite(true);
    let boss = user("boss");
    assert!(p.can_send_message(&boss) && p.can_send_reaction(&boss));
    assert!(p.can_pin(&boss) && p.can_ban(&boss));
    assert!(p.can_redact(&boss, &user("bob")));
    assert!(p.level_of(&boss) > 1_000_000);
}

#[test]
fn infinite_power_belongs_to_the_creator_alone() {
    let p = PowerLevels::new()
        .with_creator(user("boss"))
        .with_creator_infinite(true);
    assert!(!p.can_pin(&user("bob")));
    assert!(!p.can_redact(&user("bob"), &user("boss")));
    assert_eq!(p.level_of(&user("bob")), 0);
}

#[test]
fn before_v12_the_creator_has_only_the_level_listed() {
    let p = PowerLevels::new()
        .with_creator(user("boss"))
        .with_user(user("boss"), 100)
        .with_state_default(150);
    assert_eq!(p.level_of(&user("boss")), 100);
    assert!(!p.can_pin(&user("boss")));
}

#[test]
fn a_room_without_the_flag_has_no_infinite_creator_by_default() {
    let p = PowerLevels::new()
        .with_creator(user("boss"))
        .with_state_default(150);
    assert!(!p.can_pin(&user("boss")));
}

#[test]
fn the_redact_rule_alone() {
    assert!(can_redact(true, -5, 100));
    assert!(can_redact(false, 50, 50));
    assert!(!can_redact(false, 49, 50));
    assert!(can_redact(false, 100, 50));
}

proptest! {
    #[test]
    fn more_power_never_removes_a_right(level in -10i64..120, extra in 0i64..50, required in -5i64..110) {
        let lo = PowerLevels::new().with_redact(required).with_user(user("u"), level);
        let hi = PowerLevels::new().with_redact(required).with_user(user("u"), level + extra);
        if lo.can_redact(&user("u"), &user("v")) {
            prop_assert!(hi.can_redact(&user("u"), &user("v")));
        }
        prop_assert_eq!(can_redact(false, level, required), level >= required);
    }
}
