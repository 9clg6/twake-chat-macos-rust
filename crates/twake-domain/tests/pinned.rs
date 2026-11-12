//! `PinnedState`: the pins of a room (`m.room.pinned_events`) and the banner that shows them.
//!
//! Need: FR-079, D229 (several pins, the bar shows one at a time and a click shows the next,
//! "Pinned message 2 of 3"; unpinning one keeps the others; no pin lost), D260 (bulk pin in
//! timeline order, one update), e2e "message-extras" (banner shows the last pinned first,
//! "1 of 2"). Specified by `docs/dev/message-actions-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_domain::EventId;
use twake_domain::pinned::{PinnedBanner, PinnedState};

fn ev(n: usize) -> EventId {
    EventId::parse(&format!("$pin{n}")).unwrap()
}

fn state(ns: &[usize]) -> PinnedState {
    PinnedState::from_ids(ns.iter().map(|n| ev(*n)).collect())
}

fn ids(s: &PinnedState) -> Vec<EventId> {
    s.ids().to_vec()
}

#[test]
fn a_room_starts_with_no_pin_and_no_banner() {
    let s = PinnedState::new();
    assert!(s.ids().is_empty());
    assert_eq!(s.banner(0), None);
}

#[test]
fn pinning_adds_the_message_last() {
    let mut s = state(&[1]);
    s.pin(ev(2));
    assert_eq!(ids(&s), [ev(1), ev(2)]);
    assert!(s.contains(&ev(2)));
}

#[test]
fn pinning_twice_pins_once_and_keeps_the_place() {
    let mut s = state(&[1, 2]);
    s.pin(ev(1));
    assert_eq!(ids(&s), [ev(1), ev(2)]);
}

#[test]
fn unpinning_one_keeps_the_others_in_order() {
    let mut s = state(&[1, 2, 3]);
    s.unpin(&ev(2));
    assert_eq!(ids(&s), [ev(1), ev(3)]);
    assert!(!s.contains(&ev(2)));
}

#[test]
fn unpinning_an_unpinned_message_is_fine() {
    let mut s = state(&[1]);
    s.unpin(&ev(9));
    assert_eq!(ids(&s), [ev(1)]);
}

#[test]
fn state_from_the_server_drops_a_repeated_id() {
    assert_eq!(ids(&state(&[1, 2, 1, 3, 2])), [ev(1), ev(2), ev(3)]);
}

#[test]
fn bulk_pin_appends_in_the_order_given_and_skips_the_pinned() {
    let mut s = state(&[5]);
    s.pin_all(&[ev(7), ev(5), ev(8)]);
    assert_eq!(ids(&s), [ev(5), ev(7), ev(8)]);
}

#[test]
fn bulk_unpin_removes_only_the_ones_named() {
    let mut s = state(&[1, 2, 3, 4]);
    s.unpin_all(&[ev(2), ev(4), ev(9)]);
    assert_eq!(ids(&s), [ev(1), ev(3)]);
}

#[test]
fn the_banner_shows_the_newest_pin_first() {
    let s = state(&[1, 2]);
    assert_eq!(
        s.banner(0),
        Some(PinnedBanner {
            event: ev(2),
            position: 1,
            total: 2
        })
    );
}

#[test]
fn each_click_shows_the_next_pin_and_it_wraps() {
    let s = state(&[1, 2, 3]);
    let at = |n| s.banner(n).unwrap();
    assert_eq!(
        at(1),
        PinnedBanner {
            event: ev(2),
            position: 2,
            total: 3
        }
    );
    assert_eq!(
        at(2),
        PinnedBanner {
            event: ev(1),
            position: 3,
            total: 3
        }
    );
    assert_eq!(
        at(3),
        PinnedBanner {
            event: ev(3),
            position: 1,
            total: 3
        }
    );
}

#[test]
fn a_single_pin_is_one_of_one_whatever_the_clicks() {
    let s = state(&[4]);
    assert_eq!(
        s.banner(7),
        Some(PinnedBanner {
            event: ev(4),
            position: 1,
            total: 1
        })
    );
}

proptest! {
    #[test]
    fn pin_then_unpin_restores_the_pins(initial in prop::collection::vec(0usize..8, 0..8), n in 8usize..12) {
        let mut s = state(&initial);
        let before = s.clone();
        s.pin(ev(n));
        s.unpin(&ev(n));
        prop_assert_eq!(s, before);
    }

    #[test]
    fn pinning_is_idempotent_and_never_duplicates(initial in prop::collection::vec(0usize..8, 0..8), n in 0usize..12) {
        let mut once = state(&initial);
        once.pin(ev(n));
        let mut twice = once.clone();
        twice.pin(ev(n));
        prop_assert_eq!(&once, &twice);
        let mut seen = once.ids().to_vec();
        seen.sort_by_key(|e| e.to_string());
        seen.dedup();
        prop_assert_eq!(seen.len(), once.ids().len());
    }
}
