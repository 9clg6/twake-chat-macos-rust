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
use twake_domain::UserId;
use twake_domain::session::Secret;
use twake_store::{Keychain, KeychainError, OsKeychain, forget_store_key_async, store_key_for_async};

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

fn off_this_thread(spy: &Spy) {
    let mine = thread::current().id();
    let seen = spy.threads();
    assert!(!seen.is_empty(), "the keychain was never called");
    assert!(
        seen.iter().all(|t| *t != mine),
        "a keychain call ran on the calling thread (the UI thread)"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn the_store_key_is_made_and_read_off_the_calling_thread() {
    let spy = Spy::new();
    let made = store_key_for_async(spy.clone(), user("alice")).await.unwrap();
    assert!(
        made.expose().len() == 64
            && made
                .expose()
                .chars()
                .all(|c| matches!(c, '0'..='9' | 'a'..='f')),
        "the key is 64 lowercase hexadecimals"
    );
    off_this_thread(&spy);
}

#[tokio::test(flavor = "current_thread")]
async fn the_store_key_is_made_once_and_kept_under_the_entry_of_the_account() {
    let spy = Spy::new();
    let first = store_key_for_async(spy.clone(), user("alice")).await.unwrap();
    let again = store_key_for_async(spy.clone(), user("alice")).await.unwrap();
    assert_eq!(first, again, "a new key would orphan the encrypted store");
    assert_eq!(
        spy.get("store-key:@alice:example.org").unwrap(),
        Some(first),
        "A030 decision 2: the entry is store-key:<user id>"
    );
    let other = store_key_for_async(spy.clone(), user("bob")).await.unwrap();
    assert_ne!(again, other, "one key per account");
}

#[tokio::test(flavor = "current_thread")]
async fn forgetting_the_store_key_runs_off_the_calling_thread_and_removes_it() {
    let spy = Spy::new();
    store_key_for_async(spy.clone(), user("alice")).await.unwrap();
    let before = spy.threads().len();
    forget_store_key_async(spy.clone(), user("alice")).await.unwrap();
    let after = spy.threads();
    assert!(after.len() > before, "the delete reached the keychain");
    assert!(after[before..].iter().all(|t| *t != thread::current().id()));
    assert_eq!(spy.get("store-key:@alice:example.org").unwrap(), None);
}

#[tokio::test(flavor = "current_thread")]
async fn a_stored_key_that_is_not_valid_is_still_an_error_never_replaced() {
    let spy = Spy::new();
    spy.set("store-key:@alice:example.org", &Secret::new("not-a-key"))
        .unwrap();
    assert!(store_key_for_async(spy.clone(), user("alice")).await.is_err());
    assert_eq!(
        spy.get("store-key:@alice:example.org").unwrap(),
        Some(Secret::new("not-a-key"))
    );
}
