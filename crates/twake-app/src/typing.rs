//! The typing notice (FR-057): one at the first key stroke, then at most one per
//! `TYPING_RESEND_MS`, and "stopped" after a send. The screen does the I/O.

use twake_domain::Timestamp;

/// One notice per this delay while the user types.
pub const TYPING_RESEND_MS: u64 = 4_000;
/// How long the server shows the user as typing after a notice; longer than the resend delay.
pub const TYPING_TIMEOUT_MS: u64 = 6_000;

/// What to tell the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypingNotice {
    /// The user is typing.
    Started,
    /// The user is not typing any more.
    Stopped,
}

/// Decides which key strokes send a notice.
#[derive(Debug, Default)]
pub struct TypingThrottle {
    /// When the last `Started` went out, if no `Stopped` followed.
    last_started: Option<u64>,
}

impl TypingThrottle {
    /// A throttle that has sent nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// A key stroke: `Started` the first time, then once per `TYPING_RESEND_MS` since the last
    /// notice (exactly that delay later is allowed).
    pub fn on_input(&mut self, now: Timestamp) -> Option<TypingNotice> {
        let now = now.as_millis();
        let recent = self
            .last_started
            .and_then(|last| now.checked_sub(last))
            .is_some_and(|since| since < TYPING_RESEND_MS);
        if recent {
            return None;
        }
        self.last_started = Some(now);
        Some(TypingNotice::Started)
    }

    /// A message sent or the field emptied: `Stopped`, only after a `Started` and once.
    pub fn stop(&mut self) -> Option<TypingNotice> {
        self.last_started.take().map(|_| TypingNotice::Stopped)
    }
}
