//! What the sign-in screen shows, as a view model: texts and buttons for each `LoginView`, no window
//! (A033 "Not done": no UI for the login screen or the unavailable store; item 4 of the lot).
//!
//! Needs: FR-012, FR-013, FR-015, D396 (a notice of an account the server ended has an "Add it
//! again" button), A005 (an unreadable keychain is told, never taken for an empty one), A033 item
//! 4 (a failed erase is shown, not hidden). Every failure has a way out: a screen that waits for
//! nothing offers a button; a screen that waits says so (`busy`). Texts come from the catalogue
//! (en and fr), never a raw key; the texts name the server or the account where they speak of it.
//!
//! Expected public surface of `twake_ui::login_screen` (new):
//! ```text
//! #[derive(Debug, Clone, Copy, PartialEq, Eq)]
//! enum LoginAction { SubmitServer, OpenBrowser, Cancel, SignOut, AddAgain, RetryStart }
//! struct LoginButton { pub label: String, pub action: LoginAction }
//! struct LoginScreenView {
//!     pub title: String,
//!     pub message: Option<String>,       // what the screen says (names the server or the account)
//!     pub problem: Option<String>,       // why the last attempt failed
//!     pub buttons: Vec<LoginButton>,     // in the order shown, the first being the main one
//!     pub busy: bool,                    // something runs: a spinner, no way to hurry it
//! }
//! LoginScreenView::from(view: &LoginView, locale: &Locale) -> LoginScreenView
//! ```
//! The catalogues `en.ftl` and `fr.ftl` of `twake-i18n` get the keys (the specs do not name them).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use twake_app::login_screen::{LoginProblem, LoginView};
use twake_app::sso::CallbackError;
use twake_domain::UserId;
use twake_domain::session::HomeserverUrl;
use twake_i18n::Locale;
use twake_ui::login_screen::{LoginAction, LoginScreenView};

fn server() -> HomeserverUrl {
    HomeserverUrl::parse("https://matrix.example.org").unwrap()
}

fn alice() -> UserId {
    UserId::parse("@alice:example.org").unwrap()
}

fn en() -> Locale {
    Locale::bundled("en", 0)
}

fn fr() -> Locale {
    Locale::bundled("fr", 0)
}

fn show(view: &LoginView) -> LoginScreenView {
    LoginScreenView::from(view, &en())
}

fn actions(view: &LoginScreenView) -> Vec<LoginAction> {
    view.buttons.iter().map(|b| b.action).collect()
}

fn every_view() -> Vec<LoginView> {
    let user = alice();
    let homeserver = server();
    vec![
        LoginView::EnterServer { problem: None },
        LoginView::EnterServer {
            problem: Some(LoginProblem::ServerNotFound),
        },
        LoginView::Discovering,
        LoginView::ChooseSignIn {
            homeserver: homeserver.clone(),
            problem: None,
        },
        LoginView::WaitingForBrowser {
            homeserver: homeserver.clone(),
            problem: None,
        },
        LoginView::SigningIn { homeserver },
        LoginView::SignedIn { user: user.clone() },
        LoginView::SessionExpired { user: user.clone() },
        LoginView::SigningOut { user: user.clone() },
        LoginView::SignOutFailed { user },
        LoginView::StoreUnavailable,
    ]
}

/// A missing key of the catalogue comes back as the key itself: `login-title`.
fn is_a_raw_key(text: &str) -> bool {
    text.contains('-')
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
}

fn texts(view: &LoginScreenView) -> Vec<&str> {
    let mut all = vec![view.title.as_str()];
    all.extend(view.message.as_deref());
    all.extend(view.problem.as_deref());
    all.extend(view.buttons.iter().map(|b| b.label.as_str()));
    all
}

#[test]
fn every_screen_has_a_title_and_texts_from_the_catalogue_in_both_languages() {
    for locale in [en(), fr()] {
        for view in every_view() {
            let shown = LoginScreenView::from(&view, &locale);
            assert!(!shown.title.trim().is_empty(), "{view:?} has no title");
            for button in &shown.buttons {
                assert!(
                    !button.label.contains('\n'),
                    "{view:?}: a label on two lines"
                );
            }
            for text in texts(&shown) {
                assert!(!text.trim().is_empty(), "{view:?}: an empty text");
                assert!(!is_a_raw_key(text), "{view:?}: `{text}` is a missing key");
            }
        }
    }
}

#[test]
fn the_french_screen_is_not_the_english_one() {
    for view in every_view() {
        assert_ne!(
            show(&view).title,
            LoginScreenView::from(&view, &fr()).title,
            "{view:?}"
        );
    }
}

#[test]
fn a_screen_that_waits_for_nothing_has_a_button_and_one_that_works_says_so() {
    for view in every_view() {
        let shown = show(&view);
        if !shown.busy {
            assert!(
                !shown.buttons.is_empty(),
                "{view:?} leaves the user with no way out"
            );
        }
    }
    let busy: Vec<bool> = [
        LoginView::Discovering,
        LoginView::WaitingForBrowser {
            homeserver: server(),
            problem: None,
        },
        LoginView::SigningIn {
            homeserver: server(),
        },
        LoginView::SigningOut { user: alice() },
    ]
    .iter()
    .map(|v| show(v).busy)
    .collect();
    assert_eq!(busy, [true; 4]);
}

#[test]
fn the_server_is_asked_for_and_a_lookup_that_failed_says_so() {
    let first = show(&LoginView::EnterServer { problem: None });
    assert_eq!(actions(&first), [LoginAction::SubmitServer]);
    assert_eq!(first.problem, None);

    let failed = show(&LoginView::EnterServer {
        problem: Some(LoginProblem::ServerNotFound),
    });
    assert!(failed.problem.is_some());
    assert!(
        actions(&failed).contains(&LoginAction::SubmitServer),
        "asks again"
    );
}

#[test]
fn a_lookup_in_progress_can_be_given_up() {
    assert!(actions(&show(&LoginView::Discovering)).contains(&LoginAction::Cancel));
}

#[test]
fn the_choice_names_the_server_and_offers_the_browser_or_giving_up() {
    let shown = show(&LoginView::ChooseSignIn {
        homeserver: server(),
        problem: None,
    });
    assert!(
        texts(&shown)
            .iter()
            .any(|t| t.contains("matrix.example.org"))
    );
    assert_eq!(
        actions(&shown)[0],
        LoginAction::OpenBrowser,
        "the main button"
    );
    assert!(actions(&shown).contains(&LoginAction::Cancel));
}

#[test]
fn a_failed_login_shows_why_and_still_offers_the_browser() {
    let shown = show(&LoginView::ChooseSignIn {
        homeserver: server(),
        problem: Some(LoginProblem::InvalidToken),
    });
    assert!(shown.problem.is_some());
    assert!(actions(&shown).contains(&LoginAction::OpenBrowser));
}

#[test]
fn while_the_browser_is_open_the_user_can_give_up_and_a_refused_callback_is_explained() {
    let waiting = show(&LoginView::WaitingForBrowser {
        homeserver: server(),
        problem: None,
    });
    assert!(actions(&waiting).contains(&LoginAction::Cancel));
    assert_eq!(waiting.problem, None);

    let refused = show(&LoginView::WaitingForBrowser {
        homeserver: server(),
        problem: Some(LoginProblem::BrowserCallback(CallbackError::WrongState)),
    });
    assert!(refused.problem.is_some());
    assert!(actions(&refused).contains(&LoginAction::Cancel));
}

#[test]
fn the_token_exchange_offers_no_second_attempt() {
    let shown = show(&LoginView::SigningIn {
        homeserver: server(),
    });
    for action in [LoginAction::OpenBrowser, LoginAction::SubmitServer] {
        assert!(!actions(&shown).contains(&action));
    }
}

#[test]
fn a_signed_in_account_is_named_and_can_sign_out() {
    let shown = show(&LoginView::SignedIn { user: alice() });
    assert!(
        texts(&shown)
            .iter()
            .any(|t| t.contains("@alice:example.org"))
    );
    assert!(actions(&shown).contains(&LoginAction::SignOut));
}

#[test]
fn an_ended_session_names_the_account_and_offers_to_add_it_again() {
    let shown = show(&LoginView::SessionExpired { user: alice() });
    assert!(
        texts(&shown)
            .iter()
            .any(|t| t.contains("@alice:example.org"))
    );
    assert_eq!(actions(&shown)[0], LoginAction::AddAgain, "D396");
}

#[test]
fn a_sign_out_that_runs_cannot_be_hurried() {
    let shown = show(&LoginView::SigningOut { user: alice() });
    assert!(shown.busy);
    assert!(shown.buttons.is_empty());
}

#[test]
fn a_failed_erase_is_visible_names_the_account_and_offers_to_try_again() {
    let shown = show(&LoginView::SignOutFailed { user: alice() });
    assert!(
        texts(&shown)
            .iter()
            .any(|t| t.contains("@alice:example.org"))
    );
    assert!(
        shown.problem.is_some(),
        "the failure is a problem, not a quiet state"
    );
    assert_eq!(actions(&shown), [LoginAction::SignOut]);
}

#[test]
fn an_unreadable_keychain_says_so_and_offers_to_try_again_not_a_sign_in() {
    let shown = show(&LoginView::StoreUnavailable);
    assert!(shown.problem.is_some() || shown.message.is_some());
    assert_eq!(actions(&shown), [LoginAction::RetryStart]);
}

#[test]
fn each_reason_has_its_own_text_and_none_shows_a_token() {
    let reasons = [
        LoginProblem::ServerNotFound,
        LoginProblem::Unreachable,
        LoginProblem::Server,
        LoginProblem::InvalidToken,
        LoginProblem::BrowserCallback(CallbackError::WrongState),
        LoginProblem::BrowserCallback(CallbackError::MissingToken),
        LoginProblem::BrowserCallback(CallbackError::WrongTarget),
    ];
    let mut seen = std::collections::HashSet::new();
    for reason in reasons {
        let shown = show(&LoginView::ChooseSignIn {
            homeserver: server(),
            problem: Some(reason.clone()),
        });
        let text = shown.problem.expect("a reason is shown");
        assert!(
            !text.trim().is_empty() && !is_a_raw_key(&text),
            "{reason:?}"
        );
        assert!(seen.insert(text), "{reason:?} says what another one says");
    }
}
