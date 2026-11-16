//! Timeline messages, local echo identity and the calendar day of a timestamp.
//!
//! Need (not the code): `docs/spec/features/timeline.md`
//! - D495: a message in flight, then sent (server accepted), is the same bubble.
//! - e2e "message-failed": a message that could not be sent is "Not sent." with Retry/Discard.
//! - e2e "day-divider": messages of today sit under a divider "Today"; the divider is the
//!   calendar day of the reader (a day boundary moves with the time zone).
//! - D138 / e2e "message-quote": a reply knows the event it answers; an edit is marked "(edited)".
//!
//! Public items used are listed in `docs/dev/app-core-api-sketch.md`.

#![cfg(test)]

use twake_domain::timeline::{Message, MessageKey, SendState, TimelineItem};
use twake_domain::{Day, EventId, Timestamp, TxnId, UserId};

// ------------------------------------------------------------ helpers

fn event(n: usize) -> EventId {
    EventId::parse(&format!("$event{n}")).expect("valid event id")
}

fn txn(n: usize) -> TxnId {
    TxnId::parse(&format!("txn{n}")).expect("valid txn id")
}

fn alice() -> UserId {
    UserId::parse("@alice:example.org").expect("valid user id")
}

fn at(ms: u64) -> Timestamp {
    Timestamp::from_millis(ms)
}

fn day(year: i32, month: u8, day: u8) -> Day {
    Day { year, month, day }
}

// ------------------------------------------------------------ Message

#[test]
fn a_server_message_is_sent_and_has_an_event_id_only() {
    let m = Message::remote(event(1), alice(), at(1_500), "hello");
    assert_eq!(m.event_id(), Some(&event(1)));
    assert_eq!(m.txn_id(), None);
    assert_eq!(m.sender, alice());
    assert_eq!(m.timestamp, at(1_500));
    assert_eq!(m.body, "hello");
    assert_eq!(m.send_state, SendState::Sent);
    assert_eq!(m.reply_to, None);
    assert!(!m.edited);
}

#[test]
fn a_local_echo_is_sending_and_has_a_transaction_id_only() {
    let m = Message::local(txn(7), alice(), at(2_500), "on its way");
    assert_eq!(m.event_id(), None);
    assert_eq!(m.txn_id(), Some(&txn(7)));
    assert_eq!(m.send_state, SendState::Sending);
    assert_eq!(m.body, "on its way");
    assert_eq!(m.timestamp, at(2_500));
}

#[test]
fn a_server_message_can_carry_the_transaction_that_made_it() {
    let m = Message::remote(event(3), alice(), at(1), "x").with_txn_id(txn(4));
    assert_eq!(m.event_id(), Some(&event(3)));
    assert_eq!(m.txn_id(), Some(&txn(4)));
    assert_eq!(m.send_state, SendState::Sent);
}

#[test]
fn a_failed_message_says_so() {
    let m = Message::local(txn(1), alice(), at(1), "x").failed();
    assert_eq!(m.send_state, SendState::Failed);
}

#[test]
fn an_edited_message_is_marked() {
    let m = Message::remote(event(1), alice(), at(1), "x").edited();
    assert!(m.edited);
}

#[test]
fn a_reply_knows_the_event_it_answers() {
    let m = Message::remote(event(2), alice(), at(1), "x").with_reply_to(event(1));
    assert_eq!(m.reply_to, Some(event(1)));
}

#[test]
fn a_key_by_event_matches_the_message_with_that_event_id() {
    let m = Message::remote(event(1), alice(), at(1), "x");
    assert!(m.matches(&MessageKey::Event(event(1))));
    assert!(!m.matches(&MessageKey::Event(event(2))));
    assert!(!m.matches(&MessageKey::Txn(txn(1))));
}

#[test]
fn a_key_by_transaction_matches_the_message_with_that_transaction() {
    let echo = Message::local(txn(5), alice(), at(1), "x");
    assert!(echo.matches(&MessageKey::Txn(txn(5))));
    assert!(!echo.matches(&MessageKey::Txn(txn(6))));
    assert!(!echo.matches(&MessageKey::Event(event(5))));
}

#[test]
fn a_confirmed_message_matches_both_its_keys() {
    let m = Message::remote(event(8), alice(), at(1), "x").with_txn_id(txn(9));
    assert!(m.matches(&MessageKey::Event(event(8))));
    assert!(m.matches(&MessageKey::Txn(txn(9))));
}

#[test]
fn timeline_items_compare_by_content() {
    let m = Message::remote(event(1), alice(), at(1), "x");
    assert_eq!(TimelineItem::Message(m.clone()), TimelineItem::Message(m));
    assert_ne!(
        TimelineItem::NewMessages,
        TimelineItem::DayDivider(day(2026, 11, 12))
    );
}

// ------------------------------------------------------------ Day

#[test]
fn the_epoch_is_the_first_of_january_1970() {
    assert_eq!(Day::of(at(0), 0), day(1970, 1, 1));
}

#[test]
fn a_timestamp_gives_its_utc_day() {
    // 2023-11-14 22:13:20 UTC
    assert_eq!(Day::of(at(1_700_000_000_000), 0), day(2023, 11, 14));
}

#[test]
fn a_positive_offset_can_push_the_day_forward() {
    // 22:13 UTC + 3 h = 01:13 the next day
    assert_eq!(Day::of(at(1_700_000_000_000), 3 * 3600), day(2023, 11, 15));
}

#[test]
fn a_negative_offset_can_pull_the_day_back() {
    // 22:13 UTC - 23 h = 23:13 the day before
    assert_eq!(
        Day::of(at(1_700_000_000_000), -23 * 3600),
        day(2023, 11, 13)
    );
}

#[test]
fn an_offset_that_stays_in_the_day_changes_nothing() {
    assert_eq!(Day::of(at(1_700_000_000_000), 3600), day(2023, 11, 14));
    assert_eq!(Day::of(at(1_700_000_000_000), -3600), day(2023, 11, 14));
}

#[test]
fn the_last_millisecond_of_a_year_is_still_that_year() {
    assert_eq!(Day::of(at(1_704_067_199_999), 0), day(2023, 12, 31));
    assert_eq!(Day::of(at(1_704_067_200_000), 0), day(2024, 1, 1));
}

#[test]
fn a_leap_day_exists_in_2024() {
    assert_eq!(Day::of(at(1_709_164_800_000), 0), day(2024, 2, 29));
}

#[test]
fn the_year_2100_is_not_a_leap_year() {
    assert_eq!(Day::of(at(4_107_456_000_000), 0), day(2100, 2, 28));
    assert_eq!(Day::of(at(4_107_542_400_000), 0), day(2100, 3, 1));
}

#[test]
fn days_are_ordered_by_the_calendar() {
    assert!(day(2023, 12, 31) < day(2024, 1, 1));
    assert!(day(2024, 1, 31) < day(2024, 2, 1));
    assert!(day(2024, 2, 1) < day(2024, 2, 2));
}

// ------------------------------------------------------------ properties

proptest::proptest! {
    #[test]
    fn a_later_instant_never_falls_on_an_earlier_day(
        a in 0u64..4_200_000_000_000,
        step in 0u64..200_000_000,
        offset in -50_400i32..=50_400
    ) {
        proptest::prop_assert!(Day::of(at(a), offset) <= Day::of(at(a + step), offset));
    }

    #[test]
    fn a_day_always_has_a_valid_month_and_day_number(
        ms in 0u64..4_200_000_000_000,
        offset in -50_400i32..=50_400
    ) {
        let d = Day::of(at(ms), offset);
        proptest::prop_assert!((1..=12).contains(&d.month));
        proptest::prop_assert!((1..=31).contains(&d.day));
    }
}
