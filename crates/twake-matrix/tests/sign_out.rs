//! Leaving an account (sign-out) and losing it (401), as the device is concerned
//! (A017 Q7, `keepDevice` of the React `session.tsx`):
//! - sign-out: the server forgets the device and its keys, so the local data is wiped and the
//!   device id is not asked for again;
//! - expiry (401): the device and its local keys are kept for the next sign-in.
//!
//! Expected: `sign_out(Client, &Path, &UserId) -> Result<(), SignOutError>`: the folder erased is
//! `account_data_dir(root, user)` and nothing else (the keychain entry is cleared by the caller
//! through `SessionStorePort`);
//! `expiry(&Client) -> impl Stream<Item = Expiry>` with `Expiry::{Soft, Hard}` (`soft_logout`
//! true / false of the 401).
#![recursion_limit = "256"]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use common::{alice, client, homeserver, saved_session};
use futures::StreamExt;
use matrix_sdk::Client;
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use std::path::Path;
use std::time::Duration;
use twake_domain::UserId;
use twake_matrix::{Expiry, account_data_dir, expiry, restore_session, sign_out};
use wiremock::ResponseTemplate;

/// A client signed in as alice, its store in `account_data_dir(root, alice)`.
async fn signed_in(server: &MatrixMockServer, root: &Path) -> Client {
    let client = client(server, &account_data_dir(root, &alice())).await;
    restore_session(&client, &saved_session(server, "tok", "DEVICE1"))
        .await
        .unwrap();
    client
}

fn is_wiped(dir: &Path) -> bool {
    !dir.exists() || std::fs::read_dir(dir).unwrap().next().is_none()
}

#[tokio::test]
async fn signing_out_tells_the_server_with_the_access_token() {
    let server = MatrixMockServer::new().await;
    server.mock_versions().ok().mount().await;
    server
        .mock_logout()
        .expect_access_token("tok")
        .ok()
        .expect(1)
        .mount()
        .await;
    let root = tempfile::tempdir().unwrap();
    let client = signed_in(&server, root.path()).await;

    sign_out(client, root.path(), &alice()).await.unwrap();
}

#[tokio::test]
async fn signing_out_wipes_the_store_and_the_keys_of_that_account_only() {
    let server = MatrixMockServer::new().await;
    server.mock_versions().ok().mount().await;
    server.mock_logout().ok().mount().await;
    let root = tempfile::tempdir().unwrap();
    let mine = account_data_dir(root.path(), &alice());
    let bob = UserId::parse("@bob:example.org").unwrap();
    let other = account_data_dir(root.path(), &bob);
    let client = signed_in(&server, root.path()).await;
    drop(common::client(&server, &other).await);

    sign_out(client, root.path(), &alice()).await.unwrap();

    assert!(is_wiped(&mine), "the folder of the account is emptied");
    assert!(!is_wiped(&other), "another account is untouched");
}

#[tokio::test]
async fn signing_out_leaves_the_sibling_files_of_the_root() {
    let server = MatrixMockServer::new().await;
    server.mock_versions().ok().mount().await;
    server.mock_logout().ok().mount().await;
    let root = tempfile::tempdir().unwrap();
    let note = root.path().join("settings.json");
    std::fs::write(&note, "keep me").unwrap();
    let stray = root.path().join("stray");
    std::fs::create_dir(&stray).unwrap();
    std::fs::write(stray.join("f"), "keep me too").unwrap();
    let client = signed_in(&server, root.path()).await;

    sign_out(client, root.path(), &alice()).await.unwrap();

    assert!(is_wiped(&account_data_dir(root.path(), &alice())));
    assert_eq!(std::fs::read_to_string(&note).unwrap(), "keep me");
    assert_eq!(
        std::fs::read_to_string(stray.join("f")).unwrap(),
        "keep me too"
    );
}

#[tokio::test]
async fn signing_out_when_the_root_does_not_exist_is_fine() {
    let server = MatrixMockServer::new().await;
    server.mock_versions().ok().mount().await;
    server.mock_logout().ok().mount().await;
    let elsewhere = tempfile::tempdir().unwrap();
    let client = signed_in(&server, elsewhere.path()).await;
    let missing = elsewhere.path().join("no").join("such").join("root");

    let done = sign_out(client, &missing, &alice()).await;

    assert_eq!(done.ok(), Some(()));
    assert!(!missing.exists(), "nothing is created");
}

#[cfg(unix)]
#[tokio::test]
async fn a_symbolic_link_in_place_of_the_account_folder_never_costs_its_target() {
    let server = MatrixMockServer::new().await;
    server.mock_versions().ok().mount().await;
    server.mock_logout().ok().mount().await;
    let outside = tempfile::tempdir().unwrap();
    let precious = outside.path().join("precious");
    std::fs::write(&precious, "do not delete").unwrap();
    let root = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), account_data_dir(root.path(), &alice())).unwrap();
    let client = common::client(&server, &root.path().join("elsewhere")).await;

    let _ = sign_out(client, root.path(), &alice()).await;

    assert_eq!(std::fs::read_to_string(&precious).unwrap(), "do not delete");
}

#[tokio::test]
async fn signing_out_works_when_the_server_cannot_be_reached() {
    let (server, dir) = {
        let gone = MatrixMockServer::new().await;
        let dir = tempfile::tempdir().unwrap();
        let client = signed_in(&gone, dir.path()).await;
        ((client, homeserver(&gone)), dir)
    }; // the server is dropped before the sign-out
    let (client, _home) = server;

    let done = tokio::time::timeout(
        Duration::from_secs(30),
        sign_out(client, dir.path(), &alice()),
    )
    .await;

    assert!(
        matches!(done, Ok(Ok(()))),
        "sign-out ends even offline: {done:?}"
    );
    assert!(is_wiped(&account_data_dir(dir.path(), &alice())));
}

#[tokio::test]
async fn signing_out_works_when_the_token_is_already_refused() {
    let server = MatrixMockServer::new().await;
    server.mock_versions().ok().mount().await;
    server
        .mock_logout()
        .error_unknown_token(false)
        .mount()
        .await;
    let dir = tempfile::tempdir().unwrap();
    let client = signed_in(&server, dir.path()).await;

    sign_out(client, dir.path(), &alice()).await.unwrap();

    assert!(is_wiped(&account_data_dir(dir.path(), &alice())));
}

#[tokio::test]
async fn a_soft_logout_is_reported_and_keeps_the_device_and_its_keys() {
    let server = MatrixMockServer::new().await;
    server.mock_versions().ok().mount().await;
    server
        .mock_who_am_i()
        .expect_access_token("tok")
        .error_unknown_token(true)
        .mount()
        .await;
    let dir = tempfile::tempdir().unwrap();
    let client = signed_in(&server, dir.path()).await;
    let mut events = Box::pin(expiry(&client));

    let _ = client.whoami().await; // the 401

    let first = tokio::time::timeout(Duration::from_secs(5), events.next()).await;
    assert_eq!(first, Ok(Some(Expiry::Soft)));
    assert!(
        !is_wiped(&account_data_dir(dir.path(), &alice())),
        "the local data stays"
    );
    assert_eq!(
        client.device_id().map(|d| d.as_str()),
        Some("DEVICE1"),
        "the device is kept"
    );
}

#[tokio::test]
async fn a_hard_logout_is_told_apart_from_a_soft_one() {
    let server = MatrixMockServer::new().await;
    server.mock_versions().ok().mount().await;
    server
        .mock_who_am_i()
        .expect_access_token("tok")
        .respond_with(ResponseTemplate::new(401).set_body_json(
            serde_json::json!({"errcode": "M_UNKNOWN_TOKEN", "error": "gone", "soft_logout": false}),
        ))
        .mount()
        .await;
    let dir = tempfile::tempdir().unwrap();
    let client = signed_in(&server, dir.path()).await;
    let mut events = Box::pin(expiry(&client));

    let _ = client.whoami().await;

    let first = tokio::time::timeout(Duration::from_secs(5), events.next()).await;
    assert_eq!(first, Ok(Some(Expiry::Hard)));
}

#[tokio::test]
async fn no_expiry_is_reported_while_the_session_is_valid() {
    let server = MatrixMockServer::new().await;
    server.mock_versions().ok().mount().await;
    server
        .mock_who_am_i()
        .expect_access_token("tok")
        .ok()
        .mount()
        .await;
    let dir = tempfile::tempdir().unwrap();
    let client = signed_in(&server, dir.path()).await;
    let mut events = Box::pin(expiry(&client));

    client.whoami().await.unwrap();

    let nothing = tokio::time::timeout(Duration::from_millis(300), events.next()).await;
    assert!(nothing.is_err(), "no event expected, got {nothing:?}");
}
