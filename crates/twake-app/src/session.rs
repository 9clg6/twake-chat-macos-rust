//! The lifecycle of the session, as a pure state machine (FR-012 to FR-015, D72, D396).
//! The adapters do the net work and report what happened as a [`SessionEvent`].

use twake_domain::session::{DeviceId, HomeserverUrl, Session};

/// Where the sign-in stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// No account.
    SignedOut,
    /// The user entered a server; the net looks it up.
    Discovering,
    /// Flows known; the user picks SSO or types a password.
    AwaitingLogin(HomeserverUrl),
    /// Token exchange, password login or check of a stored session.
    LoggingIn(HomeserverUrl),
    /// Signed in.
    Active(Session),
    /// The server answered 401; kept to name the account in the message.
    Expired(Session),
    /// Logout in flight.
    SigningOut(Session),
}

/// What happened. An event not listed for the state is ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    /// `SignedOut`, `Expired` or `AwaitingLogin` to `Discovering`.
    ServerEntered,
    /// `Discovering` to `AwaitingLogin`.
    HomeserverResolved(HomeserverUrl),
    /// `Discovering` to `SignedOut`.
    DiscoveryFailed,
    /// `AwaitingLogin` to `LoggingIn`: the callback was accepted.
    SsoCallback,
    /// `AwaitingLogin` to `LoggingIn`.
    PasswordSubmitted,
    /// `LoggingIn` to `AwaitingLogin`, same homeserver (also a stored session the server refused).
    LoginFailed,
    /// `LoggingIn` to `Active`: the only way to `Active`.
    LoginOk(Session),
    /// `SignedOut` to `LoggingIn`, the device remembered; the adapter then checks the stored session.
    Restore(Session),
    /// `Discovering`, `AwaitingLogin` or `LoggingIn` to `SignedOut`.
    Cancelled,
    /// `Active` to `Expired`.
    Unauthorized401,
    /// `Active` to `SigningOut`; `Expired` to `SignedOut`.
    SignOut,
    /// `SigningOut` to `SignedOut`, also when the server could not be reached.
    SignOutDone,
}

/// The state of the session and the device it used last.
#[derive(Debug)]
pub struct SessionMachine {
    state: SessionState,
    device: Option<DeviceId>,
}

impl Default for SessionMachine {
    fn default() -> Self {
        Self::new()
    }
}

fn step(state: &SessionState, event: SessionEvent) -> Option<SessionState> {
    use SessionState as S;
    match state {
        S::SignedOut => signed_out(event),
        S::Discovering => discovering(event),
        S::AwaitingLogin(h) => awaiting_login(h, event),
        S::LoggingIn(h) => logging_in(h, event),
        S::Active(s) => active(s, event),
        S::Expired(_) => expired(event),
        S::SigningOut(_) => (event == SessionEvent::SignOutDone).then_some(S::SignedOut),
    }
}

fn signed_out(event: SessionEvent) -> Option<SessionState> {
    match event {
        SessionEvent::ServerEntered => Some(SessionState::Discovering),
        SessionEvent::Restore(s) => Some(SessionState::LoggingIn(s.homeserver)),
        _ => None,
    }
}

fn discovering(event: SessionEvent) -> Option<SessionState> {
    match event {
        SessionEvent::HomeserverResolved(h) => Some(SessionState::AwaitingLogin(h)),
        SessionEvent::DiscoveryFailed | SessionEvent::Cancelled => Some(SessionState::SignedOut),
        _ => None,
    }
}

fn awaiting_login(homeserver: &HomeserverUrl, event: SessionEvent) -> Option<SessionState> {
    match event {
        SessionEvent::ServerEntered => Some(SessionState::Discovering),
        SessionEvent::SsoCallback | SessionEvent::PasswordSubmitted => {
            Some(SessionState::LoggingIn(homeserver.clone()))
        }
        SessionEvent::Cancelled => Some(SessionState::SignedOut),
        _ => None,
    }
}

fn logging_in(homeserver: &HomeserverUrl, event: SessionEvent) -> Option<SessionState> {
    match event {
        SessionEvent::LoginFailed => Some(SessionState::AwaitingLogin(homeserver.clone())),
        SessionEvent::LoginOk(s) => Some(SessionState::Active(s)),
        SessionEvent::Cancelled => Some(SessionState::SignedOut),
        _ => None,
    }
}

fn active(session: &Session, event: SessionEvent) -> Option<SessionState> {
    match event {
        SessionEvent::Unauthorized401 => Some(SessionState::Expired(session.clone())),
        SessionEvent::SignOut => Some(SessionState::SigningOut(session.clone())),
        _ => None,
    }
}

fn expired(event: SessionEvent) -> Option<SessionState> {
    match event {
        SessionEvent::ServerEntered => Some(SessionState::Discovering),
        SessionEvent::SignOut => Some(SessionState::SignedOut),
        _ => None,
    }
}

impl SessionMachine {
    /// Signed out, no device.
    pub fn new() -> Self {
        Self {
            state: SessionState::SignedOut,
            device: None,
        }
    }

    /// Moves on `event`, or ignores it.
    pub fn apply(&mut self, event: SessionEvent) {
        let device = match &event {
            SessionEvent::LoginOk(s) | SessionEvent::Restore(s) => Some(s.device_id.clone()),
            _ => None,
        };
        let Some(next) = step(&self.state, event) else {
            return;
        };
        // A completed sign-out forgets the device: the server deleted it and its keys.
        let ended = matches!(
            self.state,
            SessionState::Expired(_) | SessionState::SigningOut(_)
        ) && next == SessionState::SignedOut;
        if ended {
            self.device = None;
        } else if device.is_some() {
            self.device = device;
        }
        self.state = next;
    }

    /// The current state.
    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// The device of the last session seen, for the next login (FR-014); `None` after a sign-out.
    pub fn device_id(&self) -> Option<&DeviceId> {
        self.device.as_ref()
    }
}
