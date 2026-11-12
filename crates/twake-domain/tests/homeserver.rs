//! Homeserver address, `.well-known` discovery and the `/login` flows (FR-012).
//!
//! Need: Matrix Client-Server spec, "Server Discovery" (`/.well-known/matrix/client`,
//! `m.homeserver.base_url`) and "Login" (`GET /_matrix/client/v3/login`: `m.login.sso` with
//! `identity_providers`, `m.login.password`); ARCHITECTURE.md section 9 (HTTPS only, no
//! allow-list of hosts, D373); the React `discoverLogin` (https, or loopback for a dev stack).
//!
//! Public items used are listed in `docs/dev/signin-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use twake_domain::session::{
    HomeserverUrl, HomeserverUrlError, IdentityProvider, LoginFlows, LoginFlowsError,
    WellKnownError, parse_well_known,
};

fn url(s: &str) -> HomeserverUrl {
    HomeserverUrl::parse(s).unwrap()
}

fn well_known(base_url: &str) -> String {
    format!(r#"{{"m.homeserver":{{"base_url":"{base_url}"}}}}"#)
}

// ---------------------------------------------------------------- HomeserverUrl

#[test]
fn an_https_address_is_kept_as_written() {
    assert_eq!(
        url("https://matrix.example.org").as_str(),
        "https://matrix.example.org"
    );
}

#[test]
fn a_trailing_slash_does_not_make_another_homeserver() {
    assert_eq!(
        url("https://matrix.example.org/"),
        url("https://matrix.example.org")
    );
    assert_eq!(
        url("https://matrix.example.org///").as_str(),
        "https://matrix.example.org"
    );
}

#[test]
fn the_host_is_lowercased_and_the_default_port_dropped() {
    assert_eq!(
        url("HTTPS://Matrix.Example.ORG:443/").as_str(),
        "https://matrix.example.org"
    );
}

#[test]
fn a_port_and_a_path_prefix_are_kept() {
    assert_eq!(
        url("https://example.org:8448/matrix/").as_str(),
        "https://example.org:8448/matrix"
    );
}

#[test]
fn spaces_around_the_address_are_ignored() {
    assert_eq!(
        url("  https://example.org \n").as_str(),
        "https://example.org"
    );
}

#[test]
fn plain_http_is_refused_for_a_real_server() {
    assert_eq!(
        HomeserverUrl::parse("http://matrix.example.org"),
        Err(HomeserverUrlError::NotHttps)
    );
    assert_eq!(
        HomeserverUrl::parse("http://192.168.1.20:8008"),
        Err(HomeserverUrlError::NotHttps)
    );
}

#[test]
fn plain_http_is_allowed_on_this_machine_only() {
    assert_eq!(
        url("http://localhost:8008").as_str(),
        "http://localhost:8008"
    );
    assert_eq!(
        url("http://127.0.0.1:8008").as_str(),
        "http://127.0.0.1:8008"
    );
    assert_eq!(url("http://[::1]:8008").as_str(), "http://[::1]:8008");
}

#[test]
fn a_host_that_only_looks_like_this_machine_is_not() {
    for fake in [
        "http://localhost.evil.org",
        "http://127.0.0.1.evil.org:8008",
        "http://notlocalhost:8008",
        "http://evil.org/localhost",
    ] {
        assert_eq!(
            HomeserverUrl::parse(fake),
            Err(HomeserverUrlError::NotHttps),
            "{fake}"
        );
    }
}

#[test]
fn other_schemes_are_refused() {
    for other in [
        "ftp://example.org",
        "ws://localhost:8008",
        "javascript:alert(1)",
        "localhost:8008",
    ] {
        assert!(HomeserverUrl::parse(other).is_err(), "{other}");
    }
}

#[test]
fn an_address_without_a_scheme_or_empty_is_refused() {
    assert_eq!(
        HomeserverUrl::parse("matrix.example.org"),
        Err(HomeserverUrlError::Invalid)
    );
    assert_eq!(HomeserverUrl::parse(""), Err(HomeserverUrlError::Empty));
    assert_eq!(HomeserverUrl::parse("  \t"), Err(HomeserverUrlError::Empty));
}

#[test]
fn credentials_in_the_address_are_refused() {
    assert_eq!(
        HomeserverUrl::parse("https://alice:secret@example.org"),
        Err(HomeserverUrlError::HasUserinfo)
    );
    assert_eq!(
        HomeserverUrl::parse("https://alice@example.org"),
        Err(HomeserverUrlError::HasUserinfo)
    );
}

#[test]
fn a_query_or_a_fragment_is_refused_even_when_empty() {
    assert_eq!(
        HomeserverUrl::parse("https://example.org/?a=b"),
        Err(HomeserverUrlError::HasQuery)
    );
    assert_eq!(
        HomeserverUrl::parse("https://example.org?"),
        Err(HomeserverUrlError::HasQuery)
    );
    assert_eq!(
        HomeserverUrl::parse("https://example.org/#top"),
        Err(HomeserverUrlError::HasFragment)
    );
    assert_eq!(
        HomeserverUrl::parse("https://example.org#"),
        Err(HomeserverUrlError::HasFragment)
    );
}

#[test]
fn an_api_url_is_the_address_followed_by_the_path() {
    assert_eq!(
        url("https://example.org").api_url("/_matrix/client/v3/login"),
        "https://example.org/_matrix/client/v3/login"
    );
    assert_eq!(
        url("https://example.org:8448/matrix/").api_url("/_matrix/client/v3/login"),
        "https://example.org:8448/matrix/_matrix/client/v3/login"
    );
}

#[test]
fn every_address_error_says_something_different() {
    let all = [
        HomeserverUrlError::Empty,
        HomeserverUrlError::Invalid,
        HomeserverUrlError::NotHttps,
        HomeserverUrlError::HasUserinfo,
        HomeserverUrlError::HasQuery,
        HomeserverUrlError::HasFragment,
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

// ---------------------------------------------------------------- .well-known

#[test]
fn the_well_known_gives_the_homeserver() {
    let config = parse_well_known(&well_known("https://matrix.example.org")).unwrap();
    assert_eq!(config.base_url, url("https://matrix.example.org"));
}

#[test]
fn a_trailing_slash_of_the_well_known_is_tolerated() {
    let config = parse_well_known(&well_known("https://matrix.example.org/")).unwrap();
    assert_eq!(config.base_url.as_str(), "https://matrix.example.org");
}

#[test]
fn the_rest_of_the_well_known_is_ignored() {
    let body = r#"{
        "m.homeserver": {"base_url": "https://hs.example.org", "future": [1, 2]},
        "m.identity_server": {"base_url": "https://vector.im"},
        "app.twake.tom": {"base_url": "https://tom.example.org"},
        "unknown": null
    }"#;
    assert_eq!(
        parse_well_known(body).unwrap().base_url,
        url("https://hs.example.org")
    );
}

#[test]
fn a_dev_stack_may_answer_with_a_loopback_http_address() {
    let config = parse_well_known(&well_known("http://localhost:8008")).unwrap();
    assert_eq!(config.base_url.as_str(), "http://localhost:8008");
}

#[test]
fn a_well_known_that_is_not_json_is_refused() {
    assert_eq!(
        parse_well_known("<html>404</html>"),
        Err(WellKnownError::NotJson)
    );
    assert_eq!(parse_well_known(""), Err(WellKnownError::NotJson));
}

#[test]
fn a_well_known_without_a_homeserver_is_refused() {
    for body in [
        "{}",
        "[]",
        "null",
        r#""text""#,
        r#"{"m.identity_server":{"base_url":"https://i.org"}}"#,
        r#"{"m.homeserver":"https://hs.org"}"#,
    ] {
        assert_eq!(
            parse_well_known(body),
            Err(WellKnownError::MissingHomeserver),
            "{body}"
        );
    }
}

#[test]
fn a_homeserver_without_a_usable_base_url_is_refused() {
    for body in [
        r#"{"m.homeserver":{}}"#,
        r#"{"m.homeserver":{"base_url":null}}"#,
        r#"{"m.homeserver":{"base_url":42}}"#,
        r#"{"m.homeserver":{"base_url":["https://hs.org"]}}"#,
    ] {
        assert_eq!(
            parse_well_known(body),
            Err(WellKnownError::MissingBaseUrl),
            "{body}"
        );
    }
}

#[test]
fn a_base_url_that_is_not_a_safe_address_says_why() {
    assert_eq!(
        parse_well_known(&well_known("http://matrix.example.org")),
        Err(WellKnownError::BadBaseUrl(HomeserverUrlError::NotHttps))
    );
    assert_eq!(
        parse_well_known(&well_known("https://alice:pw@matrix.example.org")),
        Err(WellKnownError::BadBaseUrl(HomeserverUrlError::HasUserinfo))
    );
    assert_eq!(
        parse_well_known(&well_known("")),
        Err(WellKnownError::BadBaseUrl(HomeserverUrlError::Empty))
    );
}

#[test]
fn every_well_known_error_says_something_different() {
    let all = [
        WellKnownError::NotJson,
        WellKnownError::MissingHomeserver,
        WellKnownError::MissingBaseUrl,
        WellKnownError::BadBaseUrl(HomeserverUrlError::NotHttps),
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

// ---------------------------------------------------------------- /login flows

fn provider(id: &str, name: &str) -> IdentityProvider {
    IdentityProvider {
        id: id.to_string(),
        name: name.to_string(),
    }
}

#[test]
fn an_sso_flow_lists_its_identity_providers_in_order() {
    let body = r#"{"flows":[{"type":"m.login.sso","identity_providers":[
        {"id":"oidc-twake","name":"Twake","icon":"mxc://x/y","brand":"twake"},
        {"id":"oidc-github","name":"GitHub"}]}]}"#;
    let flows = LoginFlows::from_json(body).unwrap();
    let sso = flows.sso.unwrap();
    assert_eq!(
        sso.identity_providers,
        vec![
            provider("oidc-twake", "Twake"),
            provider("oidc-github", "GitHub")
        ]
    );
    assert!(!flows.password);
}

#[test]
fn an_sso_flow_without_providers_is_still_an_sso_flow() {
    let flows = LoginFlows::from_json(r#"{"flows":[{"type":"m.login.sso"}]}"#).unwrap();
    assert_eq!(flows.sso.unwrap().identity_providers, vec![]);
}

#[test]
fn a_password_flow_alone_is_enough() {
    let flows = LoginFlows::from_json(r#"{"flows":[{"type":"m.login.password"}]}"#).unwrap();
    assert!(flows.password);
    assert!(flows.sso.is_none());
}

#[test]
fn a_server_may_offer_both() {
    let body = r#"{"flows":[{"type":"m.login.password"},{"type":"m.login.sso"}]}"#;
    let flows = LoginFlows::from_json(body).unwrap();
    assert!(flows.password);
    assert!(flows.sso.is_some());
}

#[test]
fn other_flows_are_ignored() {
    let body = r#"{"flows":[{"type":"m.login.token"},{"type":"m.login.cas"},{"type":"m.login.password"}]}"#;
    let flows = LoginFlows::from_json(body).unwrap();
    assert!(flows.password);
    assert!(flows.sso.is_none());
}

#[test]
fn a_server_with_no_flow_we_support_is_refused() {
    for body in [
        r#"{"flows":[]}"#,
        r#"{"flows":[{"type":"m.login.token"},{"type":"m.login.cas"}]}"#,
    ] {
        assert_eq!(
            LoginFlows::from_json(body),
            Err(LoginFlowsError::NoSupportedLogin),
            "{body}"
        );
    }
}

#[test]
fn an_answer_that_is_not_a_flow_list_is_refused() {
    assert_eq!(
        LoginFlows::from_json("not json"),
        Err(LoginFlowsError::NotJson)
    );
    for body in ["{}", r#"{"flows":"m.login.sso"}"#, "[]", "null"] {
        assert_eq!(
            LoginFlows::from_json(body),
            Err(LoginFlowsError::Malformed),
            "{body}"
        );
    }
}

#[test]
fn a_provider_without_an_id_or_a_name_is_dropped() {
    let body = r#"{"flows":[{"type":"m.login.sso","identity_providers":[
        {"name":"No id"},{"id":"no-name"},{"id":7,"name":"Number"},{"id":"ok","name":"Fine"}]}]}"#;
    let sso = LoginFlows::from_json(body).unwrap().sso.unwrap();
    assert_eq!(sso.identity_providers, vec![provider("ok", "Fine")]);
}

#[test]
fn a_provider_id_that_could_escape_the_url_path_is_dropped() {
    let body = r#"{"flows":[{"type":"m.login.sso","identity_providers":[
        {"id":"a/b","name":"Slash"},{"id":"..","name":"Dots"},{"id":"a?b","name":"Query"},
        {"id":"a b","name":"Space"},{"id":"","name":"Empty"},{"id":"oidc-twake_1.2~x","name":"Fine"}]}]}"#;
    let sso = LoginFlows::from_json(body).unwrap().sso.unwrap();
    assert_eq!(
        sso.identity_providers,
        vec![provider("oidc-twake_1.2~x", "Fine")]
    );
}

#[test]
fn every_flow_error_says_something_different() {
    let all = [
        LoginFlowsError::NotJson,
        LoginFlowsError::Malformed,
        LoginFlowsError::NoSupportedLogin,
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
