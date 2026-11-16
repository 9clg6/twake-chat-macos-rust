//! The start of the app and the life of the session: the sign-in screen, the keychain and the
//! account side of the Matrix adapter, tied together (A033 "Not done", A034). Pure: the account
//! side is the port `AccountBackend`.

use std::sync::Arc;

use futures::StreamExt;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};

use crate::login_screen::{LoginProblem, LoginScreen, LoginView};
use crate::ports::SessionStorePort;
use crate::session::{SessionEvent, SessionState};
use crate::startup::{Startup, forget, remember, startup};

/// The data of the account could not be erased.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WipeFailed;

/// What the Matrix adapter does for the account, as a port.
pub trait AccountBackend: Send + Sync {
    /// Checks a stored session, with no UI; `Err` names why.
    fn restore(&self, session: Session) -> BoxFuture<'_, Result<(), LoginProblem>>;

    /// Exchanges the token of the browser for a session, asking for `device` again when given.
    fn login_with_token(
        &self,
        homeserver: HomeserverUrl,
        token: Secret,
        device: Option<DeviceId>,
    ) -> BoxFuture<'_, Result<Session, LoginProblem>>;

    /// Signs out and erases the data of the account.
    fn sign_out(&self, session: Session) -> BoxFuture<'_, Result<(), WipeFailed>>;

    /// One item each time the server ends the session of `user` (a 401).
    fn session_lost(&self, user: UserId) -> BoxStream<'static, ()>;
}

/// The controller: owns the sign-in screen, the store of sessions and the backend.
pub struct Boot {
    screen: LoginScreen,
    store: Arc<dyn SessionStorePort>,
    backend: Arc<dyn AccountBackend>,
    store_unavailable: bool,
    remember_failed: bool,
}

impl Boot {
    /// Signed out, nothing read yet.
    pub fn new(store: Arc<dyn SessionStorePort>, backend: Arc<dyn AccountBackend>) -> Self {
        Self {
            screen: LoginScreen::new(),
            store,
            backend,
            store_unavailable: false,
            remember_failed: false,
        }
    }

    /// Reads the keychain and restores the first account (the others are not restored yet, A034).
    /// Callable again: it is the way out of `StoreUnavailable`.
    pub async fn start(&mut self) {
        self.store_unavailable = false;
        match startup(&*self.store).await {
            Startup::SignIn => {}
            Startup::StoreUnavailable => self.store_unavailable = true,
            Startup::Restore(sessions) => {
                if let Some(first) = sessions.into_iter().next() {
                    self.restore(first).await;
                }
            }
        }
    }

    async fn restore(&mut self, session: Session) {
        self.screen.apply(SessionEvent::Restore(session.clone()));
        match self.backend.restore(session.clone()).await {
            Ok(()) => self.screen.apply(SessionEvent::LoginOk(session)),
            Err(problem) => {
                // D396: only a refused token forgets the session; a network loss keeps it.
                if problem == LoginProblem::InvalidToken {
                    let _ = forget(&*self.store, session.user_id).await;
                }
                self.screen.login_failed(problem);
            }
        }
    }

    /// An event of the screen (server entered, resolved...).
    pub fn apply(&mut self, event: SessionEvent) {
        self.screen.apply(event);
    }

    /// The browser came back with `token`: it is exchanged, and the session kept for the next
    /// start. If the keychain refuses it, the user stays signed in for this run (A034).
    pub async fn sso_callback(&mut self, token: Secret) {
        let SessionState::AwaitingLogin(homeserver) = self.screen.session_state().clone() else {
            return;
        };
        let device = self.screen.device_id().cloned();
        self.screen.apply(SessionEvent::SsoCallback);
        match self
            .backend
            .login_with_token(homeserver, token, device)
            .await
        {
            Ok(session) => {
                self.screen.apply(SessionEvent::LoginOk(session.clone()));
                self.remember_failed = remember(&*self.store, session).await.is_err();
            }
            Err(problem) => self.screen.login_failed(problem),
        }
    }

    /// Signs out; a failed erase is shown and keeps the session in the keychain (A034). Asking
    /// again finishes it.
    pub async fn sign_out(&mut self) {
        let session = match self.screen.session_state() {
            SessionState::Active(s) | SessionState::SignOutFailed(s) => s.clone(),
            _ => return,
        };
        self.screen.apply(SessionEvent::SignOut);
        match self.backend.sign_out(session.clone()).await {
            Ok(()) => {
                self.screen.apply(SessionEvent::SignOutDone);
                let _ = forget(&*self.store, session.user_id).await;
            }
            Err(WipeFailed) => self.screen.apply(SessionEvent::SignOutFailed),
        }
    }

    /// An ended session (D396): asks the same server again, so the sign-in reuses the device.
    pub fn add_again(&mut self) {
        if let SessionState::Expired(session) = self.screen.session_state().clone() {
            self.screen.apply(SessionEvent::ServerEntered);
            self.screen
                .apply(SessionEvent::HomeserverResolved(session.homeserver));
        }
    }

    /// While signed in: waits for the server to end the session, then says so and forgets it
    /// (D396). Returns at once when nobody is signed in.
    pub async fn watch_session(&mut self) {
        let SessionState::Active(session) = self.screen.session_state().clone() else {
            return;
        };
        let mut lost = self.backend.session_lost(session.user_id.clone());
        if lost.next().await.is_some() {
            self.screen.apply(SessionEvent::Unauthorized401);
            let _ = forget(&*self.store, session.user_id).await;
        }
    }

    /// What to show.
    pub fn view(&self) -> LoginView {
        if self.store_unavailable {
            LoginView::StoreUnavailable
        } else {
            self.screen.view()
        }
    }

    /// The sign-in worked but the keychain could not keep it: the next start will ask again.
    pub fn remember_failed(&self) -> bool {
        self.remember_failed
    }
}
