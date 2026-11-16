//! A session store with no keychain: sessions in memory, in the order they were first saved.

use std::sync::Mutex;

use futures::FutureExt;
use futures::future::BoxFuture;
use twake_app::ports::{MAX_ACCOUNTS, SessionStoreError, SessionStorePort};
use twake_domain::UserId;
use twake_domain::session::Session;

use crate::subscribers::locked;

/// Holds the sessions of the accounts; `unavailable` makes every call fail.
pub struct FakeSessionStore {
    sessions: Mutex<Vec<Session>>,
    available: bool,
}

impl Default for FakeSessionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeSessionStore {
    /// An empty store.
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(Vec::new()),
            available: true,
        }
    }

    /// Every call returns `Unavailable`.
    pub fn unavailable(mut self) -> Self {
        self.available = false;
        self
    }

    fn run<T: Send + 'static>(
        &self,
        action: impl FnOnce(&mut Vec<Session>) -> Result<T, SessionStoreError>,
    ) -> BoxFuture<'_, Result<T, SessionStoreError>> {
        let result = if self.available {
            action(&mut locked(&self.sessions))
        } else {
            Err(SessionStoreError::Unavailable)
        };
        std::future::ready(result).boxed()
    }
}

impl SessionStorePort for FakeSessionStore {
    fn save(&self, session: Session) -> BoxFuture<'_, Result<(), SessionStoreError>> {
        self.run(|sessions| {
            match sessions.iter().position(|s| s.user_id == session.user_id) {
                Some(place) => sessions[place] = session,
                None if sessions.len() >= MAX_ACCOUNTS => {
                    return Err(SessionStoreError::TooManyAccounts);
                }
                None => sessions.push(session),
            }
            Ok(())
        })
    }

    fn load(&self, user: UserId) -> BoxFuture<'_, Result<Option<Session>, SessionStoreError>> {
        self.run(|sessions| Ok(sessions.iter().find(|s| s.user_id == user).cloned()))
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, SessionStoreError>> {
        self.run(|sessions| Ok(sessions.clone()))
    }

    fn clear(&self, user: UserId) -> BoxFuture<'_, Result<(), SessionStoreError>> {
        self.run(|sessions| {
            sessions.retain(|s| s.user_id != user);
            Ok(())
        })
    }
}
