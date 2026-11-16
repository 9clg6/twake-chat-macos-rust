//! What the sign-in screen shows, as a view model: texts and buttons for each `LoginView`
//! (FR-012, FR-013, FR-015, D396, A005, A033). No window.

use twake_app::login_screen::{LoginProblem, LoginView};
use twake_app::sso::CallbackError;
use twake_i18n::{Arg, Locale};

/// What a button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginAction {
    /// Look the server entered up.
    SubmitServer,
    /// Open the browser for the SSO.
    OpenBrowser,
    /// Give up the current step.
    Cancel,
    /// Sign out (also: try the erase again).
    SignOut,
    /// Sign an ended account in again.
    AddAgain,
    /// Read the keychain again.
    RetryStart,
}

/// A button: its text and what it does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginButton {
    /// The text, on one line.
    pub label: String,
    /// What it does.
    pub action: LoginAction,
}

/// The sign-in screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginScreenView {
    /// The heading.
    pub title: String,
    /// What the screen says; it names the server or the account.
    pub message: Option<String>,
    /// Why the last attempt failed.
    pub problem: Option<String>,
    /// In the order shown, the first being the main one.
    pub buttons: Vec<LoginButton>,
    /// Something runs: no way to hurry it.
    pub busy: bool,
}

impl LoginScreenView {
    /// The screen of `view` in the language of `locale`.
    pub fn from(view: &LoginView, locale: &Locale) -> Self {
        let t = Texts(locale);
        match view {
            LoginView::EnterServer { problem } => t.enter_server(problem),
            LoginView::Discovering => t.discovering(),
            LoginView::ChooseSignIn {
                homeserver,
                problem,
            } => t.choose(homeserver.as_str(), problem),
            LoginView::WaitingForBrowser {
                homeserver,
                problem,
            } => t.waiting(homeserver.as_str(), problem),
            LoginView::SigningIn { homeserver } => t.signing_in(homeserver.as_str()),
            LoginView::SignedIn { user } => t.signed_in(user.as_str()),
            LoginView::SessionExpired { user } => t.expired(user.as_str()),
            LoginView::SigningOut { user } => t.signing_out(user.as_str()),
            LoginView::SignOutFailed { user } => t.sign_out_failed(user.as_str()),
            LoginView::StoreUnavailable => t.store_unavailable(),
        }
    }
}

/// The catalogue, with the shapes the screens share.
struct Texts<'a>(&'a Locale);

impl Texts<'_> {
    fn tr(&self, key: &str) -> String {
        self.0.tr(key, &[])
    }

    fn named(&self, key: &str, name: &str, value: &str) -> String {
        self.0.tr(key, &[(name, Arg::Str(value))])
    }

    fn button(&self, key: &str, action: LoginAction) -> LoginButton {
        LoginButton {
            label: self.tr(key),
            action,
        }
    }

    fn cancel(&self) -> LoginButton {
        self.button("login-button-cancel", LoginAction::Cancel)
    }

    fn screen(&self, title: &str, message: Option<String>) -> LoginScreenView {
        LoginScreenView {
            title: self.tr(title),
            message,
            problem: None,
            buttons: vec![],
            busy: false,
        }
    }

    fn enter_server(&self, problem: &Option<LoginProblem>) -> LoginScreenView {
        LoginScreenView {
            problem: problem_text(problem, self.0),
            buttons: vec![self.button("login-button-continue", LoginAction::SubmitServer)],
            ..self.screen(
                "login-title-enter-server",
                Some(self.tr("login-message-enter-server")),
            )
        }
    }

    fn discovering(&self) -> LoginScreenView {
        LoginScreenView {
            busy: true,
            buttons: vec![self.cancel()],
            ..self.screen("login-title-discovering", None)
        }
    }

    /// A screen that names the server or the account: keys `login-title-{which}` and
    /// `login-message-{which}`.
    fn about(
        &self,
        which: &str,
        name: &str,
        value: &str,
        buttons: Vec<LoginButton>,
    ) -> LoginScreenView {
        LoginScreenView {
            buttons,
            ..self.screen(
                &format!("login-title-{which}"),
                Some(self.named(&format!("login-message-{which}"), name, value)),
            )
        }
    }

    fn choose(&self, server: &str, problem: &Option<LoginProblem>) -> LoginScreenView {
        let buttons = vec![
            self.button("login-button-open-browser", LoginAction::OpenBrowser),
            self.cancel(),
        ];
        LoginScreenView {
            problem: problem_text(problem, self.0),
            ..self.about("choose", "server", server, buttons)
        }
    }

    fn waiting(&self, server: &str, problem: &Option<LoginProblem>) -> LoginScreenView {
        LoginScreenView {
            problem: problem_text(problem, self.0),
            busy: true,
            ..self.about("waiting", "server", server, vec![self.cancel()])
        }
    }

    fn signing_in(&self, server: &str) -> LoginScreenView {
        LoginScreenView {
            busy: true,
            ..self.about("signing-in", "server", server, vec![])
        }
    }

    fn signed_in(&self, user: &str) -> LoginScreenView {
        let sign_out = self.button("login-button-sign-out", LoginAction::SignOut);
        self.about("signed-in", "user", user, vec![sign_out])
    }

    fn expired(&self, user: &str) -> LoginScreenView {
        let again = self.button("login-button-add-again", LoginAction::AddAgain);
        self.about("expired", "user", user, vec![again])
    }

    fn signing_out(&self, user: &str) -> LoginScreenView {
        LoginScreenView {
            busy: true,
            ..self.about("signing-out", "user", user, vec![])
        }
    }

    fn sign_out_failed(&self, user: &str) -> LoginScreenView {
        let retry = self.button("login-button-retry", LoginAction::SignOut);
        LoginScreenView {
            problem: Some(self.tr("login-problem-sign-out-failed")),
            ..self.about("sign-out-failed", "user", user, vec![retry])
        }
    }

    fn store_unavailable(&self) -> LoginScreenView {
        LoginScreenView {
            buttons: vec![self.button("login-button-retry", LoginAction::RetryStart)],
            ..self.screen(
                "login-title-store-unavailable",
                Some(self.tr("login-message-store-unavailable")),
            )
        }
    }
}

fn problem_text(problem: &Option<LoginProblem>, locale: &Locale) -> Option<String> {
    let key = match problem.as_ref()? {
        LoginProblem::ServerNotFound => "login-problem-server-not-found",
        LoginProblem::Unreachable => "login-problem-unreachable",
        LoginProblem::Server => "login-problem-server",
        LoginProblem::InvalidToken => "login-problem-invalid-token",
        LoginProblem::BrowserCallback(error) => callback_key(error),
    };
    Some(locale.tr(key, &[]))
}

fn callback_key(error: &CallbackError) -> &'static str {
    match error {
        CallbackError::Malformed => "login-problem-callback-malformed",
        CallbackError::WrongTarget => "login-problem-callback-wrong-target",
        CallbackError::MissingState => "login-problem-callback-missing-state",
        CallbackError::WrongState => "login-problem-callback-wrong-state",
        CallbackError::Replayed => "login-problem-callback-used",
        CallbackError::MissingToken => "login-problem-callback-missing-token",
        CallbackError::DuplicateParameter => "login-problem-callback-duplicate",
    }
}
