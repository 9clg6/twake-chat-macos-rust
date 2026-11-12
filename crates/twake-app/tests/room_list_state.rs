//! `RoomListState`: what the room list screen shows, from diffs, a filter and a search.
//!
//! Need (not the code): `docs/spec/features/roomlist.md`
//! - D190: favourites first, then last activity; D191/D109: chips All, Unread, Broadcasts,
//!   Directs, Spaces, Favourites; D392: the search of a name.
//! - D222: an empty search says "No conversation found." (not "No conversation yet."); the
//!   Chat badge counts the unread messages of every room, whatever chip or search is on.
//! - Screens: loading, empty, nothing found, failed (with a way out: a new batch recovers).
//!
//! Public items used are listed in `docs/dev/app-core-api-sketch.md`.

#![cfg(test)]

use twake_app::ports::RoomListError;
use twake_app::room_list::{RoomListState, RoomListStatus};
use twake_domain::read_state::UnreadCount;
use twake_domain::room_list::{RoomFilter, RoomKind, RoomListDiff, RoomSummary};
use twake_domain::{RoomId, Timestamp};

// ------------------------------------------------------------ helpers

fn id(n: usize) -> RoomId {
    RoomId::parse(&format!("!room{n}:example.org")).expect("valid room id")
}

fn room(n: usize, name: &str, activity: u64) -> RoomSummary {
    RoomSummary::new(id(n), name, RoomKind::Group)
        .with_last_activity(Timestamp::from_millis(activity))
}

fn unread(unread: u32, mentions: u32) -> UnreadCount {
    UnreadCount {
        unread,
        mentions,
        marked_unread: false,
    }
}

fn names(state: &RoomListState) -> Vec<String> {
    state.view().into_iter().map(|r| r.name).collect()
}

fn loaded(rooms: Vec<RoomSummary>) -> RoomListState {
    let mut state = RoomListState::new();
    state.apply(vec![RoomListDiff::Reset(rooms)]);
    state
}

fn team() -> Vec<RoomSummary> {
    vec![
        room(1, "Alice Martin", 100).with_unread(unread(3, 0)),
        room(2, "Bob Stone", 300),
        room(3, "Design team", 200)
            .favourite()
            .with_unread(unread(5, 1)),
        RoomSummary::new(id(4), "Announcements", RoomKind::Broadcast)
            .with_last_activity(Timestamp::from_millis(50)),
    ]
}

// ------------------------------------------------------------ start

#[test]
fn a_new_state_is_loading_and_shows_nothing() {
    let state = RoomListState::new();
    assert_eq!(state.status(), RoomListStatus::Loading);
    assert!(state.view().is_empty());
}

#[test]
fn a_new_state_shows_all_rooms_with_no_search() {
    let state = RoomListState::new();
    assert_eq!(state.filter(), RoomFilter::All);
    assert_eq!(state.search(), "");
}

// ------------------------------------------------------------ diffs

#[test]
fn the_first_reset_makes_the_list_ready_and_sorted() {
    let state = loaded(team());
    assert_eq!(state.status(), RoomListStatus::Ready);
    assert_eq!(
        names(&state),
        vec!["Design team", "Bob Stone", "Alice Martin", "Announcements"]
    );
}

#[test]
fn a_reset_to_no_room_is_the_empty_list() {
    let state = loaded(vec![]);
    assert_eq!(state.status(), RoomListStatus::Empty);
    assert!(state.view().is_empty());
}

#[test]
fn the_diffs_of_a_batch_apply_in_order() {
    let mut state = loaded(vec![room(1, "Old", 10)]);
    state.apply(vec![
        RoomListDiff::Insert(room(2, "Fresh", 20)),
        RoomListDiff::Update(room(2, "Fresh renamed", 20)),
        RoomListDiff::Remove(id(1)),
    ]);
    assert_eq!(names(&state), vec!["Fresh renamed"]);
}

#[test]
fn several_batches_build_on_each_other() {
    let mut state = loaded(vec![room(1, "One", 10), room(2, "Two", 20)]);
    state.apply(vec![RoomListDiff::Insert(room(3, "Three", 30))]);
    state.apply(vec![RoomListDiff::Remove(id(2))]);
    assert_eq!(names(&state), vec!["Three", "One"]);
}

#[test]
fn a_move_brings_a_room_to_the_top() {
    let mut state = loaded(vec![
        room(1, "One", 10),
        room(2, "Two", 20),
        room(3, "Three", 30),
    ]);
    state.apply(vec![RoomListDiff::Move {
        id: id(1),
        last_activity: Timestamp::from_millis(99),
    }]);
    assert_eq!(names(&state), vec!["One", "Three", "Two"]);
}

#[test]
fn an_empty_batch_changes_nothing() {
    let mut state = loaded(team());
    let before = state.view();
    state.apply(vec![]);
    assert_eq!(state.view(), before);
    assert_eq!(state.status(), RoomListStatus::Ready);
}

// ------------------------------------------------------------ filter

#[test]
fn the_unread_filter_shows_the_unread_rooms_in_order() {
    let mut state = loaded(team());
    state.set_filter(RoomFilter::Unread);
    assert_eq!(state.filter(), RoomFilter::Unread);
    assert_eq!(names(&state), vec!["Design team", "Alice Martin"]);
}

#[test]
fn the_broadcasts_filter_shows_the_broadcast_rooms() {
    let mut state = loaded(team());
    state.set_filter(RoomFilter::Broadcasts);
    assert_eq!(names(&state), vec!["Announcements"]);
}

#[test]
fn the_favourites_filter_shows_the_favourites() {
    let mut state = loaded(team());
    state.set_filter(RoomFilter::Favourites);
    assert_eq!(names(&state), vec!["Design team"]);
}

#[test]
fn going_back_to_all_shows_every_room_again() {
    let mut state = loaded(team());
    state.set_filter(RoomFilter::Favourites);
    state.set_filter(RoomFilter::All);
    assert_eq!(state.view().len(), 4);
}

#[test]
fn a_filter_follows_the_next_diffs() {
    let mut state = loaded(team());
    state.set_filter(RoomFilter::Unread);
    state.apply(vec![RoomListDiff::Update(
        room(2, "Bob Stone", 300).with_unread(unread(1, 0)),
    )]);
    assert_eq!(
        names(&state),
        vec!["Design team", "Bob Stone", "Alice Martin"]
    );
}

// ------------------------------------------------------------ search

#[test]
fn a_search_keeps_the_rooms_whose_name_matches() {
    let mut state = loaded(team());
    state.set_search("ali");
    assert_eq!(state.search(), "ali");
    assert_eq!(names(&state), vec!["Alice Martin"]);
}

#[test]
fn a_search_ignores_case() {
    let mut state = loaded(team());
    state.set_search("DESIGN");
    assert_eq!(names(&state), vec!["Design team"]);
}

#[test]
fn clearing_the_search_shows_every_room_again() {
    let mut state = loaded(team());
    state.set_search("bob");
    state.set_search("");
    assert_eq!(state.view().len(), 4);
}

#[test]
fn a_search_and_a_filter_both_apply() {
    let mut state = loaded(team());
    state.set_filter(RoomFilter::Unread);
    state.set_search("team");
    assert_eq!(names(&state), vec!["Design team"]);
    state.set_search("bob");
    assert!(state.view().is_empty());
}

#[test]
fn a_search_follows_the_next_diffs() {
    let mut state = loaded(team());
    state.set_search("ali");
    state.apply(vec![RoomListDiff::Insert(room(9, "Alicia Keys", 400))]);
    assert_eq!(names(&state), vec!["Alicia Keys", "Alice Martin"]);
}

// ------------------------------------------------------------ status

#[test]
fn a_search_without_result_says_nothing_found_not_empty() {
    let mut state = loaded(team());
    state.set_search("zzz");
    assert_eq!(state.status(), RoomListStatus::NoMatch);
}

#[test]
fn a_filter_without_result_says_nothing_found() {
    let mut state = loaded(vec![room(1, "Quiet", 1)]);
    state.set_filter(RoomFilter::Unread);
    assert_eq!(state.status(), RoomListStatus::NoMatch);
}

#[test]
fn a_search_on_an_account_without_room_still_says_nothing_found() {
    let mut state = loaded(vec![]);
    state.set_search("anyone");
    assert_eq!(state.status(), RoomListStatus::NoMatch);
}

#[test]
fn a_filter_on_an_account_without_room_says_the_list_is_empty() {
    let mut state = loaded(vec![]);
    state.set_filter(RoomFilter::Favourites);
    assert_eq!(state.status(), RoomListStatus::Empty);
}

#[test]
fn a_search_with_results_is_ready() {
    let mut state = loaded(team());
    state.set_search("bob");
    assert_eq!(state.status(), RoomListStatus::Ready);
}

#[test]
fn a_failure_is_shown_and_a_new_batch_recovers() {
    let mut state = RoomListState::new();
    state.fail(RoomListError::Unavailable);
    assert_eq!(
        state.status(),
        RoomListStatus::Failed(RoomListError::Unavailable)
    );
    state.apply(vec![RoomListDiff::Reset(team())]);
    assert_eq!(state.status(), RoomListStatus::Ready);
}

#[test]
fn a_failure_keeps_the_rooms_already_shown() {
    let mut state = loaded(team());
    state.fail(RoomListError::NotSignedIn);
    assert_eq!(
        state.status(),
        RoomListStatus::Failed(RoomListError::NotSignedIn)
    );
    assert_eq!(state.view().len(), 4);
}

// ------------------------------------------------------------ badge

#[test]
fn the_total_unread_sums_every_room() {
    let state = loaded(team());
    let total = state.total_unread();
    assert_eq!(total.unread, 8);
    assert_eq!(total.mentions, 1);
}

#[test]
fn the_total_unread_ignores_the_filter_and_the_search() {
    let mut state = loaded(team());
    state.set_filter(RoomFilter::Broadcasts);
    state.set_search("announce");
    assert_eq!(state.total_unread().unread, 8);
}

#[test]
fn the_total_unread_of_nothing_is_zero() {
    let total = RoomListState::new().total_unread();
    assert_eq!((total.unread, total.mentions), (0, 0));
    assert!(!total.marked_unread);
}
