//! The address of a homeserver and its `.well-known` discovery (FR-012).

use serde_json::Value;
use url::{Host, Url};

use crate::ids::UserId;

/// Why a string is not a safe homeserver address.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HomeserverUrlError {
    /// Nothing but whitespace.
    #[error("the address is empty")]
    Empty,
    /// Not an absolute URL.
    #[error("the address is not a valid URL")]
    Invalid,
    /// Not https (http is for this machine only).
    #[error("the address must use https")]
    NotHttps,
    /// A user name or password in the address.
    #[error("the address must not carry credentials")]
    HasUserinfo,
    /// A query string, even empty.
    #[error("the address must not carry a query")]
    HasQuery,
    /// A fragment, even empty.
    #[error("the address must not carry a fragment")]
    HasFragment,
}

/// The address of a homeserver, normalised and safe to put in front of a token: https (or
/// http on this machine), no credentials, query or fragment. Two spellings of one server are `==`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HomeserverUrl(String);

fn is_this_machine(host: &Host<&str>) -> bool {
    match host {
        Host::Domain(name) => *name == "localhost",
        Host::Ipv4(ip) => ip.octets() == [127, 0, 0, 1],
        Host::Ipv6(ip) => ip.is_loopback(),
    }
}

/// `host` of `host[:port]`, an IPv6 literal keeping its brackets.
fn host_of(authority: &str) -> &str {
    let end = match authority.find(']') {
        Some(i) => i + 1,
        None => authority.find(':').unwrap_or(authority.len()),
    };
    &authority[..end]
}

/// https, or http on this machine; no credentials, query or fragment.
fn check_safe(url: &Url) -> Result<(), HomeserverUrlError> {
    let secure = url.scheme() == "https"
        || (url.scheme() == "http" && url.host().as_ref().is_some_and(is_this_machine));
    if !secure {
        return Err(HomeserverUrlError::NotHttps);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(HomeserverUrlError::HasUserinfo);
    }
    if url.query().is_some() {
        return Err(HomeserverUrlError::HasQuery);
    }
    if url.fragment().is_some() {
        return Err(HomeserverUrlError::HasFragment);
    }
    Ok(())
}

impl HomeserverUrl {
    /// Parses and normalises an address.
    pub fn parse(s: &str) -> Result<Self, HomeserverUrlError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(HomeserverUrlError::Empty);
        }
        let url = Url::parse(s).map_err(|_| HomeserverUrlError::Invalid)?;
        check_safe(&url)?;
        let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
        Ok(Self(format!(
            "{}://{}{port}{}",
            url.scheme(),
            url.host_str().unwrap_or_default(),
            url.path().trim_end_matches('/')
        )))
    }

    /// The normal form: no trailing slash.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The address followed by `path`, which starts with `/`; a path prefix of the address is kept.
    pub fn api_url(&self, path: &str) -> String {
        format!("{}{path}", self.0)
    }

    /// `host[:port]` of the normal form.
    fn authority(&self) -> &str {
        let rest = self.0.split_once("://").map_or("", |(_, rest)| rest);
        rest.split('/').next().unwrap_or_default()
    }

    /// Whether the server name of `user` is the host of this address, whole, port included
    /// (an explicit `:443` is another name than none, and the case must match), or both name
    /// this machine. `false` is not final: the well-known of the server name may point here.
    pub fn serves_directly(&self, user: &UserId) -> bool {
        let (address, name) = (self.authority(), user.server_name());
        let this_machine =
            |authority| matches!(host_of(authority), "localhost" | "127.0.0.1" | "[::1]");
        address == name || (this_machine(address) && this_machine(name))
    }
}

/// What the app reads of `.well-known/matrix/client`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeserverConfig {
    /// Where the homeserver answers.
    pub base_url: HomeserverUrl,
}

/// Why a `.well-known` answer is not usable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WellKnownError {
    /// The body is not JSON.
    #[error("the well-known is not JSON")]
    NotJson,
    /// No `m.homeserver` object.
    #[error("the well-known names no homeserver")]
    MissingHomeserver,
    /// No `base_url` string under `m.homeserver`.
    #[error("the well-known has no base_url")]
    MissingBaseUrl,
    /// The `base_url` is not a safe address.
    #[error("the well-known base_url is not usable: {0}")]
    BadBaseUrl(HomeserverUrlError),
}

/// Reads the body of `GET /.well-known/matrix/client`; everything but `m.homeserver.base_url`
/// is ignored.
pub fn parse_well_known(body: &str) -> Result<HomeserverConfig, WellKnownError> {
    let json: Value = serde_json::from_str(body).map_err(|_| WellKnownError::NotJson)?;
    let homeserver = json
        .get("m.homeserver")
        .filter(|h| h.is_object())
        .ok_or(WellKnownError::MissingHomeserver)?;
    let base_url = homeserver
        .get("base_url")
        .and_then(Value::as_str)
        .ok_or(WellKnownError::MissingBaseUrl)?;
    HomeserverUrl::parse(base_url)
        .map(|base_url| HomeserverConfig { base_url })
        .map_err(WellKnownError::BadBaseUrl)
}
