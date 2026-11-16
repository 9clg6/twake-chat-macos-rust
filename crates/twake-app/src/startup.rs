//! What the app does with the store of sessions at start and when a login ends (FR-014, A005).

use twake_domain::UserId;
use twake_domain::session::Session;

use crate::ports::{SessionStoreError, SessionStorePort};

/// Where the app goes at start.
#[derive(Debug, PartialEq, Eq)]
pub enum Startup {
    /// No account is stored.
    SignIn,
    /// The stored accounts, in the order they were added: the first is the default one (D373).
    Restore(Vec<Session>),
    /// The keychain cannot be read: not the same as an empty one, the screen says so.
    StoreUnavailable,
}

/// Reads the store of sessions.
pub async fn startup(store: &dyn SessionStorePort) -> Startup {
    match store.list().await {
        Ok(sessions) if sessions.is_empty() => Startup::SignIn,
        Ok(sessions) => Startup::Restore(sessions),
        Err(_) => Startup::StoreUnavailable,
    }
}

/// Keeps a login for the next start; the error is the caller's to show.
pub async fn remember(
    store: &dyn SessionStorePort,
    session: Session,
) -> Result<(), SessionStoreError> {
    store.save(session).await
}

/// Forgets the login of `user` (sign-out).
pub async fn forget(store: &dyn SessionStorePort, user: UserId) -> Result<(), SessionStoreError> {
    store.clear(user).await
}
