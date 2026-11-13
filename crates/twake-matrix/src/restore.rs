//! Restoring a saved session into a client at start-up (FR-014, D72): same device, no network.

use matrix_sdk::authentication::matrix::MatrixSession;
use matrix_sdk::ruma::{OwnedDeviceId, OwnedUserId};
use matrix_sdk::{Client, SessionMeta, SessionTokens};
use twake_domain::session::Session;

/// Why a session was not restored. Never carries the token.
#[derive(Debug, thiserror::Error)]
pub enum RestoreError {
    /// The session was opened on another homeserver than the client's: its token goes nowhere.
    #[error("the session belongs to another homeserver")]
    WrongHomeserver,
    /// The client refused the session.
    #[error("the session could not be restored")]
    Failed,
}

/// Signs `client` in again with `session`, on its device.
pub async fn restore_session(client: &Client, session: &Session) -> Result<(), RestoreError> {
    if client.homeserver().as_str().trim_end_matches('/') != session.homeserver.as_str() {
        return Err(RestoreError::WrongHomeserver);
    }
    let user_id =
        OwnedUserId::try_from(session.user_id.as_str()).map_err(|_| RestoreError::Failed)?;
    let restored = MatrixSession {
        meta: SessionMeta {
            user_id,
            device_id: OwnedDeviceId::from(session.device_id.as_str()),
        },
        tokens: SessionTokens {
            access_token: session.access_token.expose().to_owned(),
            refresh_token: None,
        },
    };
    client
        .restore_session(restored)
        .await
        .map_err(|_| RestoreError::Failed)
}
