//! Matrix identifiers, parsed once at the boundary (ARCHITECTURE.md section 4).
//! Spec: <https://spec.matrix.org/latest/appendices/#common-identifier-format>.

use std::{fmt, str::FromStr};

use thiserror::Error;

/// Longest identifier, sigil and server name included, in bytes.
const MAX_BYTES: usize = 255;
/// Longest port of a server name, in digits.
const MAX_PORT_DIGITS: usize = 5;

/// Why a string is not a valid identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum IdError {
    /// The string, or the part after the sigil, is empty.
    #[error("empty identifier")]
    Empty,
    /// The first character is not the sigil of this kind of identifier.
    #[error("wrong sigil")]
    WrongSigil,
    /// This kind of identifier needs a `:server` part and has none.
    #[error("missing server name")]
    MissingServer,
    /// The server name is not a hostname, an IPv4 or a `[IPv6]` address with an optional port.
    #[error("invalid server name")]
    InvalidServerName,
    /// A whitespace or control character, or one the kind forbids.
    #[error("invalid character")]
    InvalidCharacter,
    /// More than 255 bytes.
    #[error("longer than 255 bytes")]
    TooLong,
}

/// Strips the sigil and checks the length: what follows the sigil.
fn body(s: &str, sigil: char) -> Result<&str, IdError> {
    let Some(body) = s.strip_prefix(sigil) else {
        return Err(if s.is_empty() {
            IdError::Empty
        } else {
            IdError::WrongSigil
        });
    };
    if s.len() > MAX_BYTES {
        return Err(IdError::TooLong);
    }
    Ok(body)
}

/// The part before the server name: not empty, no whitespace, no control character.
pub(crate) fn check_opaque(opaque: &str) -> Result<(), IdError> {
    if opaque.is_empty() {
        Err(IdError::Empty)
    } else if opaque.chars().any(|c| c.is_whitespace() || c.is_control()) {
        Err(IdError::InvalidCharacter)
    } else {
        Ok(())
    }
}

/// Checks the sigil, the length, the characters and the server name of `s`.
/// Returns the byte index of the `:` that starts the server name, if there is one.
fn split(s: &str, sigil: char) -> Result<Option<usize>, IdError> {
    let body = body(s, sigil)?;
    let (opaque, server) = match body.split_once(':') {
        Some((opaque, server)) => (opaque, Some(server)),
        None => (body, None),
    };
    check_opaque(opaque)?;
    match server {
        None => Ok(None),
        Some(server) if is_server_name(server) => Ok(Some(sigil.len_utf8() + opaque.len())),
        Some(_) => Err(IdError::InvalidServerName),
    }
}

/// Hostname, IPv4 or `[IPv6]`, then an optional `:port`.
fn is_server_name(s: &str) -> bool {
    let (host_ok, after_host) = match s.strip_prefix('[') {
        Some(rest) => rest
            .split_once(']')
            .map_or((false, ""), |(ip, after)| (is_ip_literal(ip), after)),
        None => {
            let (host, after) = s.split_at(s.find(':').unwrap_or(s.len()));
            (is_hostname(host), after)
        }
    };
    host_ok && (after_host.is_empty() || after_host.strip_prefix(':').is_some_and(is_port))
}

fn is_ip_literal(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.')
}

fn is_hostname(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

fn is_port(s: &str) -> bool {
    (1..=MAX_PORT_DIGITS).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

/// An identifier whose server name is optional (room ids of version 12, event ids from version 3).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Opaque {
    raw: String,
    colon: Option<usize>,
}

impl Opaque {
    fn parse(s: &str, sigil: char) -> Result<Self, IdError> {
        Ok(Self {
            raw: s.to_owned(),
            colon: split(s, sigil)?,
        })
    }

    fn server_name(&self) -> Option<&str> {
        self.colon.and_then(|c| self.raw.get(c + 1..))
    }
}

/// An identifier that always ends in `:server` (users, aliases).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Qualified {
    raw: String,
    colon: usize,
}

impl Qualified {
    fn parse(s: &str, sigil: char) -> Result<Self, IdError> {
        let colon = split(s, sigil)?.ok_or(IdError::MissingServer)?;
        Ok(Self {
            raw: s.to_owned(),
            colon,
        })
    }

    fn localpart(&self) -> &str {
        self.raw.get(1..self.colon).unwrap_or_default()
    }

    fn server_name(&self) -> &str {
        self.raw.get(self.colon + 1..).unwrap_or_default()
    }
}

/// Derives the shared surface of an identifier wrapping `$inner` (which has a `raw` field).
macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident($inner:ident)) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name($inner);

        impl $name {
            /// The identifier exactly as received.
            pub fn as_str(&self) -> &str {
                &self.0.raw
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(s: &str) -> Result<Self, IdError> {
                Self::parse(s)
            }
        }
    };
}

id_type!(
    /// A room id: `!opaque:server`, or `!hash` from room version 12.
    RoomId(Opaque)
);
id_type!(
    /// An event id: `$hash`, or `$opaque:server` in room versions 1 and 2.
    EventId(Opaque)
);
id_type!(
    /// A user id: `@localpart:server`.
    UserId(Qualified)
);
id_type!(
    /// A room alias: `#alias:server`.
    RoomAlias(Qualified)
);

impl RoomId {
    /// Parses a room id.
    pub fn parse(s: &str) -> Result<Self, IdError> {
        Opaque::parse(s, '!').map(Self)
    }

    /// The server part, absent from a room version 12 id.
    pub fn server_name(&self) -> Option<&str> {
        self.0.server_name()
    }
}

impl EventId {
    /// Parses an event id.
    pub fn parse(s: &str) -> Result<Self, IdError> {
        Opaque::parse(s, '$').map(Self)
    }

    /// The server part, absent from the hashes of room version 3 and later.
    pub fn server_name(&self) -> Option<&str> {
        self.0.server_name()
    }
}

impl UserId {
    /// Parses a user id. Historical uppercase localparts are accepted and kept.
    pub fn parse(s: &str) -> Result<Self, IdError> {
        let id = Qualified::parse(s, '@')?;
        if id.localpart().contains('@') {
            return Err(IdError::InvalidCharacter);
        }
        Ok(Self(id))
    }

    /// The part between `@` and `:`.
    pub fn localpart(&self) -> &str {
        self.0.localpart()
    }

    /// The server name, with its port if any.
    pub fn server_name(&self) -> &str {
        self.0.server_name()
    }
}

impl RoomAlias {
    /// Parses a room alias.
    pub fn parse(s: &str) -> Result<Self, IdError> {
        Qualified::parse(s, '#').map(Self)
    }

    /// The part between `#` and `:`.
    pub fn localpart(&self) -> &str {
        self.0.localpart()
    }

    /// The server name, with its port if any.
    pub fn server_name(&self) -> &str {
        self.0.server_name()
    }
}

/// A Matrix content URI: `mxc://<server name>/<media id>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MxcUri {
    raw: String,
    /// Byte index of the `/` between the server name and the media id.
    slash: usize,
}

impl MxcUri {
    const SCHEME: &'static str = "mxc://";

    /// Parses a content URI: a server name, then one media id without blank or control character.
    pub fn parse(s: &str) -> Result<Self, IdError> {
        if s.is_empty() {
            return Err(IdError::Empty);
        }
        let rest = s.strip_prefix(Self::SCHEME).ok_or(IdError::WrongSigil)?;
        let (server, media) = rest.split_once('/').ok_or(IdError::MissingServer)?;
        if !is_server_name(server) {
            return Err(IdError::InvalidServerName);
        }
        if media.is_empty() {
            return Err(IdError::Empty);
        }
        if media.contains('/') || has_blank(media) {
            return Err(IdError::InvalidCharacter);
        }
        Ok(Self {
            raw: s.to_owned(),
            slash: Self::SCHEME.len() + server.len(),
        })
    }

    /// The URI exactly as received.
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// The server name, with its port if any.
    pub fn server_name(&self) -> &str {
        self.raw
            .get(Self::SCHEME.len()..self.slash)
            .unwrap_or_default()
    }

    /// The media id.
    pub fn media_id(&self) -> &str {
        self.raw.get(self.slash + 1..).unwrap_or_default()
    }
}

/// A client transaction id: the identity of a message while it is being sent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TxnId(String);

impl TxnId {
    /// Parses a transaction id: not empty, no whitespace or control character.
    pub fn parse(s: &str) -> Result<Self, IdError> {
        if s.is_empty() {
            Err(IdError::Empty)
        } else if has_blank(s) {
            Err(IdError::InvalidCharacter)
        } else {
            Ok(Self(s.to_owned()))
        }
    }

    /// The id exactly as received.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn has_blank(s: &str) -> bool {
    s.chars().any(|c| c.is_whitespace() || c.is_control())
}
