//! The sign-in screen as a model: what it shows and how it moves (FR-012, FR-013, FR-015, D396,
//! ARCHITECTURE section 7: SSO through the system browser, loopback redirect). Pure, no UI, no net.
//!
//! Expected public surface of `twake_app::login_screen` (new), on top of `session::SessionMachine`:
//! ```text
//! LoginScreen::new() -> LoginScreen                        // signed out
//! LoginScreen::apply(&mut self, SessionEvent)              // the machine events, unchanged
//! LoginScreen::browser_opened(&mut self)                   // AwaitingLogin -> WaitingForBrowser
//! LoginScreen::callback_refused(&mut self, CallbackError)  // stays waiting: the request is still answerable
//! LoginScreen::login_failed(&mut self, LoginProblem)       // LoggingIn -> back to the choice, with the reason
//! LoginScreen::cancel(&mut self)                           // any step before Active -> EnterServer
//! LoginScreen::view(&self) -> LoginView
//! LoginScreen::session_state(&self) -> &SessionState       // the machine under it
//! LoginScreen::device_id(&self) -> Option<&DeviceId>        // that of the machine
//! enum LoginProblem { ServerNotFound, Unreachable, Server, InvalidToken, BrowserCallback(CallbackError) }
//! enum LoginView {
//!     EnterServer { problem: Option<LoginProblem> },
//!     Discovering,
//!     ChooseSignIn { homeserver: HomeserverUrl, problem: Option<LoginProblem> },
//!     WaitingForBrowser { homeserver: HomeserverUrl, problem: Option<LoginProblem> },
//!     SigningIn { homeserver: HomeserverUrl },
//!     SignedIn { user: UserId },
//!     SessionExpired { user: UserId },
//!     SigningOut { user: UserId },
//!     SignOutFailed { user: UserId },   // the data of the account could not be erased (Wipe)
//! }
//! ```
//! And in `session`: `SessionEvent::SignOutFailed` (`SigningOut` -> `SessionState::SignOutFailed(Session)`;
//! a new `SignOut` from there asks again; the device is kept, the keys are still on the disk).
//! Every failure has a way out: a screen with a problem accepts the next attempt.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use twake_app::login_screen::{LoginProblem, LoginScreen, LoginView};
use twake_app::session::{SessionEvent as E, SessionState};
use twake_app::sso::CallbackError;
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};

fn server() -> HomeserverUrl {
    HomeserverUrl::parse("https://matrix.example.org").unwrap()
}

fn alice() -> UserId {
    UserId::parse("@alice:example.org").unwrap()
}

fn session() -> Session {
    Session {
        user_id: alice(),
        device_id: DeviceId::parse("DEVICE1").unwrap(),
        access_token: Secret::new("tok"),
        homeserver: server(),
    }
}

fn at_choice() -> LoginScreen {
    let mut screen = LoginScreen::new();
    screen.apply(E::ServerEntered);
    screen.apply(E::HomeserverResolved(server()));
    screen
}

fn waiting() -> LoginScreen {
    let mut screen = at_choice();
    screen.browser_opened();
    screen
}

fn signed_in() -> LoginScreen {
    let mut screen = waiting();
    screen.apply(E::SsoCallback);
    screen.apply(E::LoginOk(session()));
    screen
}

#[test]
fn the_screen_starts_by_asking_for_the_server() {
    assert_eq!(
        LoginScreen::new().view(),
        LoginView::EnterServer { problem: None }
    );
}

#[test]
fn a_server_entered_is_looked_up() {
    let mut screen = LoginScreen::new();
    screen.apply(E::ServerEntered);
    assert_eq!(screen.view(), LoginView::Discovering);
}

#[test]
fn a_server_found_offers_the_sign_in() {
    assert_eq!(
        at_choice().view(),
        LoginView::ChooseSignIn {
            homeserver: server(),
            problem: None
        }
    );
}

#[test]
fn a_server_not_found_says_so_and_asks_again() {
    let mut screen = LoginScreen::new();
    screen.apply(E::ServerEntered);
    screen.apply(E::DiscoveryFailed);
    assert_eq!(
        screen.view(),
        LoginView::EnterServer {
            problem: Some(LoginProblem::ServerNotFound)
        }
    );
    screen.apply(E::ServerEntered);
    assert_eq!(
        screen.view(),
        LoginView::Discovering,
        "the next try is open"
    );
}

#[test]
fn opening_the_browser_waits_for_the_callback() {
    assert_eq!(
        waiting().view(),
        LoginView::WaitingForBrowser {
            homeserver: server(),
            problem: None
        }
    );
}

#[test]
fn the_browser_cannot_be_opened_before_a_server_is_chosen() {
    let mut screen = LoginScreen::new();
    screen.browser_opened();
    assert_eq!(screen.view(), LoginView::EnterServer { problem: None });
}

#[test]
fn a_refused_callback_keeps_waiting_and_says_why() {
    let mut screen = waiting();
    screen.callback_refused(CallbackError::WrongState);
    assert_eq!(
        screen.view(),
        LoginView::WaitingForBrowser {
            homeserver: server(),
            problem: Some(LoginProblem::BrowserCallback(CallbackError::WrongState))
        }
    );
    // FR-013: the real callback can still come.
    screen.apply(E::SsoCallback);
    assert_eq!(
        screen.view(),
        LoginView::SigningIn {
            homeserver: server()
        }
    );
}

#[test]
fn an_accepted_callback_exchanges_the_token() {
    let mut screen = waiting();
    screen.apply(E::SsoCallback);
    assert_eq!(
        screen.view(),
        LoginView::SigningIn {
            homeserver: server()
        }
    );
}

#[test]
fn a_login_that_fails_goes_back_to_the_choice_with_its_reason() {
    for problem in [
        LoginProblem::InvalidToken,
        LoginProblem::Unreachable,
        LoginProblem::Server,
    ] {
        let mut screen = waiting();
        screen.apply(E::SsoCallback);
        screen.login_failed(problem.clone());
        assert_eq!(
            screen.view(),
            LoginView::ChooseSignIn {
                homeserver: server(),
                problem: Some(problem)
            }
        );
    }
}

#[test]
fn a_new_attempt_clears_the_old_reason() {
    let mut screen = waiting();
    screen.apply(E::SsoCallback);
    screen.login_failed(LoginProblem::Unreachable);
    screen.browser_opened();
    assert_eq!(
        screen.view(),
        LoginView::WaitingForBrowser {
            homeserver: server(),
            problem: None
        }
    );
}

#[test]
fn cancelling_while_waiting_goes_back_to_the_server() {
    let mut screen = waiting();
    screen.cancel();
    assert_eq!(screen.view(), LoginView::EnterServer { problem: None });
}

#[test]
fn a_successful_login_shows_the_account() {
    assert_eq!(signed_in().view(), LoginView::SignedIn { user: alice() });
}

#[test]
fn a_stored_session_is_checked_before_it_is_trusted() {
    let mut screen = LoginScreen::new();
    screen.apply(E::Restore(session()));
    assert_eq!(
        screen.view(),
        LoginView::SigningIn {
            homeserver: server()
        }
    );
    screen.apply(E::LoginOk(session()));
    assert_eq!(screen.view(), LoginView::SignedIn { user: alice() });
}

#[test]
fn a_stored_session_the_server_refuses_asks_to_sign_in_again() {
    let mut screen = LoginScreen::new();
    screen.apply(E::Restore(session()));
    screen.login_failed(LoginProblem::InvalidToken);
    assert!(matches!(
        screen.view(),
        LoginView::ChooseSignIn {
            problem: Some(LoginProblem::InvalidToken),
            ..
        }
    ));
}

#[test]
fn a_401_names_the_account_and_offers_to_add_it_again() {
    let mut screen = signed_in();
    screen.apply(E::Unauthorized401);
    assert_eq!(screen.view(), LoginView::SessionExpired { user: alice() });
    screen.apply(E::ServerEntered);
    assert_eq!(screen.view(), LoginView::Discovering);
}

#[test]
fn signing_out_is_shown_while_it_runs_then_the_server_is_asked_again() {
    let mut screen = signed_in();
    screen.apply(E::SignOut);
    assert_eq!(screen.view(), LoginView::SigningOut { user: alice() });
    screen.apply(E::SignOutDone);
    assert_eq!(screen.view(), LoginView::EnterServer { problem: None });
}

// ------------------------------------------------------------ the erase fails (SignOutError::Wipe)

#[test]
fn a_sign_out_that_could_not_erase_the_data_is_shown_not_hidden() {
    let mut screen = signed_in();
    screen.apply(E::SignOut);
    screen.apply(E::SignOutFailed);
    assert_eq!(
        screen.view(),
        LoginView::SignOutFailed { user: alice() },
        "the screen must not go back to the server entry as if all were erased"
    );
}

#[test]
fn a_failed_erase_is_a_state_of_the_machine_and_keeps_the_session_it_names() {
    let mut screen = signed_in();
    screen.apply(E::SignOut);
    screen.apply(E::SignOutFailed);
    assert_eq!(
        screen.session_state(),
        &SessionState::SignOutFailed(session())
    );
}

#[test]
fn after_a_failed_erase_the_user_can_try_again_and_finish() {
    let mut screen = signed_in();
    screen.apply(E::SignOut);
    screen.apply(E::SignOutFailed);
    screen.apply(E::SignOut);
    assert_eq!(screen.view(), LoginView::SigningOut { user: alice() });
    screen.apply(E::SignOutDone);
    assert_eq!(screen.view(), LoginView::EnterServer { problem: None });
}

#[test]
fn a_failed_erase_keeps_the_device_because_its_keys_are_still_on_the_disk() {
    let mut screen = signed_in();
    screen.apply(E::SignOut);
    screen.apply(E::SignOutFailed);
    assert_eq!(
        screen.device_id().map(DeviceId::as_str),
        Some("DEVICE1"),
        "a completed sign-out forgets the device, a failed one does not"
    );
}

#[test]
fn nobody_can_sign_in_over_data_that_could_not_be_erased() {
    let mut screen = signed_in();
    screen.apply(E::SignOut);
    screen.apply(E::SignOutFailed);
    screen.apply(E::ServerEntered);
    assert_eq!(screen.view(), LoginView::SignOutFailed { user: alice() });
}

#[test]
fn the_failure_event_means_nothing_outside_a_sign_out() {
    let mut screen = signed_in();
    screen.apply(E::SignOutFailed);
    assert_eq!(screen.view(), LoginView::SignedIn { user: alice() });
}
