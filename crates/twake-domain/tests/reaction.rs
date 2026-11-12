//! `Reactions`: the chips under a message, from `m.reaction` events.
//!
//! Need: FR-077, D118 (few givers), D168 (own highlighted), D174 ("👍" and "👍\u{FE0F}" are the
//! same reaction, a reaction joins an existing chip by key), D583 (names in the tooltip, three
//! at most), Matrix `m.annotation` (a duplicate of the same sender and key counts once) and
//! redactions (taking one's reaction back redacts it). Specified by
//! `docs/dev/message-actions-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_domain::reaction::{ReactionEvent, Reactions};
use twake_domain::{EventId, UserId};

fn user(name: &str) -> UserId {
    UserId::parse(&format!("@{name}:example.org")).unwrap()
}

fn ev(n: usize) -> EventId {
    EventId::parse(&format!("$react{n}")).unwrap()
}

fn event(n: usize, who: &str, key: &str) -> ReactionEvent {
    ReactionEvent {
        event_id: ev(n),
        sender: user(who),
        key: key.into(),
    }
}

fn reactions(events: &[(usize, &str, &str)]) -> Reactions {
    let mut r = Reactions::new();
    for (n, who, key) in events {
        r.add(event(*n, who, key));
    }
    r
}

fn keys(r: &Reactions) -> Vec<String> {
    r.groups().iter().map(|g| g.key().to_string()).collect()
}

#[test]
fn a_message_with_no_reaction_shows_no_chip() {
    let r = Reactions::new();
    assert!(r.is_empty());
    assert!(r.groups().is_empty());
}

#[test]
fn one_reaction_is_one_chip_of_one_giver() {
    let r = reactions(&[(1, "bob", "🎉")]);
    let g = &r.groups()[0];
    assert_eq!(g.key(), "🎉");
    assert_eq!(g.count(), 1);
    assert_eq!(g.senders(), &[user("bob")]);
}

#[test]
fn several_people_on_one_emoji_make_one_chip_with_a_count() {
    let r = reactions(&[(1, "bob", "🎉"), (2, "carol", "🎉"), (3, "dave", "🎉")]);
    assert_eq!(r.groups().len(), 1);
    assert_eq!(r.groups()[0].count(), 3);
    assert_eq!(
        r.groups()[0].senders(),
        &[user("bob"), user("carol"), user("dave")]
    );
}

#[test]
fn chips_keep_the_order_of_their_first_appearance() {
    let r = reactions(&[
        (1, "bob", "😂"),
        (2, "bob", "❤️"),
        (3, "carol", "😂"),
        (4, "carol", "👀"),
        (5, "dave", "❤️"),
    ]);
    assert_eq!(keys(&r), ["😂", "❤️", "👀"]);
}

#[test]
fn the_same_sender_and_emoji_twice_counts_once() {
    let r = reactions(&[(1, "bob", "🎉"), (2, "bob", "🎉")]);
    assert_eq!(r.groups()[0].count(), 1);
    assert_eq!(r.groups()[0].senders(), &[user("bob")]);
}

#[test]
fn redacting_a_duplicate_that_was_ignored_changes_nothing() {
    let mut r = reactions(&[(1, "bob", "🎉"), (2, "bob", "🎉")]);
    assert!(!r.redact(&ev(2)));
    assert_eq!(r.groups()[0].count(), 1);
    assert_eq!(r.groups()[0].event_of(&user("bob")), Some(&ev(1)));
}

#[test]
fn the_variant_selector_does_not_make_another_chip() {
    let r = reactions(&[(1, "bob", "👍"), (2, "carol", "👍\u{FE0F}")]);
    assert_eq!(r.groups().len(), 1);
    assert_eq!(r.groups()[0].count(), 2);
}

#[test]
fn a_chip_keeps_the_spelling_first_seen() {
    let first_plain = reactions(&[(1, "bob", "👍"), (2, "carol", "👍\u{FE0F}")]);
    assert_eq!(keys(&first_plain), ["👍"]);
    let first_variant = reactions(&[(1, "bob", "👍\u{FE0F}"), (2, "carol", "👍")]);
    assert_eq!(keys(&first_variant), ["👍\u{FE0F}"]);
}

#[test]
fn the_same_sender_on_both_spellings_counts_once() {
    let r = reactions(&[(1, "bob", "👍"), (2, "bob", "👍\u{FE0F}")]);
    assert_eq!(r.groups()[0].count(), 1);
}

#[test]
fn different_emojis_stay_different_chips() {
    let r = reactions(&[(1, "bob", "👍"), (2, "bob", "👎")]);
    assert_eq!(keys(&r), ["👍", "👎"]);
}

#[test]
fn a_group_is_found_by_either_spelling() {
    let r = reactions(&[(1, "bob", "👍")]);
    assert_eq!(r.group_of("👍\u{FE0F}").map(|g| g.count()), Some(1));
    assert!(r.group_of("👎").is_none());
}

#[test]
fn a_chip_says_whether_i_reacted_and_with_which_event() {
    let r = reactions(&[(1, "bob", "🎉"), (2, "me", "🎉"), (3, "bob", "👀")]);
    let party = r.group_of("🎉").unwrap();
    assert!(party.reacted_by(&user("me")));
    assert_eq!(party.event_of(&user("me")), Some(&ev(2)));
    let eyes = r.group_of("👀").unwrap();
    assert!(!eyes.reacted_by(&user("me")));
    assert_eq!(eyes.event_of(&user("me")), None);
}

#[test]
fn taking_my_reaction_back_redacts_it_and_the_others_stay() {
    let mut r = reactions(&[(1, "bob", "🎉"), (2, "me", "🎉"), (3, "carol", "🎉")]);
    assert!(r.redact(&ev(2)));
    let g = r.group_of("🎉").unwrap();
    assert_eq!(g.count(), 2);
    assert_eq!(g.senders(), &[user("bob"), user("carol")]);
    assert!(!g.reacted_by(&user("me")));
}

#[test]
fn the_last_reaction_taken_back_removes_the_chip() {
    let mut r = reactions(&[(1, "me", "🎉"), (2, "bob", "👀")]);
    assert!(r.redact(&ev(1)));
    assert_eq!(keys(&r), ["👀"]);
}

#[test]
fn redacting_an_unknown_event_removes_nothing() {
    let mut r = reactions(&[(1, "bob", "🎉")]);
    assert!(!r.redact(&ev(99)));
    assert_eq!(r.groups()[0].count(), 1);
}

#[test]
fn redacting_twice_is_the_same_as_once() {
    let mut r = reactions(&[(1, "bob", "🎉"), (2, "carol", "🎉")]);
    assert!(r.redact(&ev(1)));
    assert!(!r.redact(&ev(1)));
    assert_eq!(r.groups()[0].count(), 1);
}

#[test]
fn an_emoji_given_again_after_its_chip_vanished_comes_last() {
    let mut r = reactions(&[(1, "me", "🎉"), (2, "bob", "👀")]);
    r.redact(&ev(1));
    r.add(event(3, "me", "🎉"));
    assert_eq!(keys(&r), ["👀", "🎉"]);
}

#[test]
fn people_are_listed_in_the_order_they_reacted_whatever_their_name() {
    let r = reactions(&[(1, "zoe", "🎉"), (2, "alice", "🎉")]);
    assert_eq!(r.groups()[0].senders(), &[user("zoe"), user("alice")]);
}

proptest! {
    #[test]
    fn counts_equal_distinct_senders_and_no_chip_is_empty(
        evs in prop::collection::vec((0usize..4, 0usize..3), 0..30)
    ) {
        let who = ["ann", "ben", "cat", "dan"];
        let emoji = ["👍", "👍\u{FE0F}", "🎉"];
        let mut r = Reactions::new();
        for (i, (w, e)) in evs.iter().enumerate() {
            r.add(event(i, who[*w], emoji[*e]));
        }
        for g in r.groups() {
            let mut s: Vec<_> = g.senders().to_vec();
            s.sort_by_key(|u| u.to_string());
            s.dedup();
            prop_assert_eq!(s.len(), g.senders().len());
            prop_assert_eq!(g.count(), g.senders().len());
            prop_assert!(g.count() >= 1);
        }
        // 👍 and 👍\u{FE0F} are one chip: at most two chips whatever the events
        prop_assert!(r.groups().len() <= 2);
    }
}
