//! `FakeDraftStore`: a deterministic stand-in of the drafts kept on disk, held as the port.
//!
//! Need: FR-055 (a draft is kept per room), ARCHITECTURE.md section 4 (every port has a fake and a
//! contract suite run on both the fake and the real adapter), NFR-015 style: a store that cannot
//! be reached is an error, never a silent loss. The behaviour is
//! `twake_testing::contract::draft_store`; this file runs it and adds what only the fake has.
//!
//! Public items used are listed in `docs/dev/composer-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::sync::Arc;

use futures::executor::block_on;
use twake_app::composer::Draft;
use twake_app::ports::{DraftStoreError, DraftStorePort};
use twake_domain::RoomId;
use twake_testing::FakeDraftStore;
use twake_testing::contract;

fn room() -> RoomId {
    RoomId::parse("!room:example.org").unwrap()
}

fn draft(text: &str) -> Draft {
    Draft {
        text: text.into(),
        mentions: vec![],
    }
}

#[test]
fn the_fake_passes_the_contract_suite() {
    contract::draft_store(|| Arc::new(FakeDraftStore::new())).expect("contract");
}

#[test]
fn each_fake_is_its_own_store() {
    let one = FakeDraftStore::new();
    let two = FakeDraftStore::new();
    block_on(one.save(room(), draft("mine"))).unwrap();
    assert_eq!(block_on(two.load(room())), Ok(None));
}

#[test]
fn an_unreachable_store_fails_every_call_and_keeps_nothing() {
    let store = FakeDraftStore::new().unavailable();
    assert_eq!(
        block_on(store.save(room(), draft("x"))),
        Err(DraftStoreError::Unavailable)
    );
    assert_eq!(
        block_on(store.load(room())),
        Err(DraftStoreError::Unavailable)
    );
    assert_eq!(
        block_on(store.clear(room())),
        Err(DraftStoreError::Unavailable)
    );
}

#[test]
fn the_default_fake_is_an_empty_store() {
    let store = FakeDraftStore::default();
    assert_eq!(block_on(store.load(room())), Ok(None));
}

#[test]
fn the_error_says_something() {
    assert!(!DraftStoreError::Unavailable.to_string().is_empty());
}
