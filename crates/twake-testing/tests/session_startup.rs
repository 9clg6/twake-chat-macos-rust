//! The session across a restart (FR-014, D72, NFR-015, A005): what the app does with the store of
//! sessions when it starts and when a login ends. Run on the fake of the port; the keychain
//! adapter holds the same contract (`session_store.rs`).
//!
//! Expected public surface of `twake_app::startup` (new):
//! ```text
//! enum Startup { SignIn, Restore(Vec<Session>), StoreUnavailable }
//! async fn startup(store: &dyn SessionStorePort) -> Startup
//!     // nothing stored -> SignIn; sessions in the order they were saved (the first is the
//!     // default account, D373) -> Restore; the store unreachable -> StoreUnavailable:
//!     // never SignIn as if there were nothing (no plaintext fallback, A005): the screen says so
//! async fn remember(store: &dyn SessionStorePort, session: Session) -> Result<(), SessionStoreError>
//!     // after `LoginOk`: the error reaches the caller, a login nobody can restore is not silent
//! async fn forget(store: &dyn SessionStorePort, user: UserId) -> Result<(), SessionStoreError>
//! ```
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use futures::executor::block_on;
use twake_app::ports::SessionStoreError;
use twake_app::startup::{Startup, forget, remember, startup};
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};
use twake_testing::FakeSessionStore;

fn session(name: &str, device: &str) -> Session {
    Session {
        user_id: UserId::parse(&format!("@{name}:example.org")).unwrap(),
        device_id: DeviceId::parse(device).unwrap(),
        access_token: Secret::new(format!("token-{name}")),
        homeserver: HomeserverUrl::parse("https://matrix.example.org").unwrap(),
    }
}

#[test]
fn a_first_start_goes_to_the_sign_in() {
    assert_eq!(block_on(startup(&FakeSessionStore::new())), Startup::SignIn);
}

#[test]
fn a_login_is_found_again_at_the_next_start_with_its_device() {
    let store = FakeSessionStore::new();
    block_on(remember(&store, session("alice", "DEVICE1"))).unwrap();
    // The restart: nothing but the store is kept.
    assert_eq!(
        block_on(startup(&store)),
        Startup::Restore(vec![session("alice", "DEVICE1")])
    );
}

#[test]
fn the_accounts_come_back_in_the_order_they_were_added() {
    let store = FakeSessionStore::new();
    for name in ["alice", "bob", "carol"] {
        block_on(remember(&store, session(name, "D"))).unwrap();
    }
    let Startup::Restore(all) = block_on(startup(&store)) else {
        panic!("sessions were stored");
    };
    let names: Vec<&str> = all.iter().map(|s| s.user_id.as_str()).collect();
    assert_eq!(
        names,
        [
            "@alice:example.org",
            "@bob:example.org",
            "@carol:example.org"
        ]
    );
}

#[test]
fn a_new_login_of_the_same_account_replaces_it_and_keeps_its_place() {
    let store = FakeSessionStore::new();
    block_on(remember(&store, session("alice", "OLD"))).unwrap();
    block_on(remember(&store, session("bob", "B"))).unwrap();
    block_on(remember(&store, session("alice", "NEW"))).unwrap();
    let Startup::Restore(all) = block_on(startup(&store)) else {
        panic!("sessions were stored");
    };
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].device_id.as_str(), "NEW");
}

#[test]
fn a_forgotten_account_is_not_restored() {
    let store = FakeSessionStore::new();
    block_on(remember(&store, session("alice", "D"))).unwrap();
    block_on(forget(&store, session("alice", "D").user_id)).unwrap();
    assert_eq!(block_on(startup(&store)), Startup::SignIn);
}

#[test]
fn an_unreachable_keychain_is_told_not_taken_for_an_empty_one() {
    let store = FakeSessionStore::new().unavailable();
    assert_eq!(block_on(startup(&store)), Startup::StoreUnavailable);
}

#[test]
fn a_login_that_cannot_be_kept_is_an_error_for_the_caller() {
    let store = FakeSessionStore::new().unavailable();
    assert_eq!(
        block_on(remember(&store, session("alice", "D"))),
        Err(SessionStoreError::Unavailable)
    );
}

#[test]
fn a_sixth_account_is_refused_and_the_five_stay() {
    let store = FakeSessionStore::new();
    for n in 0..5 {
        block_on(remember(&store, session(&format!("user{n}"), "D"))).unwrap();
    }
    assert_eq!(
        block_on(remember(&store, session("extra", "D"))),
        Err(SessionStoreError::TooManyAccounts)
    );
    let Startup::Restore(all) = block_on(startup(&store)) else {
        panic!("sessions were stored");
    };
    assert_eq!(all.len(), 5);
}

#[test]
fn forgetting_when_the_keychain_fails_is_an_error_not_a_silent_success() {
    let store = FakeSessionStore::new().unavailable();
    assert_eq!(
        block_on(forget(&store, session("alice", "D").user_id)),
        Err(SessionStoreError::Unavailable)
    );
}
