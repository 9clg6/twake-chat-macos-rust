//! `KeychainSessionStore`: the `SessionStorePort` over the OS keychain (A005, NFR-015, D373).
//! The OS is behind a trait, so these tests use a keychain in memory; the real one (`keyring-core`
//! stores per OS) is checked by the spike S3 on each OS.
//!
//! Expected public surface of `twake_store` (P2: `keyring*` lives only in this crate):
//! - `trait Keychain: Send + Sync { fn get(&self, entry: &str) -> Result<Option<Secret>, KeychainError>;
//!   fn set(&self, entry: &str, value: &Secret) -> Result<(), KeychainError>;
//!   fn delete(&self, entry: &str) -> Result<(), KeychainError>; }` (sync, as the OS calls are;
//!   `delete` of a missing entry is Ok), `struct KeychainError` (Debug + Display + Default, no secret in them);
//! - `KeychainSessionStore::new(Arc<dyn Keychain>)` implementing `SessionStorePort`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use futures::executor::block_on;
use twake_app::ports::{SessionStoreError, SessionStorePort};
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};
use twake_store::{Keychain, KeychainError, KeychainSessionStore};
use twake_testing::contract;

/// A keychain in memory that can be made to fail.
#[derive(Default)]
struct MemoryKeychain {
    entries: Mutex<HashMap<String, String>>,
    broken: Mutex<bool>,
}

impl MemoryKeychain {
    fn break_it(&self) {
        *self.broken.lock().unwrap() = true;
    }
    fn plant(&self, entry: &str, text: &str) {
        self.entries
            .lock()
            .unwrap()
            .insert(entry.to_owned(), text.to_owned());
    }
    fn all_text(&self) -> String {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .map(|(k, v)| format!("{k}={v}\n"))
            .collect()
    }
    fn check(&self) -> Result<(), KeychainError> {
        if *self.broken.lock().unwrap() {
            Err(KeychainError::default())
        } else {
            Ok(())
        }
    }
}

impl Keychain for MemoryKeychain {
    fn get(&self, entry: &str) -> Result<Option<Secret>, KeychainError> {
        self.check()?;
        Ok(self
            .entries
            .lock()
            .unwrap()
            .get(entry)
            .cloned()
            .map(Secret::new))
    }
    fn set(&self, entry: &str, value: &Secret) -> Result<(), KeychainError> {
        self.check()?;
        self.entries
            .lock()
            .unwrap()
            .insert(entry.to_owned(), value.expose().to_owned());
        Ok(())
    }
    fn delete(&self, entry: &str) -> Result<(), KeychainError> {
        self.check()?;
        self.entries.lock().unwrap().remove(entry);
        Ok(())
    }
}

fn alice_session(token: &str) -> Session {
    Session {
        user_id: UserId::parse("@alice:example.org").unwrap(),
        device_id: DeviceId::parse("DEVICE1").unwrap(),
        access_token: Secret::new(token),
        homeserver: HomeserverUrl::parse("https://matrix.example.org").unwrap(),
    }
}

fn store(keychain: &Arc<MemoryKeychain>) -> KeychainSessionStore {
    KeychainSessionStore::new(keychain.clone())
}

#[test]
fn the_keychain_store_passes_the_contract_suite_of_the_port() {
    contract::session_store(|| {
        Arc::new(KeychainSessionStore::new(Arc::new(
            MemoryKeychain::default(),
        )))
    })
    .expect("contract");
}

#[test]
fn a_session_survives_a_restart() {
    let keychain = Arc::new(MemoryKeychain::default());
    block_on(store(&keychain).save(alice_session("tok"))).unwrap();

    let after_restart = store(&keychain);

    assert_eq!(
        block_on(after_restart.list()),
        Ok(vec![alice_session("tok")])
    );
}

#[test]
fn the_token_is_kept_in_the_keychain_and_nowhere_else() {
    let keychain = Arc::new(MemoryKeychain::default());
    block_on(store(&keychain).save(alice_session("tok-very-secret"))).unwrap();
    assert!(keychain.all_text().contains("tok-very-secret"));
}

#[test]
fn an_unreachable_keychain_fails_every_call_with_unavailable() {
    let keychain = Arc::new(MemoryKeychain::default());
    let store = store(&keychain);
    block_on(store.save(alice_session("tok"))).unwrap();
    keychain.break_it();
    let user = alice_session("tok").user_id;

    assert_eq!(
        block_on(store.save(alice_session("new"))),
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
fn a_failed_save_never_falls_back_to_a_plaintext_copy() {
    let keychain = Arc::new(MemoryKeychain::default());
    let store = store(&keychain);
    keychain.break_it();
    let _ = block_on(store.save(alice_session("tok-very-secret")));
    assert!(!keychain.all_text().contains("tok-very-secret"));
}

#[test]
fn an_unreadable_entry_is_an_error_not_an_empty_store() {
    // A005: never a silent loss. Entry names are the adapter's business, so every entry is damaged.
    let keychain = Arc::new(MemoryKeychain::default());
    block_on(store(&keychain).save(alice_session("tok"))).unwrap();
    let names: Vec<String> = keychain.entries.lock().unwrap().keys().cloned().collect();
    for name in names {
        keychain.plant(&name, "\u{0}not a session\u{0}");
    }
    assert_eq!(
        block_on(store(&keychain).list()),
        Err(SessionStoreError::Unavailable)
    );
}

#[test]
fn errors_and_debug_output_show_no_token() {
    let keychain = Arc::new(MemoryKeychain::default());
    let store = store(&keychain);
    block_on(store.save(alice_session("tok-very-secret"))).unwrap();
    keychain.break_it();
    let err = block_on(store.list()).unwrap_err();
    let text = format!("{err} {err:?} {:?}", KeychainError::default());
    assert!(!text.contains("tok-very-secret"), "{text}");
}

#[test]
fn the_store_is_usable_as_the_shared_port() {
    let port: Arc<dyn SessionStorePort> = Arc::new(KeychainSessionStore::new(Arc::new(
        MemoryKeychain::default(),
    )));
    block_on(port.save(alice_session("tok"))).unwrap();
    assert_eq!(
        block_on(port.load(alice_session("tok").user_id)).unwrap(),
        Some(alice_session("tok"))
    );
}

#[test]
fn a_session_that_the_line_format_cannot_hold_is_refused_and_the_store_stays_readable() {
    // Space, line breaks, tabs, controls, other Unicode whitespace and an empty token would split
    // or drop a field on reading back: the whole entry would be unreadable (A005).
    for token in [
        "has space",
        "line\nbreak",
        "carriage\rreturn",
        "tab\there",
        "nul\u{0}byte",
        "next\u{85}line",
        "em\u{2003}space",
        "",
    ] {
        let keychain = Arc::new(MemoryKeychain::default());
        let store = store(&keychain);
        let kept = alice_session("kept-token");
        block_on(store.save(kept.clone())).unwrap();
        let mut bob = alice_session(token);
        bob.user_id = UserId::parse("@bob:example.org").unwrap();

        assert_eq!(
            block_on(store.save(bob.clone())),
            Err(SessionStoreError::Unavailable),
            "token {token:?} is refused"
        );

        assert_eq!(
            block_on(store.list()),
            Ok(vec![kept.clone()]),
            "after refusing {token:?} the saved sessions are intact"
        );
        assert_eq!(block_on(store.load(bob.user_id)), Ok(None));
    }
}

#[test]
fn replacing_a_session_by_one_that_cannot_be_held_keeps_the_old_one() {
    let keychain = Arc::new(MemoryKeychain::default());
    let store = store(&keychain);
    block_on(store.save(alice_session("good"))).unwrap();

    assert!(block_on(store.save(alice_session("bad token"))).is_err());

    assert_eq!(block_on(store.list()), Ok(vec![alice_session("good")]));
}
