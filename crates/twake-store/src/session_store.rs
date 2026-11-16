//! `SessionStorePort` over the keychain (A005, NFR-015, D373): one entry holds every session.
//! One line per session, `user device homeserver token`: none of the four has whitespace, so a
//! split on whitespace reads it back. One entry keeps the order of the accounts and every
//! change atomic.

use std::sync::{Arc, Mutex};

use futures::future::BoxFuture;
use twake_app::ports::{MAX_ACCOUNTS, SessionStoreError, SessionStorePort};
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};

use crate::keychain::{Keychain, KeychainError, off_thread};

const ENTRY: &str = "sessions";

impl From<KeychainError> for SessionStoreError {
    fn from(_: KeychainError) -> Self {
        Self::Unavailable
    }
}

/// The sessions of the signed-in accounts, kept in the OS keychain. Every call of the keychain
/// runs on a blocking thread, once the future is awaited (never on the UI thread).
pub struct KeychainSessionStore {
    inner: Arc<Inner>,
}

struct Inner {
    keychain: Arc<dyn Keychain>,
    /// Read-modify-write of the one entry is not interleaved.
    lock: Mutex<()>,
}

impl KeychainSessionStore {
    /// A store over `keychain`.
    pub fn new(keychain: Arc<dyn Keychain>) -> Self {
        Self {
            inner: Arc::new(Inner {
                keychain,
                lock: Mutex::new(()),
            }),
        }
    }

    /// Runs `action` on a thread of its own; a panic of the backend is an unavailable store.
    fn run<T: Send + 'static>(
        &self,
        action: impl FnOnce(&Inner) -> Result<T, SessionStoreError> + Send + 'static,
    ) -> BoxFuture<'_, Result<T, SessionStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let done = off_thread(move || {
                let _guard = inner.lock.lock().unwrap_or_else(|p| p.into_inner());
                action(&inner)
            });
            done.await.unwrap_or(Err(SessionStoreError::Unavailable))
        })
    }
}

impl Inner {
    fn read(&self) -> Result<Vec<Session>, SessionStoreError> {
        match self.keychain.get(ENTRY)? {
            None => Ok(Vec::new()),
            Some(text) => decode(text.expose()),
        }
    }

    fn write(&self, sessions: &[Session]) -> Result<(), SessionStoreError> {
        Ok(self.keychain.set(ENTRY, &encode(sessions))?)
    }
}

fn encode(sessions: &[Session]) -> Secret {
    Secret::new(
        sessions
            .iter()
            .map(|s| {
                format!(
                    "{} {} {} {}\n",
                    s.user_id.as_str(),
                    s.device_id,
                    s.homeserver.as_str(),
                    s.access_token.expose()
                )
            })
            .collect::<String>(),
    )
}

/// A token the line format can read back: non-empty, no whitespace, no control character.
fn holdable(token: &str) -> bool {
    !token.is_empty() && !token.chars().any(|c| c.is_whitespace() || c.is_control())
}

fn decode(text: &str) -> Result<Vec<Session>, SessionStoreError> {
    text.lines()
        .map(|line| decode_line(line).ok_or(SessionStoreError::Unavailable))
        .collect()
}

fn decode_line(line: &str) -> Option<Session> {
    let mut parts = line.split_whitespace();
    let session = Session {
        user_id: UserId::parse(parts.next()?).ok()?,
        device_id: DeviceId::parse(parts.next()?).ok()?,
        homeserver: HomeserverUrl::parse(parts.next()?).ok()?,
        access_token: Secret::new(parts.next()?),
    };
    parts.next().is_none().then_some(session)
}

impl SessionStorePort for KeychainSessionStore {
    fn save(&self, session: Session) -> BoxFuture<'_, Result<(), SessionStoreError>> {
        self.run(move |store| {
            if !holdable(session.access_token.expose()) {
                return Err(SessionStoreError::Unavailable);
            }
            let mut sessions = store.read()?;
            match sessions.iter().position(|s| s.user_id == session.user_id) {
                Some(place) => sessions[place] = session,
                None if sessions.len() >= MAX_ACCOUNTS => {
                    return Err(SessionStoreError::TooManyAccounts);
                }
                None => sessions.push(session),
            }
            store.write(&sessions)
        })
    }

    fn load(&self, user: UserId) -> BoxFuture<'_, Result<Option<Session>, SessionStoreError>> {
        self.run(move |store| Ok(store.read()?.into_iter().find(|s| s.user_id == user)))
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, SessionStoreError>> {
        self.run(Inner::read)
    }

    fn clear(&self, user: UserId) -> BoxFuture<'_, Result<(), SessionStoreError>> {
        self.run(move |store| {
            let mut sessions = store.read()?;
            sessions.retain(|s| s.user_id != user);
            store.write(&sessions)
        })
    }
}
