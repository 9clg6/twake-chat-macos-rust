//! The SSO request the app starts, and the callback it accepts (FR-012, FR-013).
//!
//! Need: Matrix Client-Server spec, "Login > SSO client login/authentication"
//! (`GET /_matrix/client/v3/login/sso/redirect[/{idpId}]?redirectUrl=...`; the homeserver
//! sends the browser back to `redirectUrl` with a `loginToken` query parameter added);
//! ARCHITECTURE.md section 7 (loopback redirect on `127.0.0.1:<random port>`); PRD FR-013
//! (a callback is accepted only if it answers a request this app started: `state` check);
//! D217 (state kept per attempt, the token exchanged only with the same state, spent once).
//! The randomness is injected so the state is deterministic here.
//!
//! Public items used are listed in `docs/dev/signin-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use twake_app::sso::{CallbackError, SsoError, SsoRequest};
use twake_domain::session::{HomeserverUrl, IdentityProvider};
use url::Url;

const RANDOM: [u8; 16] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
];
const STATE: &str = "000102030405060708090a0b0c0d0e0f";
const REDIRECT: &str = "http://127.0.0.1:53682/callback";
const TOKEN: &str = "syt_bG9naW4_Zq81x_0Ab1";

fn homeserver() -> HomeserverUrl {
    HomeserverUrl::parse("https://matrix.example.org").unwrap()
}

fn request() -> SsoRequest {
    SsoRequest::new(&homeserver(), None, REDIRECT, RANDOM).unwrap()
}

fn callback(state: &str, token: &str) -> String {
    format!("{REDIRECT}?state={state}&loginToken={token}")
}

fn query(url: &str) -> Vec<(String, String)> {
    Url::parse(url)
        .unwrap()
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

fn twake() -> IdentityProvider {
    IdentityProvider {
        id: "oidc-twake".into(),
        name: "Twake".into(),
    }
}

// ---------------------------------------------------------------- the URL we open

#[test]
fn the_state_is_the_hex_of_the_injected_bytes() {
    assert_eq!(request().state(), STATE);
    let other = SsoRequest::new(&homeserver(), None, REDIRECT, [0xff; 16]).unwrap();
    assert_eq!(other.state(), "ffffffffffffffffffffffffffffffff");
}

#[test]
fn the_url_goes_to_the_sso_redirect_of_the_homeserver() {
    let url = Url::parse(request().url()).unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("matrix.example.org"));
    assert_eq!(url.path(), "/_matrix/client/v3/login/sso/redirect");
}

#[test]
fn the_url_asks_to_come_back_to_the_loopback_with_the_state() {
    let pairs = query(request().url());
    assert_eq!(pairs.len(), 1, "only redirectUrl: {pairs:?}");
    assert_eq!(pairs[0].0, "redirectUrl");
    assert_eq!(pairs[0].1, format!("{REDIRECT}?state={STATE}"));
}

#[test]
fn a_chosen_identity_provider_ends_the_path() {
    let request = SsoRequest::new(&homeserver(), Some(&twake()), REDIRECT, RANDOM).unwrap();
    let url = Url::parse(request.url()).unwrap();
    assert_eq!(
        url.path(),
        "/_matrix/client/v3/login/sso/redirect/oidc-twake"
    );
}

#[test]
fn a_path_prefix_of_the_homeserver_is_kept() {
    let hs = HomeserverUrl::parse("https://example.org:8448/matrix/").unwrap();
    let request = SsoRequest::new(&hs, None, REDIRECT, RANDOM).unwrap();
    let url = Url::parse(request.url()).unwrap();
    assert_eq!(url.port(), Some(8448));
    assert_eq!(url.path(), "/matrix/_matrix/client/v3/login/sso/redirect");
}

#[test]
fn another_draw_makes_another_state_and_another_url() {
    let other = SsoRequest::new(&homeserver(), None, REDIRECT, [7; 16]).unwrap();
    assert_ne!(other.state(), request().state());
    assert_ne!(other.url(), request().url());
}

// ---------------------------------------------------------------- the redirect we accept to build

#[test]
fn the_redirect_must_be_http_on_the_loopback_address_with_a_port() {
    for bad in [
        "https://127.0.0.1:53682/callback",
        "http://localhost:53682/callback",
        "http://example.org:53682/callback",
        "http://127.0.0.1/callback",
        "http://127.0.0.1.evil.org:53682/callback",
        "twake://callback",
        "not a url",
        "",
    ] {
        assert_eq!(
            SsoRequest::new(&homeserver(), None, bad, RANDOM).err(),
            Some(SsoError::InvalidRedirect),
            "{bad}"
        );
    }
}

#[test]
fn the_redirect_carries_no_credentials_query_or_fragment() {
    for bad in [
        "http://user@127.0.0.1:53682/callback",
        "http://127.0.0.1:53682/callback?state=mine",
        "http://127.0.0.1:53682/callback?x=1",
        "http://127.0.0.1:53682/callback#frag",
    ] {
        assert_eq!(
            SsoRequest::new(&homeserver(), None, bad, RANDOM).err(),
            Some(SsoError::InvalidRedirect),
            "{bad}"
        );
    }
}

#[test]
fn every_sso_error_says_something() {
    assert!(!SsoError::InvalidRedirect.to_string().is_empty());
}

// ---------------------------------------------------------------- the callback we accept

#[test]
fn the_callback_of_our_request_gives_the_login_token() {
    let mut request = request();
    let token = request.accept_callback(&callback(STATE, TOKEN)).unwrap();
    assert_eq!(token.expose(), TOKEN);
}

#[test]
fn the_token_is_decoded_and_the_order_of_the_parameters_does_not_matter() {
    let mut request = request();
    let url = format!("{REDIRECT}?loginToken=syt_a%2Fb%2Bc%3D&state={STATE}&extra=1");
    assert_eq!(
        request.accept_callback(&url).unwrap().expose(),
        "syt_a/b+c="
    );
}

#[test]
fn the_token_does_not_show_in_the_debug_of_the_result() {
    let mut request = request();
    let token = request.accept_callback(&callback(STATE, TOKEN)).unwrap();
    assert!(!format!("{token:?}").contains(TOKEN));
}

#[test]
fn a_callback_without_a_state_is_refused() {
    assert_eq!(
        request().accept_callback(&format!("{REDIRECT}?loginToken={TOKEN}")),
        Err(CallbackError::MissingState)
    );
    assert_eq!(
        request().accept_callback(REDIRECT),
        Err(CallbackError::MissingState)
    );
}

#[test]
fn a_callback_with_another_state_is_refused() {
    let mut request = request();
    for wrong in [
        "",
        "x",
        "ffffffffffffffffffffffffffffffff",
        "000102030405060708090a0b0c0d0e0e",
        &STATE.to_uppercase(),
    ] {
        assert_eq!(
            request.accept_callback(&callback(wrong, TOKEN)),
            Err(CallbackError::WrongState),
            "{wrong:?}"
        );
    }
}

#[test]
fn a_state_error_wins_over_a_missing_token() {
    let mut request = request();
    assert_eq!(
        request.accept_callback(&format!("{REDIRECT}?state=bad")),
        Err(CallbackError::WrongState)
    );
    assert_eq!(
        request.accept_callback(REDIRECT),
        Err(CallbackError::MissingState)
    );
}

#[test]
fn a_refused_callback_does_not_spend_the_request() {
    let mut request = request();
    assert!(request.accept_callback(&callback("bad", "stolen")).is_err());
    assert_eq!(
        request
            .accept_callback(&callback(STATE, TOKEN))
            .unwrap()
            .expose(),
        TOKEN
    );
}

#[test]
fn a_callback_is_accepted_once() {
    let mut request = request();
    request.accept_callback(&callback(STATE, TOKEN)).unwrap();
    assert_eq!(
        request.accept_callback(&callback(STATE, TOKEN)),
        Err(CallbackError::Replayed)
    );
    assert_eq!(
        request.accept_callback(&callback(STATE, "another")),
        Err(CallbackError::Replayed)
    );
}

#[test]
fn a_callback_for_another_host_port_scheme_or_path_is_refused() {
    let mut request = request();
    for other in [
        format!("http://127.0.0.1:53683/callback?state={STATE}&loginToken={TOKEN}"),
        format!("http://127.0.0.2:53682/callback?state={STATE}&loginToken={TOKEN}"),
        format!("http://localhost:53682/callback?state={STATE}&loginToken={TOKEN}"),
        format!("https://127.0.0.1:53682/callback?state={STATE}&loginToken={TOKEN}"),
        format!("http://127.0.0.1:53682/other?state={STATE}&loginToken={TOKEN}"),
        format!("http://127.0.0.1:53682/callback/x?state={STATE}&loginToken={TOKEN}"),
        format!("http://127.0.0.1:53682/?state={STATE}&loginToken={TOKEN}"),
        format!("http://evil.org/callback?state={STATE}&loginToken={TOKEN}"),
        format!("http://user@127.0.0.1:53682/callback?state={STATE}&loginToken={TOKEN}"),
    ] {
        assert_eq!(
            request.accept_callback(&other),
            Err(CallbackError::WrongTarget),
            "{other}"
        );
    }
    // none of them spent the request
    assert!(request.accept_callback(&callback(STATE, TOKEN)).is_ok());
}

#[test]
fn a_callback_that_is_not_a_url_is_refused() {
    let mut request = request();
    for bad in ["", "not a url", "/callback?state=x"] {
        assert_eq!(
            request.accept_callback(bad),
            Err(CallbackError::Malformed),
            "{bad:?}"
        );
    }
}

#[test]
fn a_missing_or_empty_login_token_is_refused() {
    let mut request = request();
    assert_eq!(
        request.accept_callback(&format!("{REDIRECT}?state={STATE}")),
        Err(CallbackError::MissingToken)
    );
    assert_eq!(
        request.accept_callback(&callback(STATE, "")),
        Err(CallbackError::MissingToken)
    );
    // the request is still ours to answer
    assert!(request.accept_callback(&callback(STATE, TOKEN)).is_ok());
}

#[test]
fn a_repeated_state_or_token_is_refused_even_when_one_copy_is_right() {
    let mut request = request();
    let twice_state = format!("{REDIRECT}?state=bad&state={STATE}&loginToken={TOKEN}");
    let twice_state_ok = format!("{REDIRECT}?state={STATE}&state={STATE}&loginToken={TOKEN}");
    let twice_token = format!("{REDIRECT}?state={STATE}&loginToken={TOKEN}&loginToken=stolen");
    for dup in [twice_state, twice_state_ok, twice_token] {
        assert_eq!(
            request.accept_callback(&dup),
            Err(CallbackError::DuplicateParameter),
            "{dup}"
        );
    }
    assert!(request.accept_callback(&callback(STATE, TOKEN)).is_ok());
}

#[test]
fn another_parameter_may_repeat() {
    let mut request = request();
    let url = format!("{REDIRECT}?a=1&a=2&state={STATE}&loginToken={TOKEN}");
    assert!(request.accept_callback(&url).is_ok());
}

#[test]
fn a_token_for_another_request_does_not_open_this_one() {
    let mut mine = request();
    let theirs = SsoRequest::new(&homeserver(), None, REDIRECT, [9; 16]).unwrap();
    assert_eq!(
        mine.accept_callback(&callback(theirs.state(), TOKEN)),
        Err(CallbackError::WrongState)
    );
}

#[test]
fn every_callback_error_says_something_different() {
    let all = [
        CallbackError::Malformed,
        CallbackError::WrongTarget,
        CallbackError::MissingState,
        CallbackError::WrongState,
        CallbackError::Replayed,
        CallbackError::MissingToken,
        CallbackError::DuplicateParameter,
    ];
    let texts: Vec<String> = all.iter().map(ToString::to_string).collect();
    for (i, a) in texts.iter().enumerate() {
        assert!(!a.is_empty());
        assert!(
            texts.iter().skip(i + 1).all(|b| a != b),
            "duplicate text {a}"
        );
    }
}
