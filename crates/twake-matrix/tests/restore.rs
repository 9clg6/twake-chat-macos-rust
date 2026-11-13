//! Restoring a saved `Session` into a client at start-up (FR-014, D72): same device, same keys,
//! no network.
//!
//! Expected: `restore_session(&Client, &Session) -> Result<(), RestoreError>` with
//! `RestoreError::{WrongHomeserver, Failed}`.
#![recursion_limit = "256"]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use common::{alice, client, device, homeserver, saved_session};
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use twake_domain::session::HomeserverUrl;
use twake_matrix::{RestoreError, build_client, restore_session};

#[tokio::test]
async fn a_restored_client_is_the_saved_account_on_the_saved_device() {
    let server = MatrixMockServer::new().await;
    let dir = tempfile::tempdir().unwrap();
    let client = client(&server, dir.path()).await;

    restore_session(
        &client,
        &saved_session(&server, "restored-token", "DEVICE1"),
    )
    .await
    .unwrap();

    assert_eq!(client.user_id().map(|u| u.as_str()), Some(alice().as_str()));
    assert_eq!(
        client.device_id().map(|d| d.as_str()),
        Some(device("DEVICE1").as_str())
    );
    assert_eq!(client.access_token().as_deref(), Some("restored-token"));
}

#[tokio::test]
async fn restoring_needs_no_network() {
    let server = MatrixMockServer::new().await; // nothing mounted: any call would fail
    let dir = tempfile::tempdir().unwrap();
    let client = client(&server, dir.path()).await;

    restore_session(
        &client,
        &saved_session(&server, "restored-token", "DEVICE1"),
    )
    .await
    .unwrap();

    assert!(
        server
            .received_requests()
            .await
            .unwrap_or_default()
            .is_empty()
    );
}

#[tokio::test]
async fn the_restored_token_is_the_one_sent_to_the_server() {
    let server = MatrixMockServer::new().await;
    server.mock_versions().ok().mount().await;
    server
        .mock_who_am_i()
        .expect_access_token("restored-token")
        .ok()
        .expect(1)
        .mount()
        .await;
    let dir = tempfile::tempdir().unwrap();
    let client = client(&server, dir.path()).await;
    restore_session(
        &client,
        &saved_session(&server, "restored-token", "DEVICE1"),
    )
    .await
    .unwrap();

    client.whoami().await.unwrap();
}

#[tokio::test]
async fn a_session_of_another_homeserver_is_refused_and_its_token_goes_nowhere() {
    let server = MatrixMockServer::new().await;
    let dir = tempfile::tempdir().unwrap();
    let client = client(&server, dir.path()).await;
    let mut other = saved_session(&server, "restored-token", "DEVICE1");
    other.homeserver = HomeserverUrl::parse("https://other.example.org").unwrap();

    let refused = restore_session(&client, &other).await.expect_err("refused");

    assert!(
        matches!(refused, RestoreError::WrongHomeserver),
        "{refused:?}"
    );
    let text = format!("{refused} | {refused:?}");
    assert!(!text.contains("restored-token"), "{text}");
    assert!(
        client.access_token().is_none(),
        "the client stays signed out"
    );
    assert_eq!(
        homeserver(&server).as_str(),
        client.homeserver().as_str().trim_end_matches('/')
    );
}

#[tokio::test]
async fn a_client_restored_after_a_restart_keeps_its_device() {
    let server = MatrixMockServer::new().await;
    let dir = tempfile::tempdir().unwrap();
    let home = homeserver(&server);
    let session = saved_session(&server, "restored-token", "DEVICE1");
    {
        let first = build_client(&home, dir.path(), &common::key(1))
            .await
            .unwrap();
        restore_session(&first, &session).await.unwrap();
    }
    let second = build_client(&home, dir.path(), &common::key(1))
        .await
        .unwrap();
    restore_session(&second, &session).await.unwrap();
    assert_eq!(second.device_id().map(|d| d.as_str()), Some("DEVICE1"));
}
