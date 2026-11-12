//! A click on an emoji: send my reaction, or take it back by redacting it.
//!
//! Need: FR-075, FR-077, D168 (own reactions highlighted; the chip is clicked to take it back,
//! e2e "Given a reaction When its chip is clicked Then it is taken back"), D174 ("👍" and
//! "👍\u{FE0F}" are the same reaction, one chip, quick reactions hold it once, "recent" tab),
//! D234 (fixed row ❤️ 👍 👎 😂 😮 😢). Matrix: `m.annotation`, redaction of my own reaction event.
//! Specified by `docs/dev/message-actions-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_app::reactions::{QUICK_REACTIONS, ReactionIntent, RecentEmojis, apply_reaction_toggle};
use twake_domain::reaction::{ReactionEvent, Reactions};
use twake_domain::{EventId, UserId};

fn user(name: &str) -> UserId {
    UserId::parse(&format!("@{name}:example.org")).unwrap()
}

fn ev(n: usize) -> EventId {
    EventId::parse(&format!("$tog{n}")).unwrap()
}

fn with(events: &[(usize, &str, &str)]) -> Reactions {
    let mut r = Reactions::new();
    for (n, who, key) in events {
        r.add(ReactionEvent {
            event_id: ev(*n),
            sender: user(who),
            key: (*key).into(),
        });
    }
    r
}

/// Does what the server and the timeline would: sends become events, redactions remove them.
fn apply(state: &mut Reactions, intent: ReactionIntent, next_id: usize, me: &UserId) {
    match intent {
        ReactionIntent::Send(key) => state.add(ReactionEvent {
            event_id: ev(next_id),
            sender: me.clone(),
            key,
        }),
        ReactionIntent::Redact(id) => {
            state.redact(&id);
        }
    }
}

#[test]
fn a_first_click_sends_the_emoji() {
    let me = user("me");
    assert_eq!(
        apply_reaction_toggle(&Reactions::new(), &me, "🎉"),
        ReactionIntent::Send("🎉".into())
    );
}

#[test]
fn a_click_on_my_own_reaction_redacts_that_event() {
    let me = user("me");
    let state = with(&[(7, "bob", "🎉"), (8, "me", "🎉")]);
    assert_eq!(
        apply_reaction_toggle(&state, &me, "🎉"),
        ReactionIntent::Redact(ev(8))
    );
}

#[test]
fn a_click_on_a_chip_of_others_adds_mine_with_the_same_spelling() {
    let me = user("me");
    let state = with(&[(1, "bob", "👍\u{FE0F}")]);
    assert_eq!(
        apply_reaction_toggle(&state, &me, "👍"),
        ReactionIntent::Send("👍\u{FE0F}".into())
    );
}

#[test]
fn the_other_spelling_of_my_own_reaction_still_takes_it_back() {
    let me = user("me");
    let state = with(&[(4, "me", "👍")]);
    assert_eq!(
        apply_reaction_toggle(&state, &me, "👍\u{FE0F}"),
        ReactionIntent::Redact(ev(4))
    );
}

#[test]
fn another_emoji_is_a_new_reaction_next_to_mine() {
    let me = user("me");
    let state = with(&[(1, "me", "👍")]);
    assert_eq!(
        apply_reaction_toggle(&state, &me, "👎"),
        ReactionIntent::Send("👎".into())
    );
}

#[test]
fn the_choice_depends_only_on_the_state() {
    let me = user("me");
    let state = with(&[(1, "bob", "🎉")]);
    assert_eq!(
        apply_reaction_toggle(&state, &me, "🎉"),
        apply_reaction_toggle(&state, &me, "🎉")
    );
}

#[test]
fn clicking_twice_gives_back_the_reactions_there_were() {
    let me = user("me");
    let mut state = with(&[(1, "bob", "🎉"), (2, "carol", "👀")]);
    let before = state.clone();
    let first = apply_reaction_toggle(&state, &me, "🎉");
    apply(&mut state, first, 50, &me);
    assert_eq!(state.group_of("🎉").unwrap().count(), 2);
    let second = apply_reaction_toggle(&state, &me, "🎉");
    apply(&mut state, second, 51, &me);
    assert_eq!(state, before);
}

#[test]
fn clicking_twice_on_my_own_reaction_keeps_my_part_in_the_chip() {
    let me = user("me");
    let mut state = with(&[(1, "bob", "🎉"), (2, "me", "🎉")]);
    let off = apply_reaction_toggle(&state, &me, "🎉");
    apply(&mut state, off, 50, &me);
    assert!(!state.group_of("🎉").unwrap().reacted_by(&me));
    let on = apply_reaction_toggle(&state, &me, "🎉");
    apply(&mut state, on, 51, &me);
    let g = state.group_of("🎉").unwrap();
    assert_eq!(g.senders(), &[user("bob"), me.clone()]);
    assert_eq!(g.event_of(&me), Some(&ev(51)));
}

proptest! {
    #[test]
    fn toggling_twice_from_not_reacted_is_the_initial_state(
        others in prop::collection::vec((0usize..3, 0usize..3), 0..10),
        pick in 0usize..4,
    ) {
        let who = ["ann", "ben", "cat"];
        let emoji = ["👍", "👍\u{FE0F}", "🎉", "🔥"];
        let me = user("me");
        let mut state = Reactions::new();
        for (i, (w, e)) in others.iter().enumerate() {
            state.add(ReactionEvent { event_id: ev(i), sender: user(who[*w]), key: emoji[*e].into() });
        }
        let before = state.clone();
        let one = apply_reaction_toggle(&state, &me, emoji[pick]);
        apply(&mut state, one, 100, &me);
        let two = apply_reaction_toggle(&state, &me, emoji[pick]);
        apply(&mut state, two, 101, &me);
        prop_assert_eq!(state, before);
    }
}

// ------------------------------------------------------------ the rows of emojis

#[test]
fn the_fixed_row_is_the_six_of_the_design_in_order() {
    assert_eq!(QUICK_REACTIONS, ["❤️", "👍", "👎", "😂", "😮", "😢"]);
}

#[test]
fn the_fixed_row_has_no_emoji_twice_ignoring_the_variant_selector() {
    let bare: Vec<String> = QUICK_REACTIONS
        .iter()
        .map(|e| e.replace('\u{FE0F}', ""))
        .collect();
    let mut sorted = bare.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), bare.len());
}

#[test]
fn nothing_is_recent_at_first() {
    assert!(RecentEmojis::new().list().is_empty());
}

#[test]
fn the_last_emoji_used_comes_first() {
    let mut r = RecentEmojis::new();
    r.remember("🎉");
    r.remember("🔥");
    assert_eq!(r.list(), ["🔥", "🎉"]);
}

#[test]
fn an_emoji_used_again_moves_to_the_front_once_with_the_spelling_just_used() {
    let mut r = RecentEmojis::new();
    r.remember("👍");
    r.remember("🎉");
    r.remember("👍\u{FE0F}");
    assert_eq!(r.list(), ["👍\u{FE0F}", "🎉"]);
}

#[test]
fn only_the_last_eight_are_kept() {
    let mut r = RecentEmojis::new();
    let nine = ["😀", "😃", "😄", "😁", "😆", "😅", "🤣", "😂", "🙂"];
    for e in nine {
        r.remember(e);
    }
    assert_eq!(RecentEmojis::KEPT, 8);
    assert_eq!(r.list().len(), 8);
    assert_eq!(r.list()[0], "🙂");
    assert!(!r.list().contains(&"😀".to_string()));
}
