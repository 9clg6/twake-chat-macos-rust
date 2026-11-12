//! The selection of messages (the dark bar at the top): tick, untick, range, leave.
//!
//! Need: FR-080, D183 ("N selected", Escape leaves the mode), e2e "message-select" (`Select`
//! starts the mode with that message ticked; a press toggles; Shift+click on the last of three
//! ticks the range; Escape leaves, nothing done), D260 (the ids go in timeline order).
//! Specified by `docs/dev/message-actions-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_app::selection::Selection;
use twake_domain::EventId;

fn ev(n: usize) -> EventId {
    EventId::parse(&format!("$sel{n}")).unwrap()
}

fn order(n: usize) -> Vec<EventId> {
    (0..n).map(ev).collect()
}

#[test]
fn nothing_is_selected_at_first() {
    let s = Selection::new();
    assert!(!s.is_active());
    assert_eq!(s.len(), 0);
}

#[test]
fn select_starts_the_mode_with_that_message_ticked() {
    let mut s = Selection::new();
    s.start(ev(2));
    assert!(s.is_active());
    assert_eq!(s.len(), 1);
    assert!(s.contains(&ev(2)));
    assert!(!s.contains(&ev(1)));
}

#[test]
fn starting_again_begins_a_new_selection() {
    let mut s = Selection::new();
    s.start(ev(1));
    s.toggle(ev(2));
    s.start(ev(3));
    assert_eq!(s.ids(&order(5)), [ev(3)]);
}

#[test]
fn a_press_ticks_and_a_second_press_unticks() {
    let mut s = Selection::new();
    s.start(ev(1));
    s.toggle(ev(2));
    assert_eq!(s.len(), 2);
    s.toggle(ev(2));
    assert_eq!(s.len(), 1);
    assert!(!s.contains(&ev(2)));
}

#[test]
fn unticking_the_last_message_leaves_the_mode() {
    let mut s = Selection::new();
    s.start(ev(1));
    s.toggle(ev(1));
    assert!(!s.is_active());
    assert_eq!(s.len(), 0);
}

#[test]
fn unticking_one_of_two_keeps_the_mode_until_the_last_goes() {
    let mut s = Selection::new();
    s.start(ev(1));
    s.toggle(ev(2));
    s.toggle(ev(1));
    assert!(s.is_active());
    s.toggle(ev(2));
    assert!(!s.is_active());
}

#[test]
fn the_first_message_can_be_unticked_too() {
    let mut s = Selection::new();
    s.start(ev(1));
    s.toggle(ev(2));
    s.toggle(ev(1));
    assert_eq!(s.ids(&order(4)), [ev(2)]);
}

#[test]
fn escape_leaves_the_mode_and_forgets_the_ticks() {
    let mut s = Selection::new();
    s.start(ev(1));
    s.toggle(ev(2));
    s.clear();
    assert!(!s.is_active());
    assert_eq!(s.len(), 0);
    assert!(s.ids(&order(4)).is_empty());
}

#[test]
fn a_range_ticks_everything_between_the_anchor_and_the_target() {
    let mut s = Selection::new();
    s.start(ev(0));
    s.extend_to(ev(2), &order(5));
    assert_eq!(s.ids(&order(5)), [ev(0), ev(1), ev(2)]);
}

#[test]
fn a_range_works_upwards_too() {
    let mut s = Selection::new();
    s.start(ev(4));
    s.extend_to(ev(2), &order(5));
    assert_eq!(s.ids(&order(5)), [ev(2), ev(3), ev(4)]);
}

#[test]
fn the_anchor_is_the_last_message_pressed() {
    let mut s = Selection::new();
    s.start(ev(0));
    s.toggle(ev(3));
    s.extend_to(ev(5), &order(7));
    assert_eq!(s.ids(&order(7)), [ev(0), ev(3), ev(4), ev(5)]);
}

#[test]
fn a_range_keeps_the_messages_already_ticked_and_moves_the_anchor() {
    let mut s = Selection::new();
    s.start(ev(0));
    s.extend_to(ev(1), &order(8));
    s.extend_to(ev(4), &order(8));
    assert_eq!(s.ids(&order(8)), [ev(0), ev(1), ev(2), ev(3), ev(4)]);
}

#[test]
fn a_range_towards_a_message_not_in_the_list_is_a_plain_press() {
    let mut s = Selection::new();
    s.start(ev(0));
    s.extend_to(ev(9), &order(3));
    assert_eq!(s.len(), 2);
    assert!(s.contains(&ev(9)));
}

#[test]
fn ids_come_in_timeline_order_whatever_the_order_of_the_ticks() {
    let mut s = Selection::new();
    s.start(ev(3));
    s.toggle(ev(1));
    s.toggle(ev(2));
    assert_eq!(s.ids(&order(5)), [ev(1), ev(2), ev(3)]);
}

#[test]
fn a_ticked_message_that_left_the_list_is_dropped_from_the_ids() {
    let mut s = Selection::new();
    s.start(ev(1));
    s.toggle(ev(8));
    assert_eq!(s.ids(&order(3)), [ev(1)]);
}

proptest! {
    #[test]
    fn a_selection_never_holds_a_duplicate(
        ops in prop::collection::vec((0u8..3, 0usize..6), 0..40)
    ) {
        let list = order(6);
        let mut s = Selection::new();
        s.start(ev(0));
        for (op, n) in ops {
            match op {
                0 => s.toggle(ev(n)),
                1 => s.extend_to(ev(n), &list),
                _ => s.start(ev(n)),
            }
            let ids = s.ids(&list);
            let mut uniq = ids.clone();
            uniq.dedup();
            prop_assert_eq!(&uniq, &ids);
            prop_assert!(s.len() <= 6);
            prop_assert_eq!(s.is_active(), s.len() > 0);
            prop_assert_eq!(ids.len(), s.len());
        }
    }

    #[test]
    fn toggling_a_message_twice_changes_nothing(
        ticked in prop::collection::vec(0usize..6, 1..6), n in 0usize..6
    ) {
        let list = order(6);
        let mut s = Selection::new();
        s.start(ev(ticked[0]));
        for t in &ticked[1..] { if !s.contains(&ev(*t)) { s.toggle(ev(*t)); } }
        let before = s.ids(&list);
        s.toggle(ev(n));
        s.toggle(ev(n));
        prop_assert_eq!(s.ids(&list), before);
    }
}
