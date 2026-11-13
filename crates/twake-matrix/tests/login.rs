//! SSO sign-in: the `loginToken` of the callback (already validated by `twake_app::sso`) becomes
//! a domain `Session` (FR-012, FR-013, A017).
//!
//! Expected: `login_with_token(&Client, &Secret, Option<&DeviceId>) -> Result<Session, LoginError>`
//! with `LoginError::{InvalidToken, Unreachable, Server}`. The device given is asked of the server
//! again (keepDevice of the React app); none, a new device.
#![recursion_limit = "256"]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use common::{alice, bodies, client, device, homeserver, login_ok, serve_versions};
use matrix_sdk::test_utils::mocks::MatrixMockServer;
use twake_domain::session::{HomeserverUrl, Secret, Session};
use twake_matrix::{LoginError, build_client, login_with_token};
use wiremock::ResponseTemplate;

const TOKEN: &str = "login-token-4f9c1a";

#[tokio::test]
async fn a_valid_login_token_gives_the_session_of_the_account() {
    let server = MatrixMockServer::new().await;
    serve_versions(&server).await;
    server
        .mock_login()
        .ok_with(login_ok("syt_access", "DEVICE1"))
        .mount()
        .await;
    let dir = tempfile::tempdir().unwrap();
    let client = client(&server, dir.path()).await;

    let session = login_with_token(&client, &Secret::new(TOKEN), None)
        .await
        .unwrap();

    assert_eq!(
        session,
        Session {
            user_id: alice(),
            device_id: device("DEVICE1"),
            access_token: Secret::new("syt_access"),
            homeserver: homeserver(&server),
        }
    );
}

#[tokio::test]
async fn the_client_is_signed_in_afterwards() {
    let server = MatrixMockServer::new().await;
    serve_versions(&server).await;
    server
        .mock_login()
        .ok_with(login_ok("syt_access", "DEVICE1"))
        .mount()
        .await;
    let dir = tempfile::tempdir().unwrap();
    let client = client(&server, dir.path()).await;

    login_with_token(&client, &Secret::new(TOKEN), None)
        .await
        .unwrap();

    assert_eq!(
        client.user_id().map(|u| u.as_str()),
        Some("@alice:example.org")
    );
    assert_eq!(client.access_token().as_deref(), Some("syt_access"));
}

#[tokio::test]
async fn the_server_receives_a_token_login_with_that_token() {
    let server = MatrixMockServer::new().await;
    serve_versions(&server).await;
    server
        .mock_login()
        .ok_with(login_ok("syt_access", "DEVICE1"))
        .mount()
        .await;
    let dir = tempfile::tempdir().unwrap();
    let client = client(&server, dir.path()).await;

    login_with_token(&client, &Secret::new(TOKEN), None)
        .await
        .unwrap();

    let sent = bodies(&server, "/_matrix/client/v3/login").await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0]["type"], "m.login.token");
    assert_eq!(sent[0]["token"], TOKEN);
    assert!(
        sent[0].get("device_id").is_none(),
        "no device asked for a first sign-in: {}",
        sent[0]
    );
}

#[tokio::test]
async fn a_remembered_device_is_asked_for_again() {
    let server = MatrixMockServer::new().await;
    serve_versions(&server).await;
    server
        .mock_login()
        .ok_with(login_ok("syt_access", "OLDDEVICE"))
        .mount()
        .await;
    let dir = tempfile::tempdir().unwrap();
    let client = client(&server, dir.path()).await;

    let session = login_with_token(&client, &Secret::new(TOKEN), Some(&device("OLDDEVICE")))
        .await
        .unwrap();

    let sent = bodies(&server, "/_matrix/client/v3/login").await;
    assert_eq!(sent[0]["device_id"], "OLDDEVICE");
    assert_eq!(session.device_id, device("OLDDEVICE"));
}

#[tokio::test]
async fn a_refused_token_is_an_invalid_token_and_leaks_nothing() {
    let server = MatrixMockServer::new().await;
    serve_versions(&server).await;
    // A homeserver may echo what it received: the error must still not carry the token.
    let echo = format!("Invalid login token {TOKEN}");
    server
        .mock_login()
        .respond_with(
            ResponseTemplate::new(403)
                .set_body_json(serde_json::json!({"errcode": "M_FORBIDDEN", "error": echo})),
        )
        .mount()
        .await;
    let dir = tempfile::tempdir().unwrap();
    let client = client(&server, dir.path()).await;

    let refused = login_with_token(&client, &Secret::new(TOKEN), None)
        .await
        .expect_err("refused");

    assert!(matches!(refused, LoginError::InvalidToken), "{refused:?}");
    let text = format!("{refused} | {refused:?}");
    assert!(!text.contains(TOKEN), "the token leaks: {text}");
    assert!(client.access_token().is_none(), "still signed out");
}

#[tokio::test]
async fn a_server_failure_is_not_taken_for_a_bad_token() {
    let server = MatrixMockServer::new().await;
    serve_versions(&server).await;
    server.mock_login().error500().mount().await;
    let dir = tempfile::tempdir().unwrap();
    let client = client(&server, dir.path()).await;

    let failed = login_with_token(&client, &Secret::new(TOKEN), None)
        .await
        .expect_err("failed");

    assert!(matches!(failed, LoginError::Server), "{failed:?}");
    let text = format!("{failed} | {failed:?}");
    assert!(!text.contains(TOKEN), "the token leaks: {text}");
}

#[tokio::test]
async fn a_closed_port_is_reported_as_unreachable_and_leaks_nothing() {
    // A real refusal of the connection: a port that was bound, then released.
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let home = HomeserverUrl::parse(&format!("http://127.0.0.1:{port}")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let client = build_client(&home, dir.path(), &common::key(1))
        .await
        .unwrap();

    let failed = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        login_with_token(&client, &Secret::new(TOKEN), None),
    )
    .await
    .expect("answers in time")
    .expect_err("failed");

    assert!(matches!(failed, LoginError::Unreachable), "{failed:?}");
    let text = format!("{failed} | {failed:?}");
    assert!(!text.contains(TOKEN), "the token leaks: {text}");
}

/// What the login of `TOKEN` gives when the server answers `response`.
async fn login_answered(response: ResponseTemplate) -> LoginError {
    let server = MatrixMockServer::new().await;
    serve_versions(&server).await;
    server.mock_login().respond_with(response).mount().await;
    let dir = tempfile::tempdir().unwrap();
    let client = client(&server, dir.path()).await;
    login_with_token(&client, &Secret::new(TOKEN), None)
        .await
        .expect_err("refused")
}

fn matrix_error(status: u16, errcode: &str) -> ResponseTemplate {
    ResponseTemplate::new(status)
        .set_body_json(serde_json::json!({"errcode": errcode, "error": "no"}))
}

#[tokio::test]
async fn a_rate_limited_login_is_a_server_failure_not_a_bad_token() {
    let limited = login_answered(ResponseTemplate::new(429).set_body_json(
        serde_json::json!({"errcode": "M_LIMIT_EXCEEDED", "error": "slow", "retry_after_ms": 10}),
    ))
    .await;
    assert!(matches!(limited, LoginError::Server), "{limited:?}");
}

#[tokio::test]
async fn a_client_error_that_is_not_matrix_is_a_server_answer_not_unreachable() {
    // The server answered (a proxy, a wrong URL): it is up, it just does not speak Matrix.
    for status in [400, 404] {
        let got = login_answered(ResponseTemplate::new(status)).await;
        assert!(matches!(got, LoginError::Server), "{status}: {got:?}");
    }
}

#[tokio::test]
async fn a_matrix_refusal_of_the_token_is_an_invalid_token() {
    for (status, errcode) in [(403, "M_FORBIDDEN"), (401, "M_UNKNOWN_TOKEN")] {
        let got = login_answered(matrix_error(status, errcode)).await;
        assert!(
            matches!(got, LoginError::InvalidToken),
            "{status} {errcode}: {got:?}"
        );
    }
}

#[tokio::test]
async fn a_server_error_without_a_matrix_body_is_a_server_failure() {
    for status in [500, 502, 503] {
        let got = login_answered(ResponseTemplate::new(status)).await;
        assert!(matches!(got, LoginError::Server), "{status}: {got:?}");
    }
}

#[tokio::test]
async fn a_server_error_with_a_matrix_body_is_a_server_failure() {
    let got = login_answered(matrix_error(500, "M_UNKNOWN")).await;
    assert!(matches!(got, LoginError::Server), "{got:?}");
}
