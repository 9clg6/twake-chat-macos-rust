//! The typing notice: sent at the first key stroke, then at most once per 4 seconds, and
//! "stopped" after a send.
//!
//! Need (not the code): FR-057 ("Typing notifications sent") and the React app's rule
//! (`makeNotifyTyping`): one notice per `TYPING_RESEND_MS` while the user keeps typing, "stopped"
//! only after a "typing", the server shows the user as typing for `TYPING_TIMEOUT_MS` per notice.
//! Matrix spec (D418): `PUT /rooms/{roomId}/typing/{userId}` with a timeout; a notice must be
//! renewed before the timeout, so the resend delay is shorter than the timeout.
//!
//! Public items used are listed in `docs/dev/composer-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_app::typing::{TYPING_RESEND_MS, TYPING_TIMEOUT_MS, TypingNotice, TypingThrottle};
use twake_domain::Timestamp;

fn at(ms: u64) -> Timestamp {
    Timestamp::from_millis(1_700_000_000_000 + ms)
}

#[test]
fn the_delays_are_those_of_the_spec() {
    assert_eq!(TYPING_RESEND_MS, 4_000);
    assert_eq!(TYPING_TIMEOUT_MS, 6_000);
}

#[test]
fn the_first_key_stroke_sends_a_notice() {
    let mut t = TypingThrottle::new();
    assert_eq!(t.on_input(at(0)), Some(TypingNotice::Started));
}

#[test]
fn strokes_within_four_seconds_send_nothing() {
    let mut t = TypingThrottle::new();
    t.on_input(at(0));
    assert_eq!(t.on_input(at(1)), None);
    assert_eq!(t.on_input(at(3_999)), None);
}

#[test]
fn a_stroke_exactly_four_seconds_later_sends_again() {
    let mut t = TypingThrottle::new();
    t.on_input(at(0));
    assert_eq!(t.on_input(at(4_000)), Some(TypingNotice::Started));
}

#[test]
fn the_delay_runs_from_the_last_notice_not_the_last_stroke() {
    let mut t = TypingThrottle::new();
    t.on_input(at(0));
    t.on_input(at(3_000));
    assert_eq!(t.on_input(at(4_000)), Some(TypingNotice::Started));
}

#[test]
fn after_a_renewal_the_delay_starts_again() {
    let mut t = TypingThrottle::new();
    t.on_input(at(0));
    t.on_input(at(4_500));
    assert_eq!(t.on_input(at(8_499)), None);
    assert_eq!(t.on_input(at(8_500)), Some(TypingNotice::Started));
}

#[test]
fn a_long_pause_then_a_stroke_sends_again() {
    let mut t = TypingThrottle::new();
    t.on_input(at(0));
    assert_eq!(t.on_input(at(60_000)), Some(TypingNotice::Started));
}

#[test]
fn a_send_stops_the_notice_once() {
    let mut t = TypingThrottle::new();
    t.on_input(at(0));
    assert_eq!(t.stop(), Some(TypingNotice::Stopped));
    assert_eq!(t.stop(), None);
}

#[test]
fn stopping_without_typing_sends_nothing() {
    assert_eq!(TypingThrottle::new().stop(), None);
}

#[test]
fn typing_again_after_a_stop_is_not_throttled() {
    let mut t = TypingThrottle::new();
    t.on_input(at(0));
    t.stop();
    assert_eq!(t.on_input(at(10)), Some(TypingNotice::Started));
}

#[test]
fn a_stroke_after_a_stop_then_a_stop_again_sends_a_second_stopped() {
    let mut t = TypingThrottle::new();
    t.on_input(at(0));
    t.stop();
    t.on_input(at(10));
    assert_eq!(t.stop(), Some(TypingNotice::Stopped));
}

proptest! {
    #[test]
    fn notices_are_never_closer_than_the_resend_delay(gaps in proptest::collection::vec(0u64..2_500, 1..60)) {
        let mut t = TypingThrottle::new();
        let mut now = 0u64;
        let mut last: Option<u64> = None;
        for gap in gaps {
            now += gap;
            if t.on_input(at(now)) == Some(TypingNotice::Started) {
                if let Some(previous) = last {
                    prop_assert!(now - previous >= TYPING_RESEND_MS);
                }
                last = Some(now);
            }
        }
        prop_assert!(last.is_some());
    }

    #[test]
    fn continuous_typing_never_goes_quiet_for_more_than_the_server_timeout(gaps in proptest::collection::vec(1u64..1_000, 10..120)) {
        let mut t = TypingThrottle::new();
        let mut now = 0u64;
        let mut last = None;
        for gap in gaps {
            now += gap;
            if t.on_input(at(now)).is_some() {
                last = Some(now);
            }
            prop_assert!(now - last.unwrap_or(0) < TYPING_TIMEOUT_MS);
        }
    }
}
