//! The sign-in screen as a model: what it shows, on top of the session machine (FR-012, FR-013,
//! FR-015, D396). Pure: no UI, no net.

use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl};

use crate::session::{SessionEvent, SessionMachine, SessionState};
use crate::sso::CallbackError;

/// Why the last attempt did not go through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginProblem {
    /// The server entered was not found.
    ServerNotFound,
    /// The server could not be reached.
    Unreachable,
    /// The server answered with an error.
    Server,
    /// The server refused the token.
    InvalidToken,
    /// The browser came back with a callback that was refused.
    BrowserCallback(CallbackError),
}

/// What the screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginView {
    /// Asks for the server.
    EnterServer {
        /// Why the last attempt failed.
        problem: Option<LoginProblem>,
    },
    /// Looks the server up.
    Discovering,
    /// Offers the sign-in.
    ChooseSignIn {
        /// The server.
        homeserver: HomeserverUrl,
        /// Why the last attempt failed.
        problem: Option<LoginProblem>,
    },
    /// The browser is open, the callback is awaited.
    WaitingForBrowser {
        /// The server.
        homeserver: HomeserverUrl,
        /// Why a callback was refused.
        problem: Option<LoginProblem>,
    },
    /// The token is exchanged, or a stored session checked.
    SigningIn {
        /// The server.
        homeserver: HomeserverUrl,
    },
    /// Signed in.
    SignedIn {
        /// The account.
        user: UserId,
    },
    /// The server refused the session.
    SessionExpired {
        /// The account.
        user: UserId,
    },
    /// The sign-out runs.
    SigningOut {
        /// The account.
        user: UserId,
    },
    /// The data of the account could not be erased.
    SignOutFailed {
        /// The account.
        user: UserId,
    },
}

/// The sign-in screen: the session machine, whether the browser is open, the last problem.
#[derive(Debug, Default)]
pub struct LoginScreen {
    machine: SessionMachine,
    browser_open: bool,
    problem: Option<LoginProblem>,
}

impl LoginScreen {
    /// Signed out.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds the machine; a new step clears the old problem.
    pub fn apply(&mut self, event: SessionEvent) {
        let failed_lookup = event == SessionEvent::DiscoveryFailed;
        self.machine.apply(event);
        self.browser_open = false;
        let out = self.machine.state() == &SessionState::SignedOut;
        self.problem = (failed_lookup && out).then_some(LoginProblem::ServerNotFound);
    }

    /// The browser was opened: the callback is awaited.
    pub fn browser_opened(&mut self) {
        if matches!(self.machine.state(), SessionState::AwaitingLogin(_)) {
            self.browser_open = true;
            self.problem = None;
        }
    }

    /// A callback was refused: the request stays answerable.
    pub fn callback_refused(&mut self, error: CallbackError) {
        if self.browser_open {
            self.problem = Some(LoginProblem::BrowserCallback(error));
        }
    }

    /// The login failed: back to the choice, with the reason.
    pub fn login_failed(&mut self, problem: LoginProblem) {
        self.apply(SessionEvent::LoginFailed);
        if matches!(self.machine.state(), SessionState::AwaitingLogin(_)) {
            self.problem = Some(problem);
        }
    }

    /// Gives up the steps before `Active`.
    pub fn cancel(&mut self) {
        self.apply(SessionEvent::Cancelled);
    }

    /// What to show.
    pub fn view(&self) -> LoginView {
        let problem = self.problem.clone();
        match self.machine.state() {
            SessionState::SignedOut => LoginView::EnterServer { problem },
            SessionState::Discovering => LoginView::Discovering,
            SessionState::AwaitingLogin(homeserver) => {
                self.choice_view(homeserver.clone(), problem)
            }
            SessionState::LoggingIn(homeserver) => LoginView::SigningIn {
                homeserver: homeserver.clone(),
            },
            account => account_view(account),
        }
    }

    fn choice_view(&self, homeserver: HomeserverUrl, problem: Option<LoginProblem>) -> LoginView {
        if self.browser_open {
            LoginView::WaitingForBrowser {
                homeserver,
                problem,
            }
        } else {
            LoginView::ChooseSignIn {
                homeserver,
                problem,
            }
        }
    }

    /// The machine under the screen.
    pub fn session_state(&self) -> &SessionState {
        self.machine.state()
    }

    /// The device of the machine.
    pub fn device_id(&self) -> Option<&DeviceId> {
        self.machine.device_id()
    }
}

/// The views of a signed-in account: it is named, whatever happens to it.
fn account_view(state: &SessionState) -> LoginView {
    match state {
        SessionState::Active(s) => LoginView::SignedIn {
            user: s.user_id.clone(),
        },
        SessionState::Expired(s) => LoginView::SessionExpired {
            user: s.user_id.clone(),
        },
        SessionState::SigningOut(s) => LoginView::SigningOut {
            user: s.user_id.clone(),
        },
        SessionState::SignOutFailed(s) => LoginView::SignOutFailed {
            user: s.user_id.clone(),
        },
        _ => LoginView::EnterServer { problem: None },
    }
}
