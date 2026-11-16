//! The session lifecycle machine, pure (FR-014, FR-015).
//!
//! Need: PRD FR-015 (sign out and session lifecycle machine; a session ended on the server
//! returns to sign-in: a 401 on sync), FR-014 (the device id of the previous session is
//! reused at the next sign-in so E2EE keys survive), FR-012 (SSO or password), FR-013
//! (nothing reaches Active but a successful login), D396 (a 401 forgets a session; "Add it
//! again" reuses the device), D72 (a stored session is checked before it is trusted).
//!
//! Public items used are listed in `docs/dev/signin-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_app::session::{SessionEvent as E, SessionMachine, SessionState as S};
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};

fn homeserver() -> HomeserverUrl {
    HomeserverUrl::parse("https://matrix.example.org").unwrap()
}

fn session(device: &str, token: &str) -> Session {
    Session {
        user_id: UserId::parse("@alice:example.org").unwrap(),
        device_id: DeviceId::parse(device).unwrap(),
        access_token: Secret::new(token),
        homeserver: homeserver(),
    }
}

fn device(s: &str) -> DeviceId {
    DeviceId::parse(s).unwrap()
}

fn run(events: Vec<E>) -> SessionMachine {
    let mut machine = SessionMachine::new();
    for event in events {
        machine.apply(event);
    }
    machine
}

/// SignedOut to AwaitingLogin on the homeserver.
fn up_to_awaiting() -> Vec<E> {
    vec![E::ServerEntered, E::HomeserverResolved(homeserver())]
}

fn signed_in(s: &Session) -> SessionMachine {
    let mut events = up_to_awaiting();
    events.extend([E::SsoCallback, E::LoginOk(s.clone())]);
    run(events)
}

// ---------------------------------------------------------------- the way in

#[test]
fn a_new_machine_is_signed_out_without_a_device() {
    let machine = SessionMachine::new();
    assert_eq!(machine.state(), &S::SignedOut);
    assert_eq!(machine.device_id(), None);
}

#[test]
fn entering_a_server_starts_the_discovery() {
    assert_eq!(run(vec![E::ServerEntered]).state(), &S::Discovering);
}

#[test]
fn a_resolved_homeserver_waits_for_the_login() {
    let machine = run(up_to_awaiting());
    assert_eq!(machine.state(), &S::AwaitingLogin(homeserver()));
}

#[test]
fn a_failed_discovery_goes_back_to_sign_in() {
    assert_eq!(
        run(vec![E::ServerEntered, E::DiscoveryFailed]).state(),
        &S::SignedOut
    );
}

#[test]
fn the_sso_callback_starts_the_login_on_the_same_homeserver() {
    let mut events = up_to_awaiting();
    events.push(E::SsoCallback);
    assert_eq!(run(events).state(), &S::LoggingIn(homeserver()));
}

#[test]
fn a_password_starts_the_login_too() {
    let mut events = up_to_awaiting();
    events.push(E::PasswordSubmitted);
    assert_eq!(run(events).state(), &S::LoggingIn(homeserver()));
}

#[test]
fn a_successful_login_makes_the_session_active() {
    let s = session("DEVICE1", "tok-1");
    assert_eq!(signed_in(&s).state(), &S::Active(s));
}

#[test]
fn a_failed_login_lets_the_user_try_again_on_the_same_server() {
    let mut events = up_to_awaiting();
    events.extend([E::SsoCallback, E::LoginFailed]);
    assert_eq!(run(events).state(), &S::AwaitingLogin(homeserver()));
}

#[test]
fn cancelling_goes_back_to_sign_in_from_every_step_before_active() {
    let steps = [
        vec![E::ServerEntered],
        up_to_awaiting(),
        [up_to_awaiting(), vec![E::PasswordSubmitted]].concat(),
    ];
    for mut events in steps {
        events.push(E::Cancelled);
        assert_eq!(run(events).state(), &S::SignedOut);
    }
}

#[test]
fn a_stored_session_is_checked_before_it_is_trusted() {
    let s = session("DEVICE1", "tok-1");
    let mut machine = SessionMachine::new();
    machine.apply(E::Restore(s.clone()));
    assert_eq!(machine.state(), &S::LoggingIn(homeserver()));
    assert_eq!(machine.device_id(), Some(&device("DEVICE1")));
    machine.apply(E::LoginOk(s.clone()));
    assert_eq!(machine.state(), &S::Active(s));
}

#[test]
fn a_stored_session_the_server_refuses_asks_for_a_new_sign_in() {
    let mut machine = run(vec![
        E::Restore(session("DEVICE1", "tok-1")),
        E::LoginFailed,
    ]);
    assert_eq!(machine.state(), &S::AwaitingLogin(homeserver()));
    machine.apply(E::Cancelled);
    assert_eq!(machine.state(), &S::SignedOut);
}

// ---------------------------------------------------------------- the session ends

#[test]
fn a_401_on_an_active_session_expires_it_and_keeps_it_for_the_message() {
    let s = session("DEVICE1", "tok-1");
    let mut machine = signed_in(&s);
    machine.apply(E::Unauthorized401);
    assert_eq!(machine.state(), &S::Expired(s));
}

#[test]
fn an_expired_session_signs_in_again_through_the_discovery() {
    let mut machine = signed_in(&session("DEVICE1", "tok-1"));
    machine.apply(E::Unauthorized401);
    machine.apply(E::ServerEntered);
    assert_eq!(machine.state(), &S::Discovering);
}

#[test]
fn signing_out_waits_for_the_server_then_forgets_the_session() {
    let s = session("DEVICE1", "tok-1");
    let mut machine = signed_in(&s);
    machine.apply(E::SignOut);
    assert_eq!(machine.state(), &S::SigningOut(s));
    machine.apply(E::SignOutDone);
    assert_eq!(machine.state(), &S::SignedOut);
}

#[test]
fn signing_out_an_expired_session_needs_no_server() {
    let mut machine = signed_in(&session("DEVICE1", "tok-1"));
    machine.apply(E::Unauthorized401);
    machine.apply(E::SignOut);
    assert_eq!(machine.state(), &S::SignedOut);
}

#[test]
fn a_401_while_signing_out_does_not_stop_the_sign_out() {
    let s = session("DEVICE1", "tok-1");
    let mut machine = signed_in(&s);
    machine.apply(E::SignOut);
    machine.apply(E::Unauthorized401);
    assert_eq!(machine.state(), &S::SigningOut(s));
}

#[test]
fn a_401_outside_an_active_session_changes_nothing() {
    let mut machine = SessionMachine::new();
    machine.apply(E::Unauthorized401);
    assert_eq!(machine.state(), &S::SignedOut);
    let mut machine = run(up_to_awaiting());
    machine.apply(E::Unauthorized401);
    assert_eq!(machine.state(), &S::AwaitingLogin(homeserver()));
}

// ---------------------------------------------------------------- the device survives

#[test]
fn the_device_of_the_active_session_is_remembered() {
    let machine = signed_in(&session("DEVICE1", "tok-1"));
    assert_eq!(machine.device_id(), Some(&device("DEVICE1")));
}

#[test]
fn the_device_survives_an_expired_session_and_the_next_login() {
    let mut machine = signed_in(&session("DEVICE1", "tok-1"));
    machine.apply(E::Unauthorized401);
    assert_eq!(machine.device_id(), Some(&device("DEVICE1")));
    machine.apply(E::ServerEntered);
    machine.apply(E::HomeserverResolved(homeserver()));
    machine.apply(E::SsoCallback);
    // the login asked for DEVICE1 (the adapter reads it); the server may answer another one
    assert_eq!(machine.device_id(), Some(&device("DEVICE1")));
    machine.apply(E::LoginOk(session("DEVICE2", "tok-2")));
    assert_eq!(machine.device_id(), Some(&device("DEVICE2")));
}

#[test]
fn a_sign_out_forgets_the_device_the_server_deleted() {
    let mut machine = signed_in(&session("DEVICE1", "tok-1"));
    machine.apply(E::SignOut);
    assert_eq!(
        machine.device_id(),
        Some(&device("DEVICE1")),
        "still known while the logout runs"
    );
    machine.apply(E::SignOutDone);
    assert_eq!(machine.state(), &S::SignedOut);
    assert_eq!(machine.device_id(), None);
}

#[test]
fn signing_out_an_expired_session_forgets_the_device_too() {
    let mut machine = signed_in(&session("DEVICE1", "tok-1"));
    machine.apply(E::Unauthorized401);
    machine.apply(E::SignOut);
    assert_eq!(machine.device_id(), None);
}

#[test]
fn the_next_login_after_a_sign_out_starts_without_a_device() {
    let mut machine = signed_in(&session("DEVICE1", "tok-1"));
    machine.apply(E::SignOut);
    machine.apply(E::SignOutDone);
    machine.apply(E::ServerEntered);
    machine.apply(E::HomeserverResolved(homeserver()));
    machine.apply(E::SsoCallback);
    assert_eq!(machine.device_id(), None);
    machine.apply(E::LoginOk(session("DEVICE2", "tok-2")));
    assert_eq!(machine.device_id(), Some(&device("DEVICE2")));
}

#[test]
fn a_login_that_fails_does_not_forget_the_device() {
    let mut machine = signed_in(&session("DEVICE1", "tok-1"));
    machine.apply(E::Unauthorized401);
    for e in [
        E::ServerEntered,
        E::HomeserverResolved(homeserver()),
        E::PasswordSubmitted,
        E::LoginFailed,
        E::Cancelled,
    ] {
        machine.apply(e);
    }
    assert_eq!(machine.state(), &S::SignedOut);
    assert_eq!(machine.device_id(), Some(&device("DEVICE1")));
}

// ---------------------------------------------------------------- events out of place

#[test]
fn an_event_out_of_place_is_ignored() {
    let s = session("DEVICE1", "tok-1");
    let cases: Vec<(Vec<E>, E)> = vec![
        (vec![], E::LoginOk(s.clone())),
        (vec![], E::SsoCallback),
        (vec![], E::HomeserverResolved(homeserver())),
        (vec![], E::SignOutDone),
        (vec![E::ServerEntered], E::LoginOk(s.clone())),
        (vec![E::ServerEntered], E::SsoCallback),
        (up_to_awaiting(), E::LoginOk(s.clone())),
        (up_to_awaiting(), E::SignOutDone),
        (up_to_awaiting(), E::Unauthorized401),
    ];
    for (setup, event) in cases {
        let mut machine = run(setup);
        let before = machine.state().clone();
        machine.apply(event.clone());
        assert_eq!(machine.state(), &before, "{event:?} after {before:?}");
    }
}

#[test]
fn a_login_answer_after_the_sign_out_started_does_not_bring_the_session_back() {
    let s = session("DEVICE1", "tok-1");
    let mut machine = signed_in(&s);
    machine.apply(E::SignOut);
    machine.apply(E::LoginOk(session("DEVICE9", "late")));
    assert_eq!(machine.state(), &S::SigningOut(s));
}

#[test]
fn a_second_login_answer_does_not_replace_an_active_session() {
    let s = session("DEVICE1", "tok-1");
    let mut machine = signed_in(&s);
    machine.apply(E::LoginOk(session("DEVICE9", "intruder")));
    assert_eq!(machine.state(), &S::Active(s));
    assert_eq!(machine.device_id(), Some(&device("DEVICE1")));
}

#[test]
fn an_expired_session_does_not_come_back_without_a_login() {
    let mut machine = signed_in(&session("DEVICE1", "tok-1"));
    machine.apply(E::Unauthorized401);
    let before = machine.state().clone();
    machine.apply(E::LoginOk(session("DEVICE9", "late")));
    assert_eq!(machine.state(), &before);
}

// ---------------------------------------------------------------- the property

fn any_event() -> impl Strategy<Value = E> {
    let sessions = prop_oneof![
        Just(session("DEVICE1", "tok-1")),
        Just(session("DEVICE2", "tok-2"))
    ];
    prop_oneof![
        Just(E::ServerEntered),
        Just(E::HomeserverResolved(homeserver())),
        Just(E::DiscoveryFailed),
        Just(E::SsoCallback),
        Just(E::PasswordSubmitted),
        Just(E::LoginFailed),
        Just(E::Cancelled),
        Just(E::Unauthorized401),
        Just(E::SignOut),
        Just(E::SignOutDone),
        sessions.clone().prop_map(E::LoginOk),
        sessions.prop_map(E::Restore),
    ]
}

fn is_active(state: &S) -> bool {
    matches!(state, S::Active(_))
}

proptest! {
    #[test]
    fn nothing_reaches_active_but_a_login_answer_while_logging_in(
        events in proptest::collection::vec(any_event(), 0..40)
    ) {
        let mut machine = SessionMachine::new();
        for event in events {
            let before = machine.state().clone();
            machine.apply(event.clone());
            let after = machine.state().clone();
            if is_active(&after) && !is_active(&before) {
                prop_assert!(matches!(before, S::LoggingIn(_)), "{before:?} -{event:?}-> {after:?}");
                prop_assert!(
                    matches!(&event, E::LoginOk(s) if after == S::Active(s.clone())),
                    "{before:?} -{event:?}-> {after:?}"
                );
            }
        }
    }

    #[test]
    fn an_active_session_leaves_only_by_a_401_or_a_sign_out(
        events in proptest::collection::vec(any_event(), 0..40)
    ) {
        let mut machine = SessionMachine::new();
        for event in events {
            let before = machine.state().clone();
            machine.apply(event.clone());
            if is_active(&before) && machine.state() != &before {
                prop_assert!(matches!(event, E::Unauthorized401 | E::SignOut), "{event:?}");
            }
        }
    }

    #[test]
    fn an_expired_session_comes_only_from_an_active_one(
        events in proptest::collection::vec(any_event(), 0..40)
    ) {
        let mut machine = SessionMachine::new();
        for event in events {
            let before = machine.state().clone();
            machine.apply(event);
            if matches!(machine.state(), S::Expired(_)) {
                prop_assert!(matches!(before, S::Active(_) | S::Expired(_)), "{before:?}");
            }
        }
    }

    #[test]
    fn the_device_is_forgotten_only_by_a_completed_sign_out(
        events in proptest::collection::vec(any_event(), 0..40)
    ) {
        let mut machine = SessionMachine::new();
        for event in events {
            let had = machine.device_id().cloned();
            machine.apply(event.clone());
            if had.is_some() && machine.device_id().is_none() {
                prop_assert!(matches!(event, E::SignOut | E::SignOutDone), "{event:?}");
                prop_assert!(matches!(machine.state(), S::SignedOut), "{:?}", machine.state());
            }
        }
    }
}
