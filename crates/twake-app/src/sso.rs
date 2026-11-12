//! One sign-in attempt through the system browser: the URL to open and the callback to accept
//! (FR-012, FR-013, D217; ARCHITECTURE.md section 7). Pure: the randomness is given.

use std::fmt::Write as _;

use twake_domain::session::{HomeserverUrl, IdentityProvider, Secret};
use url::Url;

/// Why a sign-in attempt cannot be built.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SsoError {
    /// The redirect is not `http://127.0.0.1:<port><path>` with no credentials, query or fragment.
    #[error("the redirect must be http://127.0.0.1:<port>/<path>")]
    InvalidRedirect,
}

/// Why a callback is refused. A refused callback leaves the request answerable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CallbackError {
    /// Not an absolute URL.
    #[error("the callback is not a URL")]
    Malformed,
    /// Scheme, credentials, host, port or path differs from the redirect given.
    #[error("the callback is not for this application")]
    WrongTarget,
    /// No `state` parameter.
    #[error("the callback has no state")]
    MissingState,
    /// A `state` that is not ours.
    #[error("the callback state is not ours")]
    WrongState,
    /// The request was already answered with success.
    #[error("the callback was already used")]
    Replayed,
    /// No `loginToken`, or an empty one.
    #[error("the callback has no login token")]
    MissingToken,
    /// `state` or `loginToken` more than once.
    #[error("the callback repeats a parameter")]
    DuplicateParameter,
}

/// One sign-in attempt (D217, FR-013).
pub struct SsoRequest {
    state: String,
    url: String,
    redirect: Url,
    spent: bool,
}

fn loopback_redirect(redirect: &str) -> Option<Url> {
    let url = Url::parse(redirect).ok()?;
    let plain = url.scheme() == "http"
        && url.host_str() == Some("127.0.0.1")
        && url.port().is_some()
        && url.username().is_empty()
        && url.password().is_none();
    (plain && url.query().is_none() && url.fragment().is_none()).then_some(url)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// The value of `name` when it appears at most once.
fn single(url: &Url, name: &str) -> Result<Option<String>, CallbackError> {
    let mut values = url
        .query_pairs()
        .filter(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned());
    let first = values.next();
    match values.next() {
        Some(_) => Err(CallbackError::DuplicateParameter),
        None => Ok(first),
    }
}

fn target(url: &Url) -> (&str, &str, Option<&str>, Option<&str>, Option<u16>, &str) {
    (
        url.scheme(),
        url.username(),
        url.password(),
        url.host_str(),
        url.port(),
        url.path(),
    )
}

impl SsoRequest {
    /// `redirect`: `http://127.0.0.1:<port><path>`. `random`: 16 bytes of OS randomness, the
    /// state is their lowercase hex.
    pub fn new(
        homeserver: &HomeserverUrl,
        idp: Option<&IdentityProvider>,
        redirect: &str,
        random: [u8; 16],
    ) -> Result<Self, SsoError> {
        let redirect = loopback_redirect(redirect).ok_or(SsoError::InvalidRedirect)?;
        let state = hex(&random);
        let path = homeserver.api_url("/_matrix/client/v3/login/sso/redirect");
        let mut url = Url::parse(&path).map_err(|_| SsoError::InvalidRedirect)?;
        if let Some(idp) = idp {
            url.path_segments_mut()
                .map_err(|()| SsoError::InvalidRedirect)?
                .push(&idp.id);
        }
        url.query_pairs_mut()
            .append_pair("redirectUrl", &format!("{redirect}?state={state}"));
        Ok(Self {
            state,
            url: url.into(),
            redirect,
            spent: false,
        })
    }

    /// The state of this attempt.
    pub fn state(&self) -> &str {
        &self.state
    }

    /// The URL to open in the system browser.
    pub fn url(&self) -> &str {
        &self.url
    }

    fn check_state(&self, state: Option<String>) -> Result<(), CallbackError> {
        match state {
            None => Err(CallbackError::MissingState),
            Some(state) if state != self.state => Err(CallbackError::WrongState),
            Some(_) => Ok(()),
        }
    }

    /// The URL the homeserver sent the browser back to; gives the login token, once.
    pub fn accept_callback(&mut self, callback_url: &str) -> Result<Secret, CallbackError> {
        let url = Url::parse(callback_url).map_err(|_| CallbackError::Malformed)?;
        if target(&url) != target(&self.redirect) {
            return Err(CallbackError::WrongTarget);
        }
        let (state, token) = (single(&url, "state")?, single(&url, "loginToken")?);
        self.check_state(state)?;
        if self.spent {
            return Err(CallbackError::Replayed);
        }
        let token = token
            .filter(|token| !token.is_empty())
            .ok_or(CallbackError::MissingToken)?;
        self.spent = true;
        Ok(Secret::new(token))
    }
}
