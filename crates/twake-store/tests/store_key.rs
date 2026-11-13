//! The key of the SQLite store of a Matrix account (A027 decision 1): 64 hexadecimal characters
//! (32 bytes of a CSPRNG), one per account, kept in the keychain under the user id, never
//! regenerated once it exists (a new key would leave the account's database unreadable).
//!
//! Expected public surface of `twake_store`:
//! - `generate_store_key_with(fill: impl FnOnce(&mut [u8; 32])) -> Secret`: the pure part; `fill`
//!   is the CSPRNG (injected, so a test knows the bytes). The Secret is the 32 bytes as 64
//!   lowercase hexadecimal characters;
//! - `generate_store_key() -> Secret`: the same with the CSPRNG of the OS (`getrandom`);
//! - `store_key_for(keychain: &dyn Keychain, user: &UserId, generate: impl FnOnce() -> Secret)
//!   -> Result<Secret, KeychainError>`: the key of `user`; when the keychain has none, it keeps
//!   `generate()` and returns it; otherwise it returns the kept one and never calls `generate`;
//! - `forget_store_key(keychain: &dyn Keychain, user: &UserId) -> Result<(), KeychainError>`:
//!   removes it (sign-out); the next `store_key_for` makes a new one. A missing key is fine.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::cell::Cell;
use std::sync::Arc;

use keyring_core::{CredentialStore, Error as CoreError, mock};
use twake_domain::UserId;
use twake_domain::session::Secret;
use twake_store::{
    KEYCHAIN_SERVICE, Keychain, OsKeychain, forget_store_key, generate_store_key,
    generate_store_key_with, store_key_for,
};

fn user(name: &str) -> UserId {
    UserId::parse(&format!("@{name}:example.org")).unwrap()
}

fn keychain() -> (Arc<mock::Store>, OsKeychain) {
    let store = mock::Store::new().unwrap();
    let as_store: Arc<CredentialStore> = store.clone();
    (store, OsKeychain::new(as_store))
}

fn is_64_hex(text: &str) -> bool {
    text.len() == 64 && text.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
}

#[test]
fn the_key_is_the_32_bytes_of_the_generator_as_64_lowercase_hexadecimals() {
    let key = generate_store_key_with(|bytes| {
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = u8::try_from(i).unwrap() * 8;
        }
    });
    assert_eq!(
        key.expose(),
        "0008101820283038404850586068707880889098a0a8b0b8c0c8d0d8e0e8f0f8"
    );
}

#[test]
fn every_byte_value_is_written_on_two_characters() {
    let zeros = generate_store_key_with(|bytes| bytes.fill(0x00));
    let ones = generate_store_key_with(|bytes| bytes.fill(0xff));
    assert_eq!(zeros.expose(), "0".repeat(64));
    assert_eq!(ones.expose(), "f".repeat(64));
}

#[test]
fn the_generator_is_asked_for_exactly_32_bytes_once() {
    let calls = Cell::new(0);
    let key = generate_store_key_with(|bytes| {
        calls.set(calls.get() + 1);
        assert_eq!(bytes.len(), 32);
    });
    assert_eq!(calls.get(), 1);
    assert!(is_64_hex(key.expose()));
}

#[test]
fn the_key_of_the_os_generator_is_64_hexadecimals_and_never_the_same_twice() {
    let keys: Vec<Secret> = (0..8).map(|_| generate_store_key()).collect();
    for key in &keys {
        assert!(is_64_hex(key.expose()), "not 64 hexadecimals");
    }
    for (i, a) in keys.iter().enumerate() {
        for b in &keys[i + 1..] {
            assert_ne!(a.expose(), b.expose());
        }
    }
}

#[test]
fn the_first_request_makes_and_keeps_a_key() {
    let (_, keychain) = keychain();
    let key = store_key_for(&keychain, &user("alice"), || Secret::new("a".repeat(64))).unwrap();
    assert_eq!(key.expose(), "a".repeat(64));
}

#[test]
fn a_second_request_returns_the_same_key_and_never_generates_again() {
    let (_, keychain) = keychain();
    let first = store_key_for(&keychain, &user("alice"), || Secret::new("a".repeat(64))).unwrap();
    let second = store_key_for(&keychain, &user("alice"), || {
        panic!("a kept key must never be regenerated")
    })
    .unwrap();
    assert_eq!(first.expose(), second.expose());
}

#[test]
fn the_key_outlives_the_keychain_object() {
    let (store, first_run) = keychain();
    let first = store_key_for(&first_run, &user("alice"), generate_store_key).unwrap();
    drop(first_run);
    let as_store: Arc<CredentialStore> = store;
    let second_run = OsKeychain::new(as_store);
    let again = store_key_for(&second_run, &user("alice"), || panic!("regenerated")).unwrap();
    assert_eq!(first.expose(), again.expose());
}

#[test]
fn each_account_has_its_own_key() {
    let (_, keychain) = keychain();
    let alice = store_key_for(&keychain, &user("alice"), || Secret::new("a".repeat(64))).unwrap();
    let bob = store_key_for(&keychain, &user("bob"), || Secret::new("b".repeat(64))).unwrap();
    assert_ne!(alice.expose(), bob.expose());
    let alice_again = store_key_for(&keychain, &user("alice"), || panic!("regenerated")).unwrap();
    assert_eq!(alice.expose(), alice_again.expose());
}

#[test]
fn the_key_is_not_stored_in_the_sessions_entry_nor_in_a_shared_one() {
    let (_, keychain) = keychain();
    store_key_for(&keychain, &user("alice"), || Secret::new("a".repeat(64))).unwrap();
    assert!(keychain.get("sessions").unwrap().is_none());
}

#[test]
fn a_forgotten_key_is_replaced_by_a_new_one_on_the_next_request() {
    let (_, keychain) = keychain();
    store_key_for(&keychain, &user("alice"), || Secret::new("a".repeat(64))).unwrap();
    forget_store_key(&keychain, &user("alice")).unwrap();
    let new = store_key_for(&keychain, &user("alice"), || Secret::new("c".repeat(64))).unwrap();
    assert_eq!(new.expose(), "c".repeat(64));
}

#[test]
fn forgetting_the_key_of_one_account_leaves_the_others() {
    let (_, keychain) = keychain();
    store_key_for(&keychain, &user("alice"), || Secret::new("a".repeat(64))).unwrap();
    store_key_for(&keychain, &user("bob"), || Secret::new("b".repeat(64))).unwrap();
    forget_store_key(&keychain, &user("alice")).unwrap();
    let bob = store_key_for(&keychain, &user("bob"), || panic!("regenerated")).unwrap();
    assert_eq!(bob.expose(), "b".repeat(64));
}

#[test]
fn forgetting_a_key_that_was_never_made_is_fine() {
    let (_, keychain) = keychain();
    forget_store_key(&keychain, &user("nobody")).unwrap();
}

#[test]
fn a_failing_keychain_is_an_error_and_never_a_silent_new_key() {
    let (store, keychain) = keychain();
    let first = store_key_for(&keychain, &user("alice"), || Secret::new("a".repeat(64))).unwrap();
    assert!(is_64_hex(first.expose()));
    // Break every read of the entries: whichever name the key is kept under, the next call
    // that reads it fails, and must not answer with a freshly generated key.
    for entry in store.inner.lock().unwrap().borrow().iter() {
        entry.set_error(CoreError::NoStorageAccess("locked".into()));
    }
    let generated = Cell::new(false);
    let result = store_key_for(&keychain, &user("alice"), || {
        generated.set(true);
        Secret::new("d".repeat(64))
    });
    assert!(result.is_err());
    assert!(!generated.get(), "a key was generated on a failed read");
}

#[test]
fn the_key_is_filed_under_the_service_of_the_application() {
    let (store, keychain) = keychain();
    store_key_for(&keychain, &user("alice"), || Secret::new("a".repeat(64))).unwrap();
    let kept = store
        .inner
        .lock()
        .unwrap()
        .borrow()
        .iter()
        .any(|c| c.specifiers.0 == KEYCHAIN_SERVICE);
    assert!(kept);
}
