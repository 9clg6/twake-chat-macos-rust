//! The session store port: its exact shape (A009) and its errors (FR-014, NFR-015, D373).
//!
//! Need: ARCHITECTURE.md section 4 (ports return boxed `Send` futures, held as
//! `Arc<dyn Port + Send + Sync>`), section 9 and A005 (secrets only in the OS keychain, no
//! plaintext fallback: with no store the app fails with a clear error), D373 (up to 5 accounts).
//! The stub below implements the trait the way an adapter must; if the trait changes shape
//! it stops compiling. The behaviour is in the contract suite, run on the fake by
//! `crates/twake-testing/tests/session_store.rs`.
//!
//! Public items used are listed in `docs/dev/signin-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::sync::Arc;

use futures::executor::block_on;
use futures::future::BoxFuture;
use twake_app::ports::{MAX_ACCOUNTS, SessionStoreError, SessionStorePort};
use twake_domain::UserId;
use twake_domain::session::Session;

struct Stub;

impl SessionStorePort for Stub {
    fn save(&self, _session: Session) -> BoxFuture<'_, Result<(), SessionStoreError>> {
        Box::pin(async { Err(SessionStoreError::Unavailable) })
    }
    fn load(&self, _user: UserId) -> BoxFuture<'_, Result<Option<Session>, SessionStoreError>> {
        Box::pin(async { Ok(None) })
    }
    fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, SessionStoreError>> {
        Box::pin(async { Ok(vec![]) })
    }
    fn clear(&self, _user: UserId) -> BoxFuture<'_, Result<(), SessionStoreError>> {
        Box::pin(async { Ok(()) })
    }
}

#[test]
fn the_port_can_be_held_as_a_shared_trait_object() {
    let port: Arc<dyn SessionStorePort> = Arc::new(Stub);
    let user = UserId::parse("@alice:example.org").unwrap();
    assert_eq!(block_on(port.load(user)), Ok(None));
    assert_eq!(block_on(port.list()), Ok(vec![]));
}

#[test]
fn the_port_is_send_and_sync() {
    fn needs<T: Send + Sync + ?Sized>() {}
    needs::<dyn SessionStorePort>();
    needs::<Arc<dyn SessionStorePort>>();
}

#[test]
fn an_account_limit_of_five_is_the_decision_d373() {
    assert_eq!(MAX_ACCOUNTS, 5);
}

#[test]
fn every_store_error_says_something_different() {
    let all = [
        SessionStoreError::Unavailable,
        SessionStoreError::TooManyAccounts,
    ];
    let texts: Vec<String> = all.iter().map(ToString::to_string).collect();
    assert!(texts.iter().all(|t| !t.is_empty()));
    assert_ne!(texts[0], texts[1]);
}

#[test]
fn a_store_error_can_be_compared_and_copied_around() {
    let e = SessionStoreError::TooManyAccounts;
    assert_eq!(e.clone(), e);
    assert_ne!(e, SessionStoreError::Unavailable);
}
