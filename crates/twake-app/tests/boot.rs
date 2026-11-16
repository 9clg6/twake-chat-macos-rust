//! The controller of the start of the app and of the life of the session (A033 "Not done": the
//! desktop app did not call `startup`, `remember`, `forget` nor `LoginScreen`). Pure: no window,
//! no net, no keychain; the account side is a fake of the new port `AccountBackend`.
//!
//! Needs: FR-012..FR-015, D72 (session read at boot, checked, forgotten when invalid or at
//! sign-out), D396 (a 401 forgets a session, 429/5xx/network loss keeps it; "Add it again" reuses
//! the device), A005 (a keychain that cannot be read is told, never taken for an empty one), A033
//! item 4 (a failed erase is shown, not hidden).
//!
//! Expected public surface of `twake_app::boot` (new):
//! ```text
//! struct WipeFailed;                                   // the data of the account could not be erased
//! trait AccountBackend: Send + Sync {                  // what the matrix adapter does, as a port
//!     fn restore(&self, session: Session) -> BoxFuture<'_, Result<(), LoginProblem>>;
//!         // checks a stored session (no UI); Err names why
//!     fn login_with_token(&self, homeserver: HomeserverUrl, token: Secret, device: Option<DeviceId>)
//!         -> BoxFuture<'_, Result<Session, LoginProblem>>;
//!     fn sign_out(&self, session: Session) -> BoxFuture<'_, Result<(), WipeFailed>>;
//!     fn session_lost(&self, user: UserId) -> BoxStream<'static, ()>;   // each 401 of that account
//! }
//! struct Boot;                                         // owns a LoginScreen, the store and the backend
//! Boot::new(store: Arc<dyn SessionStorePort>, backend: Arc<dyn AccountBackend>) -> Boot
//! Boot::start(&mut self) -> impl Future<Output = ()>
//!     // `startup(store)`: SignIn -> EnterServer; Restore(list) -> the first one is checked with
//!     // `backend.restore` (view SigningIn meanwhile), Ok -> SignedIn, Err -> ChooseSignIn with the
//!     // reason; StoreUnavailable -> view StoreUnavailable, the backend never called. Callable again
//!     // (the way out of StoreUnavailable).
//! Boot::apply(&mut self, SessionEvent)                 // the events of the screen (server entered...)
//! Boot::sso_callback(&mut self, token: Secret) -> impl Future<Output = ()>
//!     // SsoCallback, then `backend.login_with_token` with the device the machine remembers;
//!     // Ok -> LoginOk + `remember`; Err(problem) -> back to the choice with that problem
//! Boot::sign_out(&mut self) -> impl Future<Output = ()>
//!     // SignOut, `backend.sign_out`; Ok -> SignOutDone + `forget`; Err -> SignOutFailed, shown
//! Boot::watch_session(&mut self) -> impl Future<Output = ()>
//!     // while Active: waits for the first item of `backend.session_lost(user)`, applies
//!     // `Unauthorized401` and forgets the stored session; returns at once when not Active
//! Boot::view(&self) -> LoginView
//! ```
//! And `LoginView::StoreUnavailable` (new variant of `twake_app::login_screen::LoginView`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use futures::FutureExt;
use futures::executor::block_on;
use futures::future::BoxFuture;
use futures::stream::{self, BoxStream, StreamExt};
use twake_app::boot::{AccountBackend, Boot, WipeFailed};
use twake_app::login_screen::{LoginProblem, LoginView};
use twake_app::ports::{SessionStoreError, SessionStorePort};
use twake_app::session::SessionEvent as E;
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};

fn server() -> HomeserverUrl {
    HomeserverUrl::parse("https://matrix.example.org").unwrap()
}

fn alice() -> UserId {
    UserId::parse("@alice:example.org").unwrap()
}

fn session(device: &str) -> Session {
    Session {
        user_id: alice(),
        device_id: DeviceId::parse(device).unwrap(),
        access_token: Secret::new("tok"),
        homeserver: server(),
    }
}

/// The keychain in memory, which can be made to fail and come back.
#[derive(Default)]
struct Store {
    sessions: Mutex<Vec<Session>>,
    down: AtomicBool,
}

impl Store {
    fn with(sessions: Vec<Session>) -> Arc<Self> {
        Arc::new(Self {
            sessions: Mutex::new(sessions),
            down: AtomicBool::new(false),
        })
    }

    fn stored(&self) -> Vec<Session> {
        self.sessions.lock().unwrap().clone()
    }

    fn fails(&self, down: bool) {
        self.down.store(down, Ordering::SeqCst);
    }

    fn run<T: Send + 'static>(
        &self,
        action: impl FnOnce(&mut Vec<Session>) -> T,
    ) -> BoxFuture<'_, Result<T, SessionStoreError>> {
        let result = if self.down.load(Ordering::SeqCst) {
            Err(SessionStoreError::Unavailable)
        } else {
            Ok(action(&mut self.sessions.lock().unwrap()))
        };
        std::future::ready(result).boxed()
    }
}

impl SessionStorePort for Store {
    fn save(&self, session: Session) -> BoxFuture<'_, Result<(), SessionStoreError>> {
        self.run(move |all| {
            all.retain(|s| s.user_id != session.user_id);
            all.push(session);
        })
    }

    fn load(&self, user: UserId) -> BoxFuture<'_, Result<Option<Session>, SessionStoreError>> {
        self.run(move |all| all.iter().find(|s| s.user_id == user).cloned())
    }

    fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, SessionStoreError>> {
        self.run(|all| all.clone())
    }

    fn clear(&self, user: UserId) -> BoxFuture<'_, Result<(), SessionStoreError>> {
        self.run(move |all| all.retain(|s| s.user_id != user))
    }
}

/// What the account side answers, and what it was asked.
struct Backend {
    restore: Result<(), LoginProblem>,
    login: Result<Session, LoginProblem>,
    wipes: Mutex<Vec<Result<(), WipeFailed>>>,
    lost: Mutex<Vec<UserId>>,
    expiries: usize,
    restored: Mutex<Vec<Session>>,
    logins: Mutex<Vec<(Secret, Option<DeviceId>)>>,
    signed_out: Mutex<Vec<Session>>,
}

impl Backend {
    fn new() -> Self {
        Self {
            restore: Ok(()),
            login: Ok(session("NEWDEVICE")),
            wipes: Mutex::new(vec![]),
            lost: Mutex::new(vec![]),
            expiries: 0,
            restored: Mutex::new(vec![]),
            logins: Mutex::new(vec![]),
            signed_out: Mutex::new(vec![]),
        }
    }

    /// The erase answers these, in order, then succeeds.
    fn wiping(self, answers: Vec<Result<(), WipeFailed>>) -> Self {
        *self.wipes.lock().unwrap() = answers;
        self
    }

    fn losing_the_session(mut self, times: usize) -> Self {
        self.expiries = times;
        self
    }
}

impl AccountBackend for Backend {
    fn restore(&self, session: Session) -> BoxFuture<'_, Result<(), LoginProblem>> {
        self.restored.lock().unwrap().push(session);
        std::future::ready(self.restore.clone()).boxed()
    }

    fn login_with_token(
        &self,
        _homeserver: HomeserverUrl,
        token: Secret,
        device: Option<DeviceId>,
    ) -> BoxFuture<'_, Result<Session, LoginProblem>> {
        self.logins.lock().unwrap().push((token, device));
        std::future::ready(self.login.clone()).boxed()
    }

    fn sign_out(&self, session: Session) -> BoxFuture<'_, Result<(), WipeFailed>> {
        self.signed_out.lock().unwrap().push(session);
        let mut wipes = self.wipes.lock().unwrap();
        let answer = if wipes.is_empty() {
            Ok(())
        } else {
            wipes.remove(0)
        };
        std::future::ready(answer).boxed()
    }

    fn session_lost(&self, user: UserId) -> BoxStream<'static, ()> {
        self.lost.lock().unwrap().push(user);
        stream::iter(vec![(); self.expiries]).boxed()
    }
}

fn boot(store: &Arc<Store>, backend: &Arc<Backend>) -> Boot {
    Boot::new(store.clone(), backend.clone())
}

/// A started app, signed in as alice on `DEVICE1` (restored).
fn signed_in(backend: Backend) -> (Boot, Arc<Store>, Arc<Backend>) {
    let store = Store::with(vec![session("DEVICE1")]);
    let backend = Arc::new(backend);
    let mut app = boot(&store, &backend);
    block_on(app.start());
    assert_eq!(app.view(), LoginView::SignedIn { user: alice() });
    (app, store, backend)
}

// ------------------------------------------------------------------ startup

#[test]
fn a_first_start_asks_for_the_server() {
    let store = Store::with(vec![]);
    let backend = Arc::new(Backend::new());
    let mut app = boot(&store, &backend);
    block_on(app.start());
    assert_eq!(app.view(), LoginView::EnterServer { problem: None });
    assert!(backend.restored.lock().unwrap().is_empty());
}

#[test]
fn a_stored_account_is_checked_and_the_user_is_signed_in_again_on_the_same_device() {
    let (_app, _store, backend) = signed_in(Backend::new());
    assert_eq!(*backend.restored.lock().unwrap(), vec![session("DEVICE1")]);
}

#[test]
fn the_first_account_stored_is_the_one_restored() {
    let other = Session {
        user_id: UserId::parse("@bob:example.org").unwrap(),
        ..session("B")
    };
    let store = Store::with(vec![session("DEVICE1"), other]);
    let backend = Arc::new(Backend::new());
    let mut app = boot(&store, &backend);
    block_on(app.start());
    assert_eq!(app.view(), LoginView::SignedIn { user: alice() });
}

#[test]
fn an_unreadable_keychain_is_told_and_never_taken_for_an_empty_one() {
    let store = Store::with(vec![session("DEVICE1")]);
    store.fails(true);
    let backend = Arc::new(Backend::new());
    let mut app = boot(&store, &backend);
    block_on(app.start());
    assert_eq!(app.view(), LoginView::StoreUnavailable);
    assert!(backend.restored.lock().unwrap().is_empty());
    assert!(backend.logins.lock().unwrap().is_empty());
}

#[test]
fn a_keychain_that_comes_back_lets_the_user_start_again() {
    let store = Store::with(vec![session("DEVICE1")]);
    store.fails(true);
    let backend = Arc::new(Backend::new());
    let mut app = boot(&store, &backend);
    block_on(app.start());

    store.fails(false);
    block_on(app.start());

    assert_eq!(app.view(), LoginView::SignedIn { user: alice() });
}

#[test]
fn a_session_the_server_refuses_asks_to_sign_in_again_and_is_forgotten() {
    let store = Store::with(vec![session("DEVICE1")]);
    let backend = Arc::new(Backend {
        restore: Err(LoginProblem::InvalidToken),
        ..Backend::new()
    });
    let mut app = boot(&store, &backend);
    block_on(app.start());
    assert_eq!(
        app.view(),
        LoginView::ChooseSignIn {
            homeserver: server(),
            problem: Some(LoginProblem::InvalidToken)
        }
    );
    assert!(store.stored().is_empty(), "D396: a 401 forgets a session");
}

#[test]
fn a_server_out_of_reach_keeps_the_session_for_the_next_start() {
    let store = Store::with(vec![session("DEVICE1")]);
    let backend = Arc::new(Backend {
        restore: Err(LoginProblem::Unreachable),
        ..Backend::new()
    });
    let mut app = boot(&store, &backend);
    block_on(app.start());
    assert_eq!(
        app.view(),
        LoginView::ChooseSignIn {
            homeserver: server(),
            problem: Some(LoginProblem::Unreachable)
        }
    );
    assert_eq!(
        store.stored(),
        vec![session("DEVICE1")],
        "D396: the network is not a 401"
    );
}

// ------------------------------------------------------------------ SSO

fn at_the_choice(store: &Arc<Store>, backend: &Arc<Backend>) -> Boot {
    let mut app = boot(store, backend);
    block_on(app.start());
    app.apply(E::ServerEntered);
    app.apply(E::HomeserverResolved(server()));
    assert!(matches!(app.view(), LoginView::ChooseSignIn { .. }));
    app
}

#[test]
fn a_successful_sso_signs_in_and_is_remembered_for_the_next_start() {
    let store = Store::with(vec![]);
    let backend = Arc::new(Backend::new());
    let mut app = at_the_choice(&store, &backend);

    block_on(app.sso_callback(Secret::new("login-token")));

    assert_eq!(app.view(), LoginView::SignedIn { user: alice() });
    assert_eq!(store.stored(), vec![session("NEWDEVICE")]);
    let logins = backend.logins.lock().unwrap();
    assert_eq!(logins.len(), 1);
    assert_eq!(logins[0].0.expose(), "login-token");
    assert_eq!(
        logins[0].1, None,
        "a first sign-in has no device to ask again"
    );
}

#[test]
fn a_token_the_server_refuses_goes_back_to_the_choice_and_keeps_nothing() {
    let store = Store::with(vec![]);
    let backend = Arc::new(Backend {
        login: Err(LoginProblem::InvalidToken),
        ..Backend::new()
    });
    let mut app = at_the_choice(&store, &backend);

    block_on(app.sso_callback(Secret::new("bad")));

    assert_eq!(
        app.view(),
        LoginView::ChooseSignIn {
            homeserver: server(),
            problem: Some(LoginProblem::InvalidToken)
        }
    );
    assert!(store.stored().is_empty());
}

// ------------------------------------------------------------------ 401 mid-sync

#[test]
fn a_401_in_the_middle_of_the_sync_reaches_the_session_and_says_so() {
    let (mut app, _store, backend) = signed_in(Backend::new().losing_the_session(1));

    block_on(app.watch_session());

    assert_eq!(app.view(), LoginView::SessionExpired { user: alice() });
    assert_eq!(*backend.lost.lock().unwrap(), vec![alice()]);
}

#[test]
fn an_expired_session_is_forgotten_by_the_keychain() {
    let (mut app, store, _backend) = signed_in(Backend::new().losing_the_session(1));
    block_on(app.watch_session());
    assert!(store.stored().is_empty(), "D396: a 401 forgets a session");
}

#[test]
fn a_session_that_stays_valid_stays_signed_in() {
    let (mut app, store, _backend) = signed_in(Backend::new().losing_the_session(0));
    block_on(app.watch_session());
    assert_eq!(app.view(), LoginView::SignedIn { user: alice() });
    assert_eq!(store.stored().len(), 1);
}

#[test]
fn watching_with_nobody_signed_in_returns_at_once() {
    let store = Store::with(vec![]);
    let backend = Arc::new(Backend::new().losing_the_session(1));
    let mut app = boot(&store, &backend);
    block_on(app.start());
    block_on(app.watch_session());
    assert_eq!(app.view(), LoginView::EnterServer { problem: None });
    assert!(backend.lost.lock().unwrap().is_empty());
}

#[test]
fn adding_the_account_again_after_a_401_asks_for_the_same_device() {
    let (mut app, store, backend) = signed_in(Backend::new().losing_the_session(1));
    block_on(app.watch_session());

    app.apply(E::ServerEntered);
    app.apply(E::HomeserverResolved(server()));
    block_on(app.sso_callback(Secret::new("again")));

    let logins = backend.logins.lock().unwrap();
    assert_eq!(
        logins[0].1,
        Some(DeviceId::parse("DEVICE1").unwrap()),
        "D396: the device is reused"
    );
    assert_eq!(app.view(), LoginView::SignedIn { user: alice() });
    assert_eq!(store.stored().len(), 1);
}

// ------------------------------------------------------------------ sign-out

#[test]
fn signing_out_erases_the_account_and_forgets_its_session() {
    let (mut app, store, backend) = signed_in(Backend::new());

    block_on(app.sign_out());

    assert_eq!(app.view(), LoginView::EnterServer { problem: None });
    assert!(store.stored().is_empty());
    assert_eq!(
        *backend.signed_out.lock().unwrap(),
        vec![session("DEVICE1")]
    );
}

#[test]
fn a_sign_out_that_cannot_erase_is_shown_not_hidden() {
    let (mut app, _store, _backend) = signed_in(Backend::new().wiping(vec![Err(WipeFailed)]));

    block_on(app.sign_out());

    assert_eq!(app.view(), LoginView::SignOutFailed { user: alice() });
}

#[test]
fn after_a_failed_erase_asking_again_finishes_the_sign_out() {
    let (mut app, store, backend) = signed_in(Backend::new().wiping(vec![Err(WipeFailed)]));
    block_on(app.sign_out());

    block_on(app.sign_out());

    assert_eq!(app.view(), LoginView::EnterServer { problem: None });
    assert!(store.stored().is_empty());
    assert_eq!(backend.signed_out.lock().unwrap().len(), 2);
}

#[test]
fn nobody_signs_in_over_data_that_could_not_be_erased() {
    let (mut app, _store, backend) = signed_in(Backend::new().wiping(vec![Err(WipeFailed)]));
    block_on(app.sign_out());

    app.apply(E::ServerEntered);
    block_on(app.sso_callback(Secret::new("tok")));

    assert_eq!(app.view(), LoginView::SignOutFailed { user: alice() });
    assert!(backend.logins.lock().unwrap().is_empty());
}
