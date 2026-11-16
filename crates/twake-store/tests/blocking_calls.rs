//! The keychain blocks (A030 open point: `KeychainSessionStore::run` calls it when the method is
//! called, on the thread of the caller, which is the UI thread). Wiring lot: every call of the
//! keychain, for the sessions and for the store key, runs where blocking is allowed
//! (`spawn_blocking`), and only once the future is awaited.
//!
//! Observed through a keychain that records the thread of each call, over the in-memory
//! `keyring-core` store: never the real keychain of the OS, never a real account.
//!
//! Expected public surface of `twake_store` (new): the store key without blocking the caller,
//! the same behaviour as `store_key_for` / `forget_store_key` of `store_key.rs`:
//! - `async fn store_key_for_async(keychain: Arc<dyn Keychain>, user: UserId)
//!       -> Result<Secret, KeychainError>` (a missing key is made with the CSPRNG of the OS);
//! - `async fn forget_store_key_async(keychain: Arc<dyn Keychain>, user: UserId)
//!       -> Result<(), KeychainError>`.
//! `KeychainSessionStore` keeps its API (`SessionStorePort`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::sync::{Arc, Mutex};
use std::thread::{self, ThreadId};

use keyring_core::{CredentialStore, mock};
use twake_app::ports::{SessionStoreError, SessionStorePort};
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};
use twake_store::{Keychain, KeychainError, KeychainSessionStore, OsKeychain};

/// A keychain over the in-memory store that writes down the thread of every call.
struct Spy {
    inner: OsKeychain,
    threads: Mutex<Vec<ThreadId>>,
    panics: bool,
}

impl Spy {
    fn new() -> Arc<Self> {
        Self::build(false)
    }

    /// A keychain whose every call panics, as a broken backend may.
    fn panicking() -> Arc<Self> {
        Self::build(true)
    }

    fn build(panics: bool) -> Arc<Self> {
        let store = mock::Store::new().unwrap();
        let as_store: Arc<CredentialStore> = store;
        Arc::new(Self {
            inner: OsKeychain::with_service(as_store, "twake-chat-test"),
            threads: Mutex::new(vec![]),
            panics,
        })
    }

    fn note(&self) {
        self.threads.lock().unwrap().push(thread::current().id());
        assert!(!self.panics, "the backend panicked");
    }

    fn threads(&self) -> Vec<ThreadId> {
        self.threads.lock().unwrap().clone()
    }
}

impl Keychain for Spy {
    fn get(&self, entry: &str) -> Result<Option<Secret>, KeychainError> {
        self.note();
        self.inner.get(entry)
    }
    fn set(&self, entry: &str, value: &Secret) -> Result<(), KeychainError> {
        self.note();
        self.inner.set(entry, value)
    }
    fn delete(&self, entry: &str) -> Result<(), KeychainError> {
        self.note();
        self.inner.delete(entry)
    }
}

fn user(name: &str) -> UserId {
    UserId::parse(&format!("@{name}:example.org")).unwrap()
}

fn session(name: &str) -> Session {
    Session {
        user_id: user(name),
        device_id: DeviceId::parse("DEVICE1").unwrap(),
        access_token: Secret::new("tok"),
        homeserver: HomeserverUrl::parse("https://matrix.example.org").unwrap(),
    }
}

fn off_this_thread(spy: &Spy) {
    let mine = thread::current().id();
    let seen = spy.threads();
    assert!(!seen.is_empty(), "the keychain was never called");
    assert!(
        seen.iter().all(|t| *t != mine),
        "a keychain call ran on the calling thread (the UI thread)"
    );
}

#[test]
fn building_the_future_does_not_touch_the_keychain() {
    let spy = Spy::new();
    let store = KeychainSessionStore::new(spy.clone());
    let _save = store.save(session("alice"));
    let _load = store.load(user("alice"));
    let _list = store.list();
    let _clear = store.clear(user("alice"));
    assert!(
        spy.threads().is_empty(),
        "the keychain was called before any await"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn saving_a_session_blocks_a_blocking_thread_not_the_caller() {
    let spy = Spy::new();
    KeychainSessionStore::new(spy.clone())
        .save(session("alice"))
        .await
        .unwrap();
    off_this_thread(&spy);
}

#[tokio::test(flavor = "current_thread")]
async fn loading_listing_and_clearing_run_off_the_calling_thread_too() {
    let spy = Spy::new();
    let store = KeychainSessionStore::new(spy.clone());
    store.save(session("alice")).await.unwrap();
    let before = spy.threads().len();
    assert_eq!(
        store.load(user("alice")).await.unwrap().map(|s| s.user_id),
        Some(user("alice"))
    );
    assert_eq!(store.list().await.unwrap().len(), 1);
    store.clear(user("alice")).await.unwrap();
    assert!(spy.threads().len() > before);
    off_this_thread(&spy);
}

#[tokio::test(flavor = "current_thread")]
async fn two_saves_at_once_lose_neither() {
    // The read-modify-write of the one entry stays whole when it moves off the thread.
    let spy = Spy::new();
    let store = Arc::new(KeychainSessionStore::new(spy.clone()));
    let (a, b) = (store.clone(), store.clone());
    let one = tokio::spawn(async move { a.save(session("alice")).await });
    let two = tokio::spawn(async move { b.save(session("bob")).await });
    one.await.unwrap().unwrap();
    two.await.unwrap().unwrap();
    assert_eq!(store.list().await.unwrap().len(), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn a_keychain_that_panics_is_an_unavailable_store_not_a_crash() {
    let store = KeychainSessionStore::new(Spy::panicking());
    assert_eq!(
        store.list().await,
        Err(SessionStoreError::Unavailable),
        "a panic of the backend must come back as an error"
    );
}
