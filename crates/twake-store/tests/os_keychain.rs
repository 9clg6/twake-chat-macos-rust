//! `OsKeychain`: the `Keychain` trait over `keyring-core` (spike S3, A005, A027). The OS is the
//! store given to it: these tests hand it the mock store of `keyring-core` (memory, no OS call);
//! the one test on the real macOS keychain is `#[ignore]` since it writes in the keychain of
//! whoever runs it: `cargo test -p twake-store --test os_keychain -- --ignored`.
//!
//! Expected public surface of `twake_store` (P2: `keyring*` lives only in this crate):
//! - `const KEYCHAIN_SERVICE: &str = "twake-chat"`, the service name every entry of the app is
//!   filed under (the application prefix: no other app of the user shares it);
//! - `OsKeychain::new(store: Arc<keyring_core::CredentialStore>) -> OsKeychain` (service
//!   `KEYCHAIN_SERVICE`) and `OsKeychain::with_service(store, service: impl Into<String>)`
//!   (tests and parallel installs); `OsKeychain: Keychain`;
//! - `platform_store() -> Result<Arc<keyring_core::CredentialStore>, KeychainError>`: the store of
//!   the OS the build targets (macOS Keychain, Windows Credential Manager, Secret Service).
//!
//! Limits, documented not tested: Windows Credential Manager holds at most 2560 bytes per entry
//! (`CRED_MAX_CREDENTIAL_BLOB_SIZE`), below the 4 KB the mock takes; five sessions of
//! `KeychainSessionStore` weigh about 1 KB, so they fit, and a value over the limit is a
//! `KeychainError`, never a silent cut.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::sync::Arc;

use futures::executor::block_on;
use keyring_core::api::CredentialStoreApi;
use keyring_core::{CredentialStore, Error as CoreError, mock};
use twake_app::ports::{SessionStoreError, SessionStorePort};
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};
use twake_store::{KEYCHAIN_SERVICE, Keychain, KeychainSessionStore, OsKeychain};
use twake_testing::contract;

fn mock_store() -> Arc<mock::Store> {
    mock::Store::new().unwrap()
}

fn keychain_over(store: &Arc<mock::Store>) -> OsKeychain {
    let as_store: Arc<CredentialStore> = store.clone();
    OsKeychain::new(as_store)
}

/// Makes the next call on the entry `(service, name)` of the mock store fail with `error`.
fn fail_next_call(store: &Arc<mock::Store>, service: &str, name: &str, error: CoreError) {
    let entry = store.build(service, name, None).unwrap();
    let cred: &mock::Cred = entry.as_any().downcast_ref().unwrap();
    cred.set_error(error);
}

fn secret(text: &str) -> Secret {
    Secret::new(text)
}

fn read(keychain: &OsKeychain, entry: &str) -> Option<String> {
    keychain.get(entry).unwrap().map(|s| s.expose().to_owned())
}

#[test]
fn a_secret_is_read_back_as_it_was_set() {
    let keychain = keychain_over(&mock_store());
    keychain.set("sessions", &secret("tok-1")).unwrap();
    assert_eq!(read(&keychain, "sessions").as_deref(), Some("tok-1"));
}

#[test]
fn an_entry_never_set_is_none_not_an_error() {
    let keychain = keychain_over(&mock_store());
    assert_eq!(read(&keychain, "sessions"), None);
}

#[test]
fn setting_an_entry_again_replaces_its_value() {
    let keychain = keychain_over(&mock_store());
    keychain.set("sessions", &secret("old")).unwrap();
    keychain.set("sessions", &secret("new")).unwrap();
    assert_eq!(read(&keychain, "sessions").as_deref(), Some("new"));
}

#[test]
fn entries_are_independent() {
    let keychain = keychain_over(&mock_store());
    keychain.set("a", &secret("1")).unwrap();
    keychain.set("b", &secret("2")).unwrap();
    keychain.delete("a").unwrap();
    assert_eq!(read(&keychain, "a"), None);
    assert_eq!(read(&keychain, "b").as_deref(), Some("2"));
}

#[test]
fn a_deleted_entry_reads_as_none() {
    let keychain = keychain_over(&mock_store());
    keychain.set("sessions", &secret("tok")).unwrap();
    keychain.delete("sessions").unwrap();
    assert_eq!(read(&keychain, "sessions"), None);
}

#[test]
fn deleting_an_entry_that_is_not_there_is_fine() {
    let keychain = keychain_over(&mock_store());
    keychain.delete("never-set").unwrap();
    keychain.set("x", &secret("1")).unwrap();
    keychain.delete("x").unwrap();
    keychain.delete("x").unwrap();
}

#[test]
fn a_unicode_value_is_read_back_whole() {
    let keychain = keychain_over(&mock_store());
    let text = "accès é 日本語 \u{1F512} ñ\u{0301} émoji 🔑";
    keychain.set("sessions", &secret(text)).unwrap();
    assert_eq!(read(&keychain, "sessions").as_deref(), Some(text));
}

#[test]
fn a_unicode_entry_name_works() {
    let keychain = keychain_over(&mock_store());
    keychain
        .set("clé:@élodie:exemple.org", &secret("v"))
        .unwrap();
    assert_eq!(
        read(&keychain, "clé:@élodie:exemple.org").as_deref(),
        Some("v")
    );
}

#[test]
fn a_value_of_four_kilobytes_is_read_back_whole() {
    let keychain = keychain_over(&mock_store());
    let long = "x".repeat(4096);
    keychain.set("sessions", &secret(&long)).unwrap();
    assert_eq!(read(&keychain, "sessions"), Some(long));
}

#[test]
fn an_empty_value_is_read_back_as_empty_not_as_absent() {
    let keychain = keychain_over(&mock_store());
    keychain.set("sessions", &secret("")).unwrap();
    assert_eq!(read(&keychain, "sessions").as_deref(), Some(""));
}

#[test]
fn entries_are_filed_under_the_service_of_the_application() {
    assert!(KEYCHAIN_SERVICE.starts_with("twake-chat"));
    let store = mock_store();
    keychain_over(&store)
        .set("sessions", &secret("tok"))
        .unwrap();
    let entry = store.build(KEYCHAIN_SERVICE, "sessions", None).unwrap();
    assert_eq!(entry.get_password().unwrap(), "tok");
}

#[test]
fn two_services_over_one_store_do_not_see_each_other() {
    let store = mock_store();
    let as_store: Arc<CredentialStore> = store.clone();
    let one = OsKeychain::with_service(as_store.clone(), "twake-chat-one");
    let two = OsKeychain::with_service(as_store, "twake-chat-two");
    one.set("sessions", &secret("from-one")).unwrap();
    assert_eq!(read(&two, "sessions"), None);
    two.set("sessions", &secret("from-two")).unwrap();
    one.delete("sessions").unwrap();
    assert_eq!(read(&one, "sessions"), None);
    assert_eq!(read(&two, "sessions").as_deref(), Some("from-two"));
}

#[test]
fn a_failing_backend_on_get_is_a_keychain_error() {
    let store = mock_store();
    let keychain = keychain_over(&store);
    keychain.set("sessions", &secret("tok")).unwrap();
    fail_next_call(
        &store,
        KEYCHAIN_SERVICE,
        "sessions",
        CoreError::NoStorageAccess("locked".into()),
    );
    assert!(keychain.get("sessions").is_err());
    // The error was one call: the entry is still there.
    assert_eq!(read(&keychain, "sessions").as_deref(), Some("tok"));
}

#[test]
fn a_failing_backend_on_set_is_a_keychain_error() {
    let store = mock_store();
    let keychain = keychain_over(&store);
    fail_next_call(
        &store,
        KEYCHAIN_SERVICE,
        "sessions",
        CoreError::PlatformFailure("denied".into()),
    );
    assert!(keychain.set("sessions", &secret("tok")).is_err());
}

#[test]
fn a_failing_backend_on_delete_is_a_keychain_error() {
    let store = mock_store();
    let keychain = keychain_over(&store);
    keychain.set("sessions", &secret("tok")).unwrap();
    fail_next_call(
        &store,
        KEYCHAIN_SERVICE,
        "sessions",
        CoreError::NoStorageAccess("locked".into()),
    );
    assert!(keychain.delete("sessions").is_err());
}

#[test]
fn a_value_over_the_limit_of_the_os_is_an_error_not_a_cut() {
    let store = mock_store();
    let keychain = keychain_over(&store);
    fail_next_call(
        &store,
        KEYCHAIN_SERVICE,
        "sessions",
        CoreError::TooLong("password".into(), 2560),
    );
    assert!(
        keychain
            .set("sessions", &secret(&"x".repeat(4096)))
            .is_err()
    );
}

#[test]
fn a_value_that_is_not_text_is_an_error_without_the_bytes_in_it() {
    let store = mock_store();
    let keychain = keychain_over(&store);
    fail_next_call(
        &store,
        KEYCHAIN_SERVICE,
        "sessions",
        CoreError::BadEncoding(b"hunter2-bytes".to_vec()),
    );
    let err = keychain.get("sessions").unwrap_err();
    assert!(!format!("{err} {err:?}").contains("hunter2"));
}

#[test]
fn no_error_of_the_backend_shows_the_secret_in_display_or_debug() {
    let leaky = "hunter2-the-secret";
    let errors = [
        CoreError::PlatformFailure(leaky.into()),
        CoreError::NoStorageAccess(leaky.into()),
        CoreError::BadDataFormat(leaky.as_bytes().to_vec(), leaky.into()),
        CoreError::BadStoreFormat(leaky.to_owned()),
        CoreError::Invalid("password".to_owned(), leaky.to_owned()),
        CoreError::NotSupportedByStore(leaky.to_owned()),
    ];
    for error in errors {
        let store = mock_store();
        let keychain = keychain_over(&store);
        keychain.set("sessions", &secret(leaky)).unwrap();
        fail_next_call(&store, KEYCHAIN_SERVICE, "sessions", error);
        let err = keychain.get("sessions").unwrap_err();
        let shown = format!("{err} {err:?}");
        assert!(!shown.contains("hunter2"), "leaked: {shown}");
    }
}

#[test]
fn the_secret_set_is_not_in_the_error_of_a_failed_set() {
    let store = mock_store();
    let keychain = keychain_over(&store);
    fail_next_call(
        &store,
        KEYCHAIN_SERVICE,
        "sessions",
        CoreError::Invalid("password".into(), "bad value hunter2-the-secret".into()),
    );
    let err = keychain
        .set("sessions", &secret("hunter2-the-secret"))
        .unwrap_err();
    assert!(!format!("{err} {err:?}").contains("hunter2"));
}

#[test]
fn the_keychain_passes_the_contract_suite_of_the_session_store() {
    contract::session_store(|| {
        Arc::new(KeychainSessionStore::new(Arc::new(keychain_over(
            &mock_store(),
        ))))
    })
    .expect("contract");
}

#[test]
fn sessions_survive_a_new_keychain_over_the_same_store() {
    let store = mock_store();
    let user = UserId::parse("@alice:example.org").unwrap();
    let session = Session {
        user_id: user.clone(),
        device_id: DeviceId::parse("DEVICEA").unwrap(),
        access_token: secret("syt_token"),
        homeserver: HomeserverUrl::parse("https://matrix.example.org").unwrap(),
    };
    block_on(KeychainSessionStore::new(Arc::new(keychain_over(&store))).save(session)).unwrap();
    let restarted = KeychainSessionStore::new(Arc::new(keychain_over(&store)));
    let back = block_on(restarted.load(user)).unwrap().unwrap();
    assert_eq!(back.access_token.expose(), "syt_token");
}

#[test]
fn a_failing_keychain_makes_the_session_store_unavailable_never_empty() {
    let store = mock_store();
    let sessions = KeychainSessionStore::new(Arc::new(keychain_over(&store)));
    fail_next_call(
        &store,
        KEYCHAIN_SERVICE,
        "sessions",
        CoreError::NoStorageAccess("locked".into()),
    );
    assert_eq!(
        block_on(sessions.list()),
        Err(SessionStoreError::Unavailable)
    );
}

/// Removes the entry when dropped, so a failed assertion leaves nothing in the keychain.
#[cfg(target_os = "macos")]
struct Cleanup(OsKeychain);

#[cfg(target_os = "macos")]
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = self.0.delete("sessions");
    }
}

/// Touches the keychain of the user who runs it: never by default.
#[cfg(target_os = "macos")]
#[test]
#[ignore = "writes in the login keychain of the user: cargo test -p twake-store --test os_keychain -- --ignored"]
fn the_macos_keychain_stores_reads_and_deletes_an_entry() {
    let service = format!("twake-chat-test-{}", uuid::Uuid::new_v4());
    let on_os = || {
        OsKeychain::with_service(
            twake_store::platform_store().expect("the macOS keychain"),
            service.clone(),
        )
    };
    let _cleanup = Cleanup(on_os());
    let keychain = on_os();
    assert_eq!(read(&keychain, "sessions"), None);
    keychain.set("sessions", &secret("tok é 日本")).unwrap();
    assert_eq!(read(&on_os(), "sessions").as_deref(), Some("tok é 日本"));
    keychain.delete("sessions").unwrap();
    assert_eq!(read(&keychain, "sessions"), None);
    keychain.delete("sessions").unwrap();
}
