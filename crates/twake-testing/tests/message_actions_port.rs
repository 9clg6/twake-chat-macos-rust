//! `FakeMessageActions`: a deterministic stand-in of the server side of reactions, pins and
//! reports, held as the port.
//!
//! Need: FR-075, FR-077, FR-079, D229, D255; ARCHITECTURE.md section 4 (every port has a fake and
//! a contract suite run on both the fake and the real adapter). The behaviour is
//! `twake_testing::contract::message_actions`; this file runs it and adds what only the fake has:
//! the failure modes a screen must survive (permission refused, server unreachable).
//!
//! Public items used are listed in `docs/dev/message-actions-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::sync::Arc;

use futures::executor::block_on;
use twake_app::ports::{ActionError, MessageActionsPort};
use twake_domain::{EventId, RoomId};
use twake_testing::FakeMessageActions;
use twake_testing::contract;

fn room() -> RoomId {
    RoomId::parse("!actions1:example.org").unwrap()
}

fn ev(n: usize) -> EventId {
    EventId::parse(&format!("$fake{n}")).unwrap()
}

fn known() -> FakeMessageActions {
    FakeMessageActions::new().with_room(room())
}

#[test]
fn the_fake_passes_the_contract_suite() {
    contract::message_actions(|| Arc::new(FakeMessageActions::new().with_room(room())))
        .expect("contract");
}

#[test]
fn each_fake_is_its_own_server() {
    let one = known();
    let two = known();
    block_on(one.pin(room(), ev(1))).unwrap();
    assert_eq!(block_on(two.pinned(room())), Ok(vec![]));
}

#[test]
fn a_fake_that_knows_no_room_refuses_them_all() {
    let fake = FakeMessageActions::new();
    assert_eq!(
        block_on(fake.pinned(room())),
        Err(ActionError::RoomNotFound)
    );
}

#[test]
fn a_member_without_the_right_is_refused_and_nothing_changes() {
    let fake = known().forbidding();
    assert_eq!(
        block_on(fake.pin(room(), ev(1))),
        Err(ActionError::Forbidden)
    );
    assert_eq!(
        block_on(fake.send_reaction(room(), ev(1), "🎉".into())),
        Err(ActionError::Forbidden)
    );
    assert_eq!(
        block_on(fake.redact(room(), ev(1))),
        Err(ActionError::Forbidden)
    );
    assert_eq!(
        block_on(fake.unpin(room(), ev(1))),
        Err(ActionError::Forbidden)
    );
    assert_eq!(
        block_on(fake.report(room(), ev(1), None)),
        Err(ActionError::Forbidden)
    );
}

#[test]
fn a_refused_pin_leaves_the_pins_as_they_were() {
    let fake = known().forbidding();
    let _ = block_on(fake.pin(room(), ev(1)));
    // reading is not forbidden: only the changes are
    assert_eq!(block_on(fake.pinned(room())), Ok(vec![]));
}

#[test]
fn an_unreachable_server_fails_every_call() {
    let fake = known().unavailable();
    assert_eq!(
        block_on(fake.pin(room(), ev(1))),
        Err(ActionError::Unavailable)
    );
    assert_eq!(block_on(fake.pinned(room())), Err(ActionError::Unavailable));
    assert_eq!(
        block_on(fake.send_reaction(room(), ev(1), "🎉".into())),
        Err(ActionError::Unavailable)
    );
    assert_eq!(
        block_on(fake.redact(room(), ev(1))),
        Err(ActionError::Unavailable)
    );
    assert_eq!(
        block_on(fake.unpin(room(), ev(1))),
        Err(ActionError::Unavailable)
    );
    assert_eq!(
        block_on(fake.report(room(), ev(1), Some("spam".into()))),
        Err(ActionError::Unavailable)
    );
}

#[test]
fn the_errors_say_something_and_are_distinct() {
    let all = [
        ActionError::RoomNotFound,
        ActionError::Forbidden,
        ActionError::Unavailable,
    ];
    let texts: Vec<String> = all.iter().map(ToString::to_string).collect();
    assert!(texts.iter().all(|t| !t.is_empty()));
    assert!(texts[0] != texts[1] && texts[1] != texts[2] && texts[0] != texts[2]);
}
