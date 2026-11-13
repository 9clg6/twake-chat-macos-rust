//! Leaving an account and losing it (A017 Q7, `keepDevice` of the React app).

use std::path::Path;

use futures::{Stream, StreamExt};
use matrix_sdk::{Client, SessionChange};
use tokio::sync::broadcast::error::RecvError;

/// Why a sign-out did not finish.
#[derive(Debug, thiserror::Error)]
pub enum SignOutError {
    /// The folder of the account could not be emptied.
    #[error("the account data could not be erased")]
    Wipe,
}

/// How a session was lost: a 401 with `soft_logout` true or false.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expiry {
    /// The token expired; the device and its keys are kept for the next sign-in.
    Soft,
    /// The server dropped the device.
    Hard,
}

/// Signs out and erases `dir`, the folder of the account. The server is asked once; if it
/// cannot answer, the local data goes anyway.
pub async fn sign_out(client: Client, dir: &Path) -> Result<(), SignOutError> {
    let _ = client.matrix_auth().logout().await; // ponytail: offline or 401 still wipes
    drop(client); // closes the SQLite files
    match std::fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(SignOutError::Wipe),
    }
}

/// Each time the server refuses the token of `client`.
pub fn expiry(client: &Client) -> impl Stream<Item = Expiry> + use<> {
    let changes = client.subscribe_to_session_changes();
    futures::stream::unfold(changes, |mut changes| async move {
        loop {
            match changes.recv().await {
                Ok(SessionChange::UnknownToken(data)) => {
                    let kind = if data.soft_logout {
                        Expiry::Soft
                    } else {
                        Expiry::Hard
                    };
                    return Some((kind, changes));
                }
                Ok(_) | Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => return None,
            }
        }
    })
    .boxed()
}
