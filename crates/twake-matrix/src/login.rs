//! SSO sign-in: the `loginToken` of the callback becomes a `Session` (FR-012, FR-013).

use matrix_sdk::ruma::api::error::ErrorKind;
use matrix_sdk::{Client, Error, HttpError};
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};

/// Why the sign-in failed. Never carries the token.
#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    /// The homeserver refused the token.
    #[error("the login token was refused")]
    InvalidToken,
    /// The homeserver could not be reached.
    #[error("the homeserver is unreachable")]
    Unreachable,
    /// The homeserver failed or answered something unexpected.
    #[error("the homeserver failed")]
    Server,
}

/// Signs `client` in with `token`, asking for `device` again when given (keepDevice).
pub async fn login_with_token(
    client: &Client,
    token: &Secret,
    device: Option<&DeviceId>,
) -> Result<Session, LoginError> {
    let auth = client.matrix_auth();
    let mut request = auth.login_token(token.expose());
    if let Some(device) = device {
        request = request.device_id(device.as_str());
    }
    request.send().await.map_err(|e| classify(&e))?;
    session_of(client).ok_or(LoginError::Server)
}

fn classify(error: &Error) -> LoginError {
    if let Some(api) = error.as_client_api_error() {
        return match error.client_api_error_kind() {
            _ if api.status_code.is_server_error() => LoginError::Server,
            Some(ErrorKind::LimitExceeded(_)) => LoginError::Server,
            Some(_) => LoginError::InvalidToken,
            // A client error with no Matrix kind: the server answered, it is not a transport failure.
            None => LoginError::Server,
        };
    }
    match error {
        Error::Http(http) if is_transport(http) => LoginError::Unreachable,
        _ => LoginError::Server,
    }
}

fn is_transport(error: &HttpError) -> bool {
    matches!(error, HttpError::Reqwest(r) if r.status().is_none())
}

fn session_of(client: &Client) -> Option<Session> {
    Some(Session {
        user_id: UserId::parse(client.user_id()?.as_str()).ok()?,
        device_id: DeviceId::parse(client.device_id()?.as_str()).ok()?,
        access_token: Secret::new(client.access_token()?),
        homeserver: HomeserverUrl::parse(client.homeserver().as_str().trim_end_matches('/'))
            .ok()?,
    })
}
