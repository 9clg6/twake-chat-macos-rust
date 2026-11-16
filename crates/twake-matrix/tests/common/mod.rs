//! Shared helpers of the adapter tests: a mock homeserver on loopback, no real account.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    dead_code
)]

pub mod history;
pub mod seed;

use matrix_sdk::Client;
use matrix_sdk::ruma::user_id;
use matrix_sdk::test_utils::mocks::{LoginResponseTemplate200, MatrixMockServer};
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};

/// A store key: 32 bytes, as 64 hex characters (see the report: open question on the format).
pub fn key(byte: u8) -> Secret {
    Secret::new(format!("{byte:02x}").repeat(32))
}

pub fn homeserver(server: &MatrixMockServer) -> HomeserverUrl {
    HomeserverUrl::parse(&server.uri()).unwrap()
}

pub fn alice() -> UserId {
    UserId::parse("@alice:example.org").unwrap()
}

pub fn device(id: &str) -> DeviceId {
    DeviceId::parse(id).unwrap()
}

/// What the homeserver answers to a successful login of alice.
pub fn login_ok(token: &str, device: &str) -> LoginResponseTemplate200 {
    LoginResponseTemplate200::new(
        token,
        matrix_sdk::ruma::OwnedDeviceId::from(device),
        user_id!("@alice:example.org").to_owned(),
    )
}

pub fn saved_session(server: &MatrixMockServer, token: &str, device_id: &str) -> Session {
    Session {
        user_id: alice(),
        device_id: device(device_id),
        access_token: Secret::new(token),
        homeserver: homeserver(server),
    }
}

/// Mounts `/versions`, which every client asks before its first call.
pub async fn serve_versions(server: &MatrixMockServer) {
    server.mock_versions().ok().mount().await;
}

/// A client of `server` with a store in `dir`, built by the adapter under test.
pub async fn client(server: &MatrixMockServer, dir: &std::path::Path) -> Client {
    twake_matrix::build_client(&homeserver(server), dir, &key(1))
        .await
        .unwrap()
}

/// The JSON bodies the server received on `path`.
pub async fn bodies(server: &MatrixMockServer, path: &str) -> Vec<serde_json::Value> {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|r| r.url.path() == path)
        .map(|r| serde_json::from_slice(&r.body).unwrap_or_default())
        .collect()
}
