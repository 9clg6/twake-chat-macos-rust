//! The composer with its draft store: type in a room, switch room and back, restart, send.
//!
//! Need (not the code): FR-055 ("Given text typed in room A, When the user switches room and back
//! or restarts, Then the draft is restored; Enter empties it"), D382, D487 (picked mentions are
//! saved with the draft), D512 (sending forgets the draft). The composer is pure: the screen saves
//! `draft()` through the port when the text changes and restores it when a room opens.
//!
//! Public items used are listed in `docs/dev/composer-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use futures::executor::block_on;
use twake_app::composer::{ComposerState, PrimaryAction, Sources};
use twake_app::ports::DraftStorePort;
use twake_domain::mention::Member;
use twake_domain::{RoomId, UserId};
use twake_testing::FakeDraftStore;

fn room(n: u8) -> RoomId {
    RoomId::parse(&format!("!flow{n}:example.org")).unwrap()
}

fn me() -> UserId {
    UserId::parse("@me:example.org").unwrap()
}

fn bob() -> UserId {
    UserId::parse("@bob:example.org").unwrap()
}

/// What the screen does when a room opens: a composer, the draft put back if there is one.
fn open(store: &FakeDraftStore, n: u8) -> ComposerState {
    let mut s = ComposerState::new(room(n), me());
    s.set_sources(Sources {
        members: vec![Member {
            user_id: bob(),
            name: "Bob".into(),
            matrix_name: None,
        }],
        ..Sources::default()
    });
    if let Some(draft) = block_on(store.load(room(n))).unwrap() {
        s.restore(draft);
    }
    s
}

/// What the screen does after a change of the text.
fn keep(store: &FakeDraftStore, n: u8, s: &ComposerState) {
    block_on(store.save(room(n), s.draft())).unwrap();
}

fn type_in(store: &FakeDraftStore, n: u8, s: &mut ComposerState, text: &str) {
    s.set_text(text, text.len());
    keep(store, n, s);
}

#[test]
fn a_draft_comes_back_when_the_room_is_opened_again() {
    let store = FakeDraftStore::new();
    let mut a = open(&store, 1);
    type_in(&store, 1, &mut a, "half a thought");
    drop(a);
    let again = open(&store, 1);
    assert_eq!(again.text(), "half a thought");
    assert_eq!(again.action(), PrimaryAction::Send);
}

#[test]
fn each_room_has_its_own_draft() {
    let store = FakeDraftStore::new();
    let mut a = open(&store, 1);
    type_in(&store, 1, &mut a, "for A");
    let mut b = open(&store, 2);
    type_in(&store, 2, &mut b, "for B");
    assert_eq!(open(&store, 1).text(), "for A");
    assert_eq!(open(&store, 2).text(), "for B");
    assert_eq!(open(&store, 3).text(), "");
}

#[test]
fn enter_empties_the_draft() {
    let store = FakeDraftStore::new();
    let mut a = open(&store, 1);
    type_in(&store, 1, &mut a, "sent soon");
    a.submit().unwrap();
    keep(&store, 1, &a);
    assert_eq!(block_on(store.load(room(1))), Ok(None));
    assert_eq!(open(&store, 1).text(), "");
}

#[test]
fn emptying_the_field_by_hand_forgets_the_draft() {
    let store = FakeDraftStore::new();
    let mut a = open(&store, 1);
    type_in(&store, 1, &mut a, "oops");
    type_in(&store, 1, &mut a, "");
    assert_eq!(block_on(store.load(room(1))), Ok(None));
}

#[test]
fn a_picked_mention_survives_a_restart() {
    let store = FakeDraftStore::new();
    let mut a = open(&store, 1);
    type_in(&store, 1, &mut a, "hi @bo");
    let pick = a.suggestions().remove(0);
    a.accept(pick);
    keep(&store, 1, &a);
    let mut again = open(&store, 1);
    assert_eq!(again.text(), "hi Bob ");
    assert_eq!(again.submit().unwrap().mentions.user_ids, vec![bob()]);
}

#[test]
fn an_edit_never_overwrites_the_draft() {
    use twake_app::composer::EditTarget;
    use twake_domain::EventId;
    let store = FakeDraftStore::new();
    let mut a = open(&store, 1);
    type_in(&store, 1, &mut a, "my draft");
    a.start_edit(EditTarget {
        event_id: EventId::parse("$mine").unwrap(),
        body: "old message".into(),
        mentions: vec![],
    });
    a.set_text("old message edited", 18);
    keep(&store, 1, &a);
    assert_eq!(open(&store, 1).text(), "my draft");
}
