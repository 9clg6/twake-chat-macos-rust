//! The two calls the wiring needs besides login and restore: a client to exchange a token before
//! the account is known, and a check of a restored session against the server (A034).

use matrix_sdk::Client;
use twake_domain::session::HomeserverUrl;

use crate::client::ClientError;
use crate::login::{LoginError, classify};

/// A client of `homeserver` with no store on disk, signed out. The folder and the key of an
/// account depend on its user id, which a login token does not tell: log in here, then restore
/// the session into the client of the account (`build_client`, `restore_session`).
pub async fn login_client(homeserver: &HomeserverUrl) -> Result<Client, ClientError> {
    Client::builder()
        .homeserver_url(homeserver.as_str())
        .request_config(matrix_sdk::config::RequestConfig::short_retry())
        .build()
        .await
        .map_err(|_| ClientError::StoreUnavailable)
}

/// Asks the server who the token of `client` belongs to: `InvalidToken` when it refuses it (a
/// 401 also reaches `expiry`), `Unreachable` or `Server` when it could not say.
pub async fn check_session(client: &Client) -> Result<(), LoginError> {
    client
        .whoami()
        .await
        .map(|_| ())
        .map_err(|e| classify(&e.into()))
}
