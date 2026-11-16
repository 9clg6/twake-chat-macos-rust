//! `FakeSessionStore`: a deterministic stand-in of the keychain store, held as the port.
//!
//! Need: ARCHITECTURE.md section 4 (every port has a fake and a contract suite run on both the
//! fake and the real adapter), A005 and NFR-015 (a store that cannot be reached is an error,
//! never a silent loss), D373 (up to 5 accounts). The behaviour is
//! `twake_testing::contract::session_store`; this file runs it and adds what only the fake has.
//!
//! Public items used are listed in `docs/dev/signin-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::sync::Arc;

use futures::executor::block_on;
use twake_app::ports::{SessionStoreError, SessionStorePort};
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};
use twake_testing::FakeSessionStore;
use twake_testing::contract;

fn session() -> Session {
    Session {
        user_id: UserId::parse("@alice:example.org").unwrap(),
        device_id: DeviceId::parse("DEVICE1").unwrap(),
        access_token: Secret::new("tok"),
        homeserver: HomeserverUrl::parse("https://matrix.example.org").unwrap(),
    }
}

#[test]
fn the_fake_passes_the_contract_suite() {
    contract::session_store(|| Arc::new(FakeSessionStore::new())).expect("contract");
}

#[test]
fn each_fake_is_its_own_store() {
    let one = FakeSessionStore::new();
    let two = FakeSessionStore::new();
    block_on(one.save(session())).unwrap();
    assert_eq!(block_on(two.list()), Ok(vec![]));
}

#[test]
fn an_unreachable_store_fails_every_call_and_keeps_nothing() {
    let store = FakeSessionStore::new().unavailable();
    let user = session().user_id;
    assert_eq!(
        block_on(store.save(session())),
        Err(SessionStoreError::Unavailable)
    );
    assert_eq!(
        block_on(store.load(user.clone())),
        Err(SessionStoreError::Unavailable)
    );
    assert_eq!(block_on(store.list()), Err(SessionStoreError::Unavailable));
    assert_eq!(
        block_on(store.clear(user)),
        Err(SessionStoreError::Unavailable)
    );
}

#[test]
fn the_fake_is_usable_as_the_shared_port() {
    let port: Arc<dyn SessionStorePort> = Arc::new(FakeSessionStore::new());
    block_on(port.save(session())).unwrap();
    assert_eq!(
        block_on(port.load(session().user_id)).unwrap(),
        Some(session())
    );
}
