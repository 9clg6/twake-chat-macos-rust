# Sign-in core API sketch: pure logic, no network, no UI

Written by the test author from the need, before the production code exists. This is the
exact public surface the tests use. The implementer may add more, but every item here must
exist with these names, types and semantics. Anything the implementer thinks wrong is
challenged against the need (the cited source), not against the code.

Slice: EPIC-02 "sign in", pure part (FR-012 discovery and SSO, FR-013 state check, FR-014 session
and device id, FR-015 lifecycle). Sources: PRD FR-012..FR-015, NFR-015/019, ARCHITECTURE.md
sections 7 and 9, A005, A009, A011 Q6, `docs/spec/features/auth.md` (D4, D72, D217),
`accounts.md` (D373, D395, D396), the Matrix Client-Server spec (Server Discovery, Login, SSO
client login; D418), and, read only, the React `discoverHomeserver.ts` / `matrixLogin.ts`.

Tests: `crates/twake-domain/tests/{homeserver,session}.rs`,
`crates/twake-app/tests/{sso,session_machine,session_store_port}.rs`,
`crates/twake-testing/tests/session_store.rs`, and the contract suite
`crates/twake-testing/src/session_store_contract.rs` (re-exported as
`twake_testing::contract::session_store`; written, not to be loosened).
Manifest edit by the test author: `twake-app` dev-dependency `url = "2"` (used by `sso.rs` to
read the URL built; the implementer also needs it, or `percent-encoding`, as a normal
dependency). Also edited: `contract.rs` (`same` / `holds` are `pub(crate)`, one `pub use`),
`twake-testing/src/lib.rs` (`mod session_store_contract;`; it must also re-export
`FakeSessionStore`).
The implementer adds: `twake-domain` -> `url` and `serde_json` (JSON is read from `&str`, no
serde type crosses the API); `twake-app` -> `url`. A secret zeroizing on drop (`zeroize`) is the
implementer's call: no test can observe it. No tokio, no network anywhere (A011 Q6).

---

## 1. `twake-domain` (module `twake_domain::session`; `Session`, `Secret`, `DeviceId`, `HomeserverUrl` also at the crate root)

```rust
/// A text that must not reach a log. Debug and Display print a fixed placeholder, never the text.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret { /* private */ }
impl Secret {
    pub fn new(text: impl Into<String>) -> Self;
    pub fn expose(&self) -> &str;         // the one way out, named so a review finds it
}                                         // impl Debug, Display: non-empty placeholder, no trace of the text

/// Matrix device id: an opaque string (spec), not empty, no whitespace, no control character.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeviceId { /* private */ }
impl DeviceId {
    pub fn parse(s: &str) -> Result<Self, IdError>;   // "" -> Empty; whitespace / control -> InvalidCharacter
    pub fn as_str(&self) -> &str;                     // impl Display
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {                       // Debug is derived: the token stays hidden through Secret
    pub user_id: UserId,
    pub device_id: DeviceId,
    pub access_token: Secret,
    pub homeserver: HomeserverUrl,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HomeserverUrlError { Empty, Invalid, NotHttps, HasUserinfo, HasQuery, HasFragment }
                                           // distinct non-empty Display texts

/// Address of a homeserver, normalised, safe to put in front of a token.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HomeserverUrl { /* private */ }
impl HomeserverUrl {
    pub fn parse(s: &str) -> Result<Self, HomeserverUrlError>;
    pub fn as_str(&self) -> &str;
    /// `path` starts with '/'; the result is the address, then the path. A path prefix of the address is kept.
    pub fn api_url(&self, path: &str) -> String;
    /// Whether the server name of `user` is the host of this address (the first two rules of
    /// the React `servesUser`). `false` is not final: the caller then reads the well-known of
    /// `user.server_name()` and compares the address found with `==` (that part needs the net).
    pub fn serves_directly(&self, user: &UserId) -> bool;
}
```

`HomeserverUrl::parse` rules: surrounding whitespace ignored; empty -> `Empty`; no scheme or not a
URL -> `Invalid`; scheme other than https, or http on a host that is not exactly `localhost`,
`127.0.0.1` or `[::1]` -> `NotHttps` (`localhost.evil.org` is not loopback); user or password ->
`HasUserinfo`; a query or a fragment, even empty (`https://a.org?`) -> `HasQuery` / `HasFragment`.
Normal form: scheme and host lowercase, default port dropped, trailing slashes of the path
removed, port and path prefix kept (`https://example.org:8448/matrix`). Two spellings of one
homeserver are `==`.

`serves_directly`: true when `host[:port]` of the address (default port dropped, as in the
normal form) equals the server name of the user, compared whole (no suffix or prefix match;
a port makes another server); or when the address host and the server name host (port ignored)
are both in {`localhost`, `127.0.0.1`}, `[::1]` included: a stack on this machine names its
server as it likes.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeserverConfig { pub base_url: HomeserverUrl }

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WellKnownError {
    NotJson,                       // the body is not JSON
    MissingHomeserver,             // not an object, or no object under "m.homeserver"
    MissingBaseUrl,                // "base_url" absent, null, or not a string
    BadBaseUrl(HomeserverUrlError),// a string that is not a safe address ("" -> Empty, http -> NotHttps...)
}                                  // distinct non-empty Display texts

/// Body of `GET /.well-known/matrix/client`. Everything but `m.homeserver.base_url` is ignored
/// (identity server, `app.twake.*`, unknown keys). A trailing slash is tolerated.
pub fn parse_well_known(body: &str) -> Result<HomeserverConfig, WellKnownError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityProvider { pub id: String, pub name: String }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsoFlow { pub identity_providers: Vec<IdentityProvider> }   // may be empty
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginFlows { pub sso: Option<SsoFlow>, pub password: bool }

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoginFlowsError { NotJson, Malformed, NoSupportedLogin }      // distinct non-empty Display texts

impl LoginFlows {
    /// Body of `GET /_matrix/client/v3/login`. `flows` absent or not an array -> `Malformed`;
    /// no `m.login.sso` and no `m.login.password` among them (`m.login.token`, `m.login.cas`,
    /// unknown ones are ignored) -> `NoSupportedLogin`.
    /// A provider needs a string `id` and a string `name`, else it is dropped; its `id` must
    /// match `[A-Za-z0-9._~-]+` and not be `.` or `..` (it goes into a URL path), else dropped.
    /// Order of the server kept; `icon`, `brand` ignored.
    pub fn from_json(body: &str) -> Result<Self, LoginFlowsError>;
}
```

## 2. `twake-app`

### `twake_app::sso`

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SsoError { InvalidRedirect }

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CallbackError {
    Malformed,          // not an absolute URL
    WrongTarget,        // scheme, userinfo, host, port or path differs from the redirect we gave
    MissingState,       // no `state` parameter
    WrongState,         // `state` present (even empty) and not ours (exact, case-sensitive)
    Replayed,           // our request was already answered with success
    MissingToken,       // no `loginToken`, or an empty one
    DuplicateParameter, // `state` or `loginToken` more than once (even if one copy is right)
}                       // distinct non-empty Display texts

/// One sign-in attempt through the system browser (D217, FR-013). Pure: the randomness is given.
pub struct SsoRequest { /* private */ }
impl SsoRequest {
    /// `redirect`: `http://127.0.0.1:<port><path>` with a port, no userinfo, no query, no fragment
    /// (ARCHITECTURE.md section 7); anything else -> `InvalidRedirect` (`localhost`, https, no port...).
    /// `random` is drawn by the caller (16 bytes of OS randomness); the state is its lowercase hex (32 chars).
    pub fn new(
        homeserver: &HomeserverUrl,
        idp: Option<&IdentityProvider>,
        redirect: &str,
        random: [u8; 16],
    ) -> Result<Self, SsoError>;
    pub fn state(&self) -> &str;
    /// `{homeserver}/_matrix/client/v3/login/sso/redirect[/{idp.id}]?redirectUrl={redirect}?state={state}`,
    /// `redirectUrl` percent-encoded as one query parameter, the only one.
    pub fn url(&self) -> &str;
    /// The URL the homeserver sent the browser back to. Order of checks: parse (`Malformed`),
    /// target (`WrongTarget`), duplicated `state` / `loginToken` (`DuplicateParameter`), `state`
    /// (`MissingState`, `WrongState`), already used (`Replayed`), token (`MissingToken`).
    /// Other parameters are ignored, repeated or not. The token is percent-decoded.
    /// Only a success spends the request: a refused callback leaves it answerable.
    pub fn accept_callback(&mut self, callback_url: &str) -> Result<Secret, CallbackError>;
}
```

### `twake_app::session`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    SignedOut,
    Discovering,                    // the user entered a server; the net looks it up
    AwaitingLogin(HomeserverUrl),   // flows known; the user picks SSO or types a password
    LoggingIn(HomeserverUrl),       // token exchange / password login / check of a stored session
    Active(Session),
    Expired(Session),               // the server answered 401; kept to name the account in the message
    SigningOut(Session),            // logout in flight
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    ServerEntered,                  // SignedOut | Expired -> Discovering (also AwaitingLogin: another server)
    HomeserverResolved(HomeserverUrl), // Discovering -> AwaitingLogin
    DiscoveryFailed,                // Discovering -> SignedOut
    SsoCallback,                    // AwaitingLogin -> LoggingIn (the callback was accepted by SsoRequest)
    PasswordSubmitted,              // AwaitingLogin -> LoggingIn
    LoginFailed,                    // LoggingIn -> AwaitingLogin (same homeserver)
    LoginOk(Session),               // LoggingIn -> Active: the ONLY way to Active
    Restore(Session),               // SignedOut -> LoggingIn(session.homeserver), device remembered; the adapter then
                                    // checks the stored session (`whoami`) and answers LoginOk or LoginFailed
    Cancelled,                      // Discovering | AwaitingLogin | LoggingIn -> SignedOut
    Unauthorized401,                // Active -> Expired
    SignOut,                        // Active -> SigningOut; Expired -> SignedOut
    SignOutDone,                    // SigningOut -> SignedOut (also when the server could not be reached)
}

pub struct SessionMachine { /* private */ }
impl SessionMachine {
    pub fn new() -> Self;                                    // SignedOut, no device
    pub fn apply(&mut self, event: SessionEvent);            // an event not listed for the state is ignored
    pub fn state(&self) -> &SessionState;
    /// The device of the last session seen (`LoginOk`, `Restore`): kept through Expired, SigningOut,
    /// SignedOut and failed attempts, replaced by the next `LoginOk`. The login sends it as
    /// `device_id` (FR-014); the server may still answer another one.
    pub fn device_id(&self) -> Option<&DeviceId>;
}
```

Not listed = ignored: e.g. `LoginOk` outside `LoggingIn` (a late answer after a sign-out, a
second answer when Active), `Unauthorized401` outside Active (also while SigningOut),
`Restore` outside SignedOut.

### `twake_app::ports` (A009)

```rust
pub const MAX_ACCOUNTS: usize = 5;                           // D373

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionStoreError { Unavailable, TooManyAccounts }  // distinct non-empty Display texts

/// Sessions of the signed-in accounts, in the OS keychain (A005, NFR-015). One per user id.
pub trait SessionStorePort: Send + Sync {
    /// Inserts, or replaces the session of the same user id in place (its place in `list` is kept).
    /// A sixth distinct user id -> `TooManyAccounts`, nothing changes. Saving the same session twice changes nothing.
    fn save(&self, session: Session) -> BoxFuture<'_, Result<(), SessionStoreError>>;
    fn load(&self, user: UserId) -> BoxFuture<'_, Result<Option<Session>, SessionStoreError>>;
    /// In the order the accounts were first saved (the first signed in is the default one, D373).
    fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, SessionStoreError>>;
    /// Unknown user: Ok. A cleared account frees its place; saved again it goes last.
    fn clear(&self, user: UserId) -> BoxFuture<'_, Result<(), SessionStoreError>>;
}
```

## 3. `twake-testing`

```rust
pub struct FakeSessionStore { /* Mutex inside; shareable as Arc */ }
impl FakeSessionStore {
    pub fn new() -> Self;
    pub fn unavailable(self) -> Self;    // every call returns Err(Unavailable)
}
impl SessionStorePort for FakeSessionStore { /* passes contract::session_store */ }

pub fn contract::session_store(make: impl Fn() -> Arc<dyn SessionStorePort>) -> Verdict;  // `make` builds an EMPTY store
```

## 4. Open questions (need, not code)

1. **`AwaitingSso` renamed `AwaitingLogin`**: FR-012 also asks for the password form, so the
   state after discovery is not only SSO. Say if you want both names.
2. **Device id after an explicit sign-out**: kept (FR-014 says "reused at next sign-in"), but the
   server deletes the device on logout, so its E2EE keys are gone; reusing the id is allowed by
   the spec (`device_id` of the login request) but D396 only speaks of a session the server ended.
   Pinned as kept; decide before the adapter.
3. **Restore as `LoggingIn`**: a stored session is checked (D72, `whoami`) before it is Active; if
   the check fails with a 401 the machine goes to `AwaitingLogin`, not `Expired` (no "session has
   ended" message for a restart). FR-015's 401 message is therefore for sync only.
4. **State and token in a URL the browser can show**: the state travels in `redirectUrl`, as in
   the React app; the loopback listener must answer once and close (adapter concern).
5. **`serves_directly` and the default port**: `https://example.org` vs `@a:example.org:443` is
   false (as in the React `host === server`); the Matrix spec treats an explicit 443 as another
   server name too, so left unpinned by the tests. Host case is also unpinned.
6. **Provider id charset** `[A-Za-z0-9._~-]`: my reading of the spec for URL safety (the spec
   restricts idp ids loosely); a stricter or looser rule changes one test.
7. **`SsoRequest::new` takes `random: [u8; 16]`** rather than an `Rng` trait: simpler and as
   deterministic; the caller (desktop crate) draws it from the OS.
8. The `unreachable!()` in `session_store_contract.rs` follows `contract.rs`; replace both by
   a `Verdict` if clippy `unreachable` is ever denied.
