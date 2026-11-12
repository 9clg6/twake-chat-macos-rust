//! The session of one account: its secret never leaks, its device id is checked, and the
//! rule that says whether a homeserver answers for a user id (FR-014, FR-013, D395).
//!
//! Need: PRD FR-014 (session = access token, user id, device id), NFR-015 and NFR-019
//! (a token never shows in a log), the React `servesUser` (`discoverHomeserver.ts`: the server
//! name of the user id is the host of the homeserver, or both are this machine; otherwise the
//! caller reads the well-known of that server name and compares), Matrix spec "Common
//! identifier format" for the device id (an opaque string).
//!
//! Public items used are listed in `docs/dev/signin-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};
use twake_domain::{IdError, UserId};

const TOKEN: &str = "syt_YWxpY2U_xyzzy7391_0aB9";

fn user(s: &str) -> UserId {
    UserId::parse(s).unwrap()
}

fn homeserver(s: &str) -> HomeserverUrl {
    HomeserverUrl::parse(s).unwrap()
}

fn session() -> Session {
    Session {
        user_id: user("@alice:example.org"),
        device_id: DeviceId::parse("DEVICE7391").unwrap(),
        access_token: Secret::new(TOKEN),
        homeserver: homeserver("https://matrix.example.org"),
    }
}

// ---------------------------------------------------------------- Secret

#[test]
fn a_secret_gives_its_text_to_whoever_asks_for_it_by_name() {
    assert_eq!(Secret::new(TOKEN).expose(), TOKEN);
    assert_eq!(Secret::new(String::from("other")).expose(), "other");
}

#[test]
fn a_secret_never_shows_in_debug_or_display() {
    let secret = Secret::new(TOKEN);
    for shown in [
        format!("{secret:?}"),
        format!("{secret:#?}"),
        format!("{secret}"),
        secret.to_string(),
    ] {
        assert!(!shown.is_empty());
        assert!(!shown.contains(TOKEN), "{shown}");
        assert!(!shown.contains("xyzzy"), "{shown}");
    }
}

#[test]
fn two_secrets_are_equal_when_their_text_is() {
    assert_eq!(Secret::new("a-1"), Secret::new("a-1"));
    assert_ne!(Secret::new("a-1"), Secret::new("a-2"));
    assert_eq!(Secret::new("a-1").clone(), Secret::new("a-1"));
}

// ---------------------------------------------------------------- DeviceId

#[test]
fn a_device_id_keeps_its_text() {
    let id = DeviceId::parse("ABCDEFGHIJ").unwrap();
    assert_eq!(id.as_str(), "ABCDEFGHIJ");
    assert_eq!(id.to_string(), "ABCDEFGHIJ");
}

#[test]
fn a_device_id_is_not_empty_nor_blank() {
    assert_eq!(DeviceId::parse(""), Err(IdError::Empty));
    assert_eq!(DeviceId::parse("AB CD"), Err(IdError::InvalidCharacter));
    assert_eq!(DeviceId::parse("AB\nCD"), Err(IdError::InvalidCharacter));
    assert_eq!(DeviceId::parse("AB\0CD"), Err(IdError::InvalidCharacter));
}

#[test]
fn two_device_ids_differ_when_their_text_does() {
    assert_eq!(
        DeviceId::parse("AAAA").unwrap(),
        DeviceId::parse("AAAA").unwrap()
    );
    assert_ne!(
        DeviceId::parse("AAAA").unwrap(),
        DeviceId::parse("BBBB").unwrap()
    );
}

// ---------------------------------------------------------------- Session

#[test]
fn a_session_never_shows_its_token_in_debug() {
    let s = session();
    for shown in [
        format!("{s:?}"),
        format!("{s:#?}"),
        format!("{:?}", Some(&s)),
        format!("{:?}", vec![s.clone()]),
    ] {
        assert!(!shown.contains(TOKEN), "{shown}");
        assert!(!shown.contains("xyzzy"), "{shown}");
    }
}

#[test]
fn a_session_equals_its_clone_and_differs_by_any_field() {
    let base = session();
    assert_eq!(base, base.clone());
    let mut other = base.clone();
    other.access_token = Secret::new("syt_other");
    assert_ne!(base, other);
    let mut other = base.clone();
    other.device_id = DeviceId::parse("OTHERDEVICE").unwrap();
    assert_ne!(base, other);
    let mut other = base.clone();
    other.user_id = user("@bob:example.org");
    assert_ne!(base, other);
    let mut other = base.clone();
    other.homeserver = homeserver("https://other.example.org");
    assert_ne!(base, other);
}

// ---------------------------------------------------------------- serves_directly

#[test]
fn a_homeserver_answers_for_the_users_of_its_own_host() {
    assert!(homeserver("https://example.org").serves_directly(&user("@alice:example.org")));
    assert!(
        homeserver("https://example.org:8448").serves_directly(&user("@alice:example.org:8448"))
    );
}

#[test]
fn a_port_makes_another_server() {
    assert!(!homeserver("https://example.org:8448").serves_directly(&user("@alice:example.org")));
    assert!(!homeserver("https://example.org").serves_directly(&user("@alice:example.org:8448")));
}

#[test]
fn a_user_of_another_host_is_not_served_directly() {
    // The server name of the user may still point here through its well-known: the caller
    // reads that well-known and compares the two addresses, `serves_directly` does not.
    assert!(!homeserver("https://matrix.example.org").serves_directly(&user("@alice:example.org")));
    assert!(!homeserver("https://example.org").serves_directly(&user("@alice:evil.org")));
}

#[test]
fn a_host_that_only_ends_or_starts_like_the_server_name_is_another_server() {
    assert!(
        !homeserver("https://example.org.evil.org").serves_directly(&user("@alice:example.org"))
    );
    assert!(!homeserver("https://evilexample.org").serves_directly(&user("@alice:example.org")));
    assert!(!homeserver("https://example.org").serves_directly(&user("@alice:evilexample.org")));
}

#[test]
fn a_stack_on_this_machine_names_its_server_as_it_likes() {
    assert!(homeserver("http://localhost:8008").serves_directly(&user("@alice:localhost:8008")));
    assert!(homeserver("http://localhost:8008").serves_directly(&user("@alice:127.0.0.1:8008")));
    assert!(homeserver("http://127.0.0.1:8008").serves_directly(&user("@alice:localhost")));
}

#[test]
fn this_machine_never_answers_for_a_real_server_nor_the_other_way_round() {
    assert!(!homeserver("http://localhost:8008").serves_directly(&user("@alice:example.org")));
    assert!(!homeserver("https://example.org").serves_directly(&user("@alice:localhost:8008")));
}
