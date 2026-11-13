//! The client factory: a `matrix_sdk::Client` on the homeserver, with an encrypted SQLite store
//! per account (ARCHITECTURE.md section 7, A019, A025). No network: nothing here logs in.
//!
//! Expected public surface of `twake_matrix`:
//! `install_tls_provider()`, `account_data_dir(root, &UserId) -> PathBuf`,
//! `build_client(&HomeserverUrl, &Path, &Secret) -> Result<Client, ClientError>` with
//! `ClientError::{InvalidStoreKey, StoreUnavailable}`; the key is 64 hex characters (32 raw bytes).
#![recursion_limit = "256"]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use common::{alice, key};
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use twake_domain::UserId;
use twake_domain::session::{HomeserverUrl, Secret};
use twake_matrix::{ClientError, account_data_dir, build_client, install_tls_provider};

#[tokio::test]
async fn a_client_talks_to_the_homeserver_it_was_built_for() {
    let server = MatrixMockServer::new().await;
    let dir = tempfile::tempdir().unwrap();
    let client = build_client(&common::homeserver(&server), dir.path(), &key(1))
        .await
        .unwrap();
    assert_eq!(
        client.homeserver().as_str().trim_end_matches('/'),
        server.uri()
    );
}

#[tokio::test]
async fn a_new_client_is_signed_out() {
    let server = MatrixMockServer::new().await;
    let dir = tempfile::tempdir().unwrap();
    let client = build_client(&common::homeserver(&server), dir.path(), &key(1))
        .await
        .unwrap();
    assert!(client.user_id().is_none());
    assert!(client.access_token().is_none());
}

#[tokio::test]
async fn the_store_is_created_in_the_data_folder_even_when_it_does_not_exist_yet() {
    let server = MatrixMockServer::new().await;
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("accounts").join("alice");
    let _client = build_client(&common::homeserver(&server), &dir, &key(1))
        .await
        .unwrap();
    assert!(dir.is_dir(), "the account folder exists");
    assert!(
        std::fs::read_dir(&dir).unwrap().next().is_some(),
        "the SQLite store is in it"
    );
}

#[tokio::test]
async fn a_store_reopens_with_its_key() {
    let server = MatrixMockServer::new().await;
    let dir = tempfile::tempdir().unwrap();
    let home = common::homeserver(&server);
    drop(build_client(&home, dir.path(), &key(1)).await.unwrap());
    assert!(build_client(&home, dir.path(), &key(1)).await.is_ok());
}

#[tokio::test]
async fn a_store_does_not_open_with_another_key_and_the_error_shows_no_key() {
    let server = MatrixMockServer::new().await;
    let dir = tempfile::tempdir().unwrap();
    let home = common::homeserver(&server);
    drop(build_client(&home, dir.path(), &key(1)).await.unwrap());
    let refused = build_client(&home, dir.path(), &key(2))
        .await
        .expect_err("refused");
    let text = format!("{refused} {refused:?}");
    assert!(
        !text.contains(key(1).expose()) && !text.contains(key(2).expose()),
        "{text}"
    );
}

#[tokio::test]
async fn a_key_that_is_not_32_bytes_is_refused_without_echoing_it() {
    let server = MatrixMockServer::new().await;
    let home = common::homeserver(&server);
    let too_short = "ab".repeat(31);
    let too_long = "ab".repeat(33);
    let not_hex = "zz".repeat(32);
    for bad in [
        "",
        "secret",
        too_short.as_str(),
        too_long.as_str(),
        not_hex.as_str(),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let refused = build_client(&home, dir.path(), &Secret::new(bad))
            .await
            .expect_err("refused");
        assert!(
            matches!(refused, ClientError::InvalidStoreKey),
            "{bad:?} gave {refused:?}"
        );
        let text = format!("{refused} {refused:?}");
        assert!(
            bad.is_empty() || !text.contains(bad),
            "the key leaks: {text}"
        );
        assert!(
            std::fs::read_dir(dir.path()).unwrap().next().is_none(),
            "no store for a bad key"
        );
    }
}

#[tokio::test]
async fn an_https_homeserver_builds_after_the_tls_provider_is_installed_twice() {
    install_tls_provider();
    install_tls_provider();
    let dir = tempfile::tempdir().unwrap();
    let home = HomeserverUrl::parse("https://matrix.example.org").unwrap();
    assert!(build_client(&home, dir.path(), &key(1)).await.is_ok());
}

#[test]
fn each_account_has_its_own_folder_under_the_root_whatever_its_id() {
    let root = std::path::Path::new("/data/accounts");
    let bob = UserId::parse("@bob:example.org").unwrap();
    let sneaky = UserId::parse("@../../etc:example.org").unwrap();
    let slash = UserId::parse("@a/b:example.org").unwrap();
    let dirs: Vec<_> = [&alice(), &bob, &sneaky, &slash]
        .iter()
        .map(|u| account_data_dir(root, u))
        .collect();
    for dir in &dirs {
        assert!(
            dir.starts_with(root) && dir != root,
            "{dir:?} is not inside the root"
        );
        assert!(
            dir.strip_prefix(root)
                .unwrap()
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
            "{dir:?} escapes by a path segment"
        );
    }
    let distinct: std::collections::HashSet<_> = dirs.iter().collect();
    assert_eq!(distinct.len(), dirs.len(), "two accounts share a folder");
    assert_eq!(
        account_data_dir(root, &alice()),
        dirs[0],
        "the same account, the same folder"
    );
}
