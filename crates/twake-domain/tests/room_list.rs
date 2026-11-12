//! Room summaries, their order, the filters and the search of the room list, and the
//! pure application of a diff.
//!
//! Need (not the code): `docs/spec/features/roomlist.md`
//! - D190: one list, favourites (pinned) first, then every room by last activity, low priority
//!   at the end; D101: a room is ordered by its last message.
//! - D191 / D109 / D187: chips All, Unread, Broadcasts, Directs, Spaces, Favourites.
//! - D286: a space is never in Unread nor in Broadcasts; "Spaces" = a space and every room
//!   inside a space of the user.
//! - D247: Pin = favourite tag. D392: case and accents ignored; a search ending with a space
//!   matches its last word as a whole word ("ann" finds "Anna", "ann " does not).
//! - ARCHITECTURE.md section 4 and 5: streams carry diffs; tests with proptest for diffs.
//!
//! Public items used are listed in `docs/dev/app-core-api-sketch.md`.

#![cfg(test)]

use proptest::prelude::*;
use twake_domain::read_state::UnreadCount;
use twake_domain::room_list::{
    RoomFilter, RoomKind, RoomListDiff, RoomSummary, apply_diff, matches_search, sort_rooms,
};
use twake_domain::{MxcUri, RoomId, Timestamp};

// ------------------------------------------------------------ helpers

fn id(n: usize) -> RoomId {
    RoomId::parse(&format!("!room{n}:example.org")).expect("valid room id")
}

fn room(n: usize, activity: u64) -> RoomSummary {
    RoomSummary::new(id(n), format!("Room {n}"), RoomKind::Group)
        .with_last_activity(Timestamp::from_millis(activity))
}

fn named(n: usize, name: &str) -> RoomSummary {
    RoomSummary::new(id(n), name, RoomKind::Group)
}

fn of_kind(n: usize, kind: RoomKind) -> RoomSummary {
    RoomSummary::new(id(n), format!("Room {n}"), kind)
}

fn counts(unread: u32, mentions: u32, marked_unread: bool) -> UnreadCount {
    UnreadCount {
        unread,
        mentions,
        marked_unread,
    }
}

fn ids(rooms: &[RoomSummary]) -> Vec<RoomId> {
    rooms.iter().map(|r| r.id.clone()).collect()
}

fn sorted(mut rooms: Vec<RoomSummary>) -> Vec<RoomSummary> {
    sort_rooms(&mut rooms);
    rooms
}

// ------------------------------------------------------------ RoomSummary

#[test]
fn a_new_room_summary_keeps_its_identity_and_shows_nothing_yet() {
    let r = RoomSummary::new(id(1), "General", RoomKind::Group);
    assert_eq!(r.id, id(1));
    assert_eq!(r.name, "General");
    assert_eq!(r.kind, RoomKind::Group);
    assert_eq!(r.avatar, None);
    assert_eq!(r.last_message_preview, None);
}

#[test]
fn a_new_room_summary_is_quiet_and_unflagged() {
    let r = RoomSummary::new(id(1), "General", RoomKind::Group);
    assert_eq!(r.last_activity, Timestamp::from_millis(0));
    assert_eq!(r.unread, counts(0, 0, false));
    assert!(!r.is_favourite);
    assert!(!r.is_low_priority);
    assert_eq!(r.space, None);
}

#[test]
fn the_builders_of_a_room_summary_set_each_field() {
    let avatar = MxcUri::parse("mxc://example.org/pic42").expect("valid");
    let r = RoomSummary::new(id(1), "Design", RoomKind::Broadcast)
        .with_avatar(avatar.clone())
        .with_last_message_preview("Ship it on Friday")
        .with_last_activity(Timestamp::from_millis(1_234))
        .with_unread(counts(7, 2, false))
        .favourite()
        .low_priority()
        .in_space(id(9));
    assert_eq!(r.avatar, Some(avatar));
    assert_eq!(r.last_message_preview.as_deref(), Some("Ship it on Friday"));
    assert_eq!(r.last_activity, Timestamp::from_millis(1_234));
    assert_eq!(r.unread, counts(7, 2, false));
    assert!(r.is_favourite);
    assert!(r.is_low_priority);
    assert_eq!(r.space, Some(id(9)));
}

// ------------------------------------------------------------ order (D190, D101)

#[test]
fn rooms_are_listed_by_last_activity_newest_first() {
    let list = sorted(vec![room(1, 100), room(2, 300), room(3, 200)]);
    assert_eq!(ids(&list), vec![id(2), id(3), id(1)]);
}

#[test]
fn a_favourite_comes_before_a_newer_room() {
    let list = sorted(vec![room(1, 900), room(2, 100).favourite(), room(3, 500)]);
    assert_eq!(ids(&list), vec![id(2), id(1), id(3)]);
}

#[test]
fn favourites_are_ordered_by_activity_among_themselves() {
    let list = sorted(vec![
        room(1, 100).favourite(),
        room(2, 400),
        room(3, 300).favourite(),
        room(4, 200).favourite(),
    ]);
    assert_eq!(ids(&list), vec![id(3), id(4), id(1), id(2)]);
}

#[test]
fn a_low_priority_room_is_last_even_when_it_is_the_newest() {
    let list = sorted(vec![room(1, 999).low_priority(), room(2, 10), room(3, 20)]);
    assert_eq!(ids(&list), vec![id(3), id(2), id(1)]);
}

#[test]
fn low_priority_rooms_keep_the_activity_order_among_themselves() {
    let list = sorted(vec![
        room(1, 10).low_priority(),
        room(2, 30).low_priority(),
        room(3, 20).low_priority(),
    ]);
    assert_eq!(ids(&list), vec![id(2), id(3), id(1)]);
}

#[test]
fn rooms_with_the_same_activity_keep_their_given_order() {
    let list = sorted(vec![room(5, 50), room(3, 50), room(8, 50), room(1, 50)]);
    assert_eq!(ids(&list), vec![id(5), id(3), id(8), id(1)]);
    let reversed = sorted(vec![room(1, 50), room(8, 50), room(3, 50), room(5, 50)]);
    assert_eq!(ids(&reversed), vec![id(1), id(8), id(3), id(5)]);
}

#[test]
fn sorting_nothing_or_one_room_is_harmless() {
    assert!(sorted(vec![]).is_empty());
    assert_eq!(ids(&sorted(vec![room(4, 7)])), vec![id(4)]);
}

// ------------------------------------------------------------ filters (D109, D187, D191, D286)

#[test]
fn the_all_filter_keeps_every_kind_of_room() {
    for kind in [
        RoomKind::Direct,
        RoomKind::Group,
        RoomKind::Space,
        RoomKind::Broadcast,
        RoomKind::Voice,
    ] {
        assert!(RoomFilter::All.matches(&of_kind(1, kind)), "{kind:?}");
    }
}

#[test]
fn the_unread_filter_keeps_a_room_with_a_count() {
    let r = room(1, 1).with_unread(counts(3, 0, false));
    assert!(RoomFilter::Unread.matches(&r));
}

#[test]
fn the_unread_filter_keeps_a_room_with_only_a_mention() {
    let r = room(1, 1).with_unread(counts(0, 1, false));
    assert!(RoomFilter::Unread.matches(&r));
}

#[test]
fn the_unread_filter_keeps_a_room_marked_unread_by_hand() {
    let r = room(1, 1).with_unread(counts(0, 0, true));
    assert!(RoomFilter::Unread.matches(&r));
}

#[test]
fn the_unread_filter_drops_a_read_room() {
    assert!(!RoomFilter::Unread.matches(&room(1, 1)));
}

#[test]
fn a_space_is_never_in_the_unread_filter() {
    let space = of_kind(1, RoomKind::Space).with_unread(counts(5, 1, false));
    assert!(!RoomFilter::Unread.matches(&space));
}

#[test]
fn the_broadcasts_filter_keeps_broadcast_rooms_only() {
    assert!(RoomFilter::Broadcasts.matches(&of_kind(1, RoomKind::Broadcast)));
    for kind in [
        RoomKind::Direct,
        RoomKind::Group,
        RoomKind::Space,
        RoomKind::Voice,
    ] {
        assert!(
            !RoomFilter::Broadcasts.matches(&of_kind(1, kind)),
            "{kind:?}"
        );
    }
}

#[test]
fn the_directs_filter_keeps_direct_rooms_only() {
    assert!(RoomFilter::Directs.matches(&of_kind(1, RoomKind::Direct)));
    for kind in [
        RoomKind::Group,
        RoomKind::Space,
        RoomKind::Broadcast,
        RoomKind::Voice,
    ] {
        assert!(!RoomFilter::Directs.matches(&of_kind(1, kind)), "{kind:?}");
    }
}

#[test]
fn the_spaces_filter_keeps_a_space_and_the_rooms_inside_a_space() {
    assert!(RoomFilter::Spaces.matches(&of_kind(1, RoomKind::Space)));
    assert!(RoomFilter::Spaces.matches(&of_kind(2, RoomKind::Group).in_space(id(1))));
    assert!(!RoomFilter::Spaces.matches(&of_kind(3, RoomKind::Group)));
    assert!(!RoomFilter::Spaces.matches(&of_kind(4, RoomKind::Direct)));
}

#[test]
fn the_favourites_filter_keeps_favourites_only() {
    assert!(RoomFilter::Favourites.matches(&room(1, 1).favourite()));
    assert!(!RoomFilter::Favourites.matches(&room(2, 1)));
}

// ------------------------------------------------------------ search (D392)

#[test]
fn an_empty_search_matches_every_name() {
    assert!(matches_search("Anything at all", ""));
}

#[test]
fn a_search_finds_a_part_of_the_name_whatever_the_case() {
    assert!(matches_search("Alice Martin", "ali"));
    assert!(matches_search("alice martin", "MART"));
    assert!(matches_search("Team Alpha Chat", "team alpha"));
}

#[test]
fn a_search_that_is_not_in_the_name_does_not_match() {
    assert!(!matches_search("Alice Martin", "bob"));
    assert!(!matches_search("Alice", "alice martin"));
}

#[test]
fn a_search_ignores_accents_on_both_sides() {
    assert!(matches_search("Élodie Durand", "elo"));
    assert!(matches_search("Elodie Durand", "ÉLO"));
    assert!(matches_search("Zoë", "zoe"));
}

#[test]
fn a_search_ending_with_a_space_matches_a_whole_word() {
    assert!(matches_search("Ann Lee", "ann "));
    assert!(matches_search("Lee, Ann", "ann "));
}

#[test]
fn a_search_ending_with_a_space_does_not_match_a_longer_word() {
    assert!(!matches_search("Anna Lee", "ann "));
    assert!(!matches_search("Joann Lee", "ann "));
}

#[test]
fn a_search_without_a_trailing_space_matches_the_start_of_a_longer_word() {
    assert!(matches_search("Anna Lee", "ann"));
}

// ------------------------------------------------------------ apply_diff

#[test]
fn a_reset_replaces_the_list_and_sorts_it() {
    let old = vec![room(1, 100)];
    let list = apply_diff(
        &old,
        RoomListDiff::Reset(vec![room(2, 10), room(3, 30), room(4, 20)]),
    );
    assert_eq!(ids(&list), vec![id(3), id(4), id(2)]);
}

#[test]
fn a_reset_to_nothing_empties_the_list() {
    let list = apply_diff(&[room(1, 100)], RoomListDiff::Reset(vec![]));
    assert!(list.is_empty());
}

#[test]
fn a_reset_with_the_same_id_twice_keeps_the_last_one() {
    let list = apply_diff(
        &[],
        RoomListDiff::Reset(vec![
            named(1, "First"),
            named(2, "Other"),
            named(1, "Second"),
        ]),
    );
    assert_eq!(list.len(), 2);
    let kept = list.iter().find(|r| r.id == id(1)).expect("room 1 is kept");
    assert_eq!(kept.name, "Second");
}

#[test]
fn an_insert_puts_the_room_at_its_place() {
    let start = sorted(vec![room(1, 300), room(2, 100)]);
    let list = apply_diff(&start, RoomListDiff::Insert(room(3, 200)));
    assert_eq!(ids(&list), vec![id(1), id(3), id(2)]);
}

#[test]
fn an_insert_of_a_known_id_replaces_the_room() {
    let start = sorted(vec![named(1, "Old name"), room(2, 0)]);
    let list = apply_diff(&start, RoomListDiff::Insert(named(1, "New name")));
    assert_eq!(list.len(), 2);
    let kept = list.iter().find(|r| r.id == id(1)).expect("room 1 is kept");
    assert_eq!(kept.name, "New name");
}

#[test]
fn a_remove_drops_the_room_and_keeps_the_order_of_the_others() {
    let start = sorted(vec![room(1, 300), room(2, 200), room(3, 100)]);
    let list = apply_diff(&start, RoomListDiff::Remove(id(2)));
    assert_eq!(ids(&list), vec![id(1), id(3)]);
}

#[test]
fn a_remove_of_an_unknown_room_changes_nothing() {
    let start = sorted(vec![room(1, 300), room(2, 200)]);
    assert_eq!(apply_diff(&start, RoomListDiff::Remove(id(9))), start);
}

#[test]
fn an_update_replaces_the_room_and_moves_it_to_its_new_place() {
    let start = sorted(vec![room(1, 300), room(2, 200), room(3, 100)]);
    let updated = named(3, "Renamed").with_last_activity(Timestamp::from_millis(400));
    let list = apply_diff(&start, RoomListDiff::Update(updated));
    assert_eq!(ids(&list), vec![id(3), id(1), id(2)]);
    assert_eq!(list[0].name, "Renamed");
}

#[test]
fn an_update_can_make_a_room_a_favourite() {
    let start = sorted(vec![room(1, 300), room(2, 100)]);
    let list = apply_diff(&start, RoomListDiff::Update(room(2, 100).favourite()));
    assert_eq!(ids(&list), vec![id(2), id(1)]);
}

#[test]
fn an_update_of_an_unknown_room_is_ignored() {
    let start = sorted(vec![room(1, 300)]);
    assert_eq!(
        apply_diff(&start, RoomListDiff::Update(room(9, 999))),
        start
    );
}

#[test]
fn a_move_gives_the_room_its_new_activity_and_reorders_the_list() {
    let start = sorted(vec![
        room(1, 300).with_last_message_preview("one"),
        room(2, 200).with_last_message_preview("two"),
        room(3, 100).with_last_message_preview("three"),
    ]);
    let list = apply_diff(
        &start,
        RoomListDiff::Move {
            id: id(3),
            last_activity: Timestamp::from_millis(250),
        },
    );
    assert_eq!(ids(&list), vec![id(1), id(3), id(2)]);
    assert_eq!(list[1].last_activity, Timestamp::from_millis(250));
    assert_eq!(list[1].last_message_preview.as_deref(), Some("three"));
}

#[test]
fn a_move_of_an_unknown_room_changes_nothing() {
    let start = sorted(vec![room(1, 300), room(2, 200)]);
    let diff = RoomListDiff::Move {
        id: id(9),
        last_activity: Timestamp::from_millis(999),
    };
    assert_eq!(apply_diff(&start, diff), start);
}

#[test]
fn applying_a_diff_leaves_the_given_list_alone() {
    let start = sorted(vec![room(1, 300), room(2, 200)]);
    let copy = start.clone();
    let _ = apply_diff(&start, RoomListDiff::Remove(id(1)));
    assert_eq!(start, copy);
}

// ------------------------------------------------------------ properties

fn any_room() -> impl Strategy<Value = RoomSummary> {
    (0usize..6, 0u64..5, any::<bool>(), any::<bool>()).prop_map(|(n, ts, fav, low)| {
        let r = room(n, ts);
        let r = if fav { r.favourite() } else { r };
        if low { r.low_priority() } else { r }
    })
}

fn any_diff() -> impl Strategy<Value = RoomListDiff> {
    prop_oneof![
        prop::collection::vec(any_room(), 0..8).prop_map(RoomListDiff::Reset),
        any_room().prop_map(RoomListDiff::Insert),
        any_room().prop_map(RoomListDiff::Update),
        (0usize..6).prop_map(|n| RoomListDiff::Remove(id(n))),
        (0usize..6, 0u64..5).prop_map(|(n, ts)| RoomListDiff::Move {
            id: id(n),
            last_activity: Timestamp::from_millis(ts),
        }),
    ]
}

fn rank(r: &RoomSummary) -> (bool, bool, std::cmp::Reverse<Timestamp>) {
    (
        r.is_low_priority,
        !r.is_favourite,
        std::cmp::Reverse(r.last_activity),
    )
}

proptest! {
    #[test]
    fn whatever_the_diffs_the_list_stays_sorted_without_duplicate_ids(
        diffs in prop::collection::vec(any_diff(), 0..12)
    ) {
        let list = diffs.into_iter().fold(Vec::new(), |list, d| apply_diff(&list, d));
        let mut seen = std::collections::HashSet::new();
        for r in &list {
            prop_assert!(seen.insert(r.id.clone()), "duplicate id {:?}", r.id);
        }
        for pair in list.windows(2) {
            prop_assert!(rank(&pair[0]) <= rank(&pair[1]), "{:?} before {:?}", pair[0], pair[1]);
        }
    }

    #[test]
    fn sorting_twice_changes_nothing_and_loses_no_room(
        rooms in prop::collection::vec(any_room(), 0..10)
    ) {
        let once = sorted(rooms.clone());
        let twice = sorted(once.clone());
        prop_assert_eq!(&once, &twice);
        prop_assert_eq!(once.len(), rooms.len());
    }

    #[test]
    fn a_remove_leaves_no_room_with_that_id(
        rooms in prop::collection::vec(any_room(), 0..10),
        n in 0usize..6
    ) {
        let start = apply_diff(&[], RoomListDiff::Reset(rooms));
        let list = apply_diff(&start, RoomListDiff::Remove(id(n)));
        prop_assert!(list.iter().all(|r| r.id != id(n)));
    }
}
