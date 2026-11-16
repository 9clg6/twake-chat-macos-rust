//! The session of one account and its pieces (FR-014, NFR-015, D373).
//! Sign-in discovery lives in `homeserver` and `login`.

mod homeserver;
mod login;

use std::fmt;

use crate::ids::{IdError, UserId, check_opaque};

pub use homeserver::{
    HomeserverConfig, HomeserverUrl, HomeserverUrlError, WellKnownError, parse_well_known,
};
pub use login::{IdentityProvider, LoginFlows, LoginFlowsError, SsoFlow};

/// A text that must not reach a log: `Debug` and `Display` print a placeholder, never the text.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    /// Wraps `text`.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The text itself: the one way out, named so a review finds it.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<hidden>)")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<hidden>")
    }
}

/// A Matrix device id: an opaque string, not empty, with no whitespace or control character.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId(String);

impl DeviceId {
    /// Parses a device id.
    pub fn parse(s: &str) -> Result<Self, IdError> {
        check_opaque(s)?;
        Ok(Self(s.to_owned()))
    }

    /// The id as received.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A signed-in account on a homeserver. `Debug` hides the token through [`Secret`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// Who is signed in.
    pub user_id: UserId,
    /// The device the server knows this session by.
    pub device_id: DeviceId,
    /// The access token.
    pub access_token: Secret,
    /// Where the session was opened.
    pub homeserver: HomeserverUrl,
}
