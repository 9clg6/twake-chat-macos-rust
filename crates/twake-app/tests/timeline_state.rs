//! `TimelineState`: what the conversation shows, from diffs of the port.
//!
//! Need (not the code): `docs/spec/features/timeline.md`
//! - e2e "day-divider": a divider per calendar day of the reader.
//! - D495 + e2e "send-echo", "send-clock": a sent message shows at once (local echo, sending),
//!   then the server event takes its place: one bubble, never two.
//! - e2e "message-failed": a failed message stays, can be retried, or discarded (it vanishes).
//! - e2e "others-edits": an edit shows at once, marked "(edited)".
//! - D523 + e2e "new-messages": a "New messages" separator before the first unread message.
//! - D532 / D605: older pages are prepended without a gap; the start of history is known.
//! - Screens: loading, empty, failed with a way out.
//!
//! Public items used are listed in `docs/dev/app-core-api-sketch.md`.

#![cfg(test)]

use proptest::prelude::*;
use twake_app::ports::TimelineError;
use twake_app::timeline::{TimelineState, TimelineStatus};
use twake_domain::timeline::{Message, MessageKey, SendState, TimelineDiff, TimelineItem};
use twake_domain::{Day, EventId, Timestamp, TxnId, UserId};

// ------------------------------------------------------------ helpers

/// 2023-11-14 22:13:20 UTC.
const T0: u64 = 1_700_000_000_000;
const DAY_MS: u64 = 86_400_000;

fn event(n: usize) -> EventId {
    EventId::parse(&format!("$event{n}")).expect("valid event id")
}

fn txn(n: usize) -> TxnId {
    TxnId::parse(&format!("txn{n}")).expect("valid txn id")
}

fn bob() -> UserId {
    UserId::parse("@bob:example.org").expect("valid user id")
}

fn me() -> UserId {
    UserId::parse("@me:example.org").expect("valid user id")
}

/// A server message of Bob, `n` seconds after T0, whose body is `m<n>`.
fn msg(n: usize) -> Message {
    Message::remote(
        event(n),
        bob(),
        Timestamp::from_millis(T0 + n as u64 * 1_000),
        format!("m{n}"),
    )
}

/// The same, `days` days later.
fn msg_on(n: usize, days: u64) -> Message {
    Message::remote(
        event(n),
        bob(),
        Timestamp::from_millis(T0 + days * DAY_MS + n as u64 * 1_000),
        format!("m{n}"),
    )
}

fn echo(t: usize, body: &str) -> Message {
    Message::local(
        txn(t),
        me(),
        Timestamp::from_millis(T0 + 500_000 + t as u64),
        body,
    )
}

fn confirmed(e: usize, t: usize, body: &str) -> Message {
    Message::remote(
        event(e),
        me(),
        Timestamp::from_millis(T0 + 600_000 + t as u64),
        body,
    )
    .with_txn_id(txn(t))
}

/// One short string per row: the body of a message, `NEW`, or `D yyyy-mm-dd`.
fn shape(state: &TimelineState) -> Vec<String> {
    state
        .items()
        .into_iter()
        .map(|item| match item {
            TimelineItem::Message(m) => m.body,
            TimelineItem::NewMessages => "NEW".to_string(),
            TimelineItem::DayDivider(d) => format!("D {}-{:02}-{:02}", d.year, d.month, d.day),
        })
        .collect()
}

fn messages(state: &TimelineState) -> Vec<Message> {
    state
        .items()
        .into_iter()
        .filter_map(|item| match item {
            TimelineItem::Message(m) => Some(m),
            _ => None,
        })
        .collect()
}

fn loaded(list: Vec<Message>) -> TimelineState {
    let mut state = TimelineState::new(0);
    state.apply(TimelineDiff::Reset(list));
    state
}

// ------------------------------------------------------------ start and status

#[test]
fn a_new_timeline_is_loading_and_shows_nothing() {
    let state = TimelineState::new(0);
    assert_eq!(state.status(), TimelineStatus::Loading);
    assert!(state.items().is_empty());
    assert!(!state.reached_start());
}

#[test]
fn a_reset_to_no_message_is_the_empty_conversation() {
    let state = loaded(vec![]);
    assert_eq!(state.status(), TimelineStatus::Empty);
    assert!(state.items().is_empty());
}

#[test]
fn a_reset_with_messages_is_ready() {
    let state = loaded(vec![msg(1), msg(2)]);
    assert_eq!(state.status(), TimelineStatus::Ready);
}

#[test]
fn a_conversation_that_gets_its_first_message_is_no_longer_empty() {
    let mut state = loaded(vec![]);
    state.apply(TimelineDiff::Append(vec![msg(1)]));
    assert_eq!(state.status(), TimelineStatus::Ready);
}

#[test]
fn a_failure_is_shown_and_a_reset_recovers() {
    let mut state = TimelineState::new(0);
    state.fail(TimelineError::Unavailable);
    assert_eq!(
        state.status(),
        TimelineStatus::Failed(TimelineError::Unavailable)
    );
    state.apply(TimelineDiff::Reset(vec![msg(1)]));
    assert_eq!(state.status(), TimelineStatus::Ready);
}

#[test]
fn a_failure_keeps_the_messages_already_shown() {
    let mut state = loaded(vec![msg(1), msg(2)]);
    state.fail(TimelineError::RoomNotFound);
    assert_eq!(
        state.status(),
        TimelineStatus::Failed(TimelineError::RoomNotFound)
    );
    assert_eq!(messages(&state).len(), 2);
}

// ------------------------------------------------------------ day dividers

#[test]
fn a_divider_opens_the_first_day() {
    let state = loaded(vec![msg(1), msg(2)]);
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m1", "m2"]);
}

#[test]
fn a_new_day_gets_its_own_divider() {
    let state = loaded(vec![
        msg(1),
        msg(2),
        msg_on(3, 1),
        msg_on(4, 1),
        msg_on(5, 3),
    ]);
    assert_eq!(
        shape(&state),
        vec![
            "D 2023-11-14",
            "m1",
            "m2",
            "D 2023-11-15",
            "m3",
            "m4",
            "D 2023-11-17",
            "m5"
        ]
    );
}

#[test]
fn the_divider_day_follows_the_offset_of_the_reader() {
    // m1 at 22:13:21 UTC and m2 at 22:13:22 UTC; with +3 h they fall on the 15th at 01:13.
    let mut state = TimelineState::new(3 * 3600);
    state.apply(TimelineDiff::Reset(vec![msg(1), msg(2)]));
    assert_eq!(shape(&state), vec!["D 2023-11-15", "m1", "m2"]);
}

#[test]
fn an_offset_can_split_what_utc_keeps_in_one_day() {
    // 22:13:21 UTC and 23:03:20 UTC are on the 14th; with +1 h the second is 00:03 on the 15th.
    let first = msg(1);
    let second = Message::remote(
        event(2),
        bob(),
        Timestamp::from_millis(T0 + 3_000_000),
        "m2",
    );
    let mut utc = TimelineState::new(0);
    utc.apply(TimelineDiff::Reset(vec![first.clone(), second.clone()]));
    assert_eq!(shape(&utc), vec!["D 2023-11-14", "m1", "m2"]);
    let mut east = TimelineState::new(3600);
    east.apply(TimelineDiff::Reset(vec![first, second]));
    assert_eq!(
        shape(&east),
        vec!["D 2023-11-14", "m1", "D 2023-11-15", "m2"]
    );
}

#[test]
fn the_divider_carries_the_calendar_day() {
    let state = loaded(vec![msg(1)]);
    assert_eq!(
        state.items()[0],
        TimelineItem::DayDivider(Day {
            year: 2023,
            month: 11,
            day: 14
        })
    );
}

// ------------------------------------------------------------ append

#[test]
fn appended_messages_go_after_the_others_in_order() {
    let mut state = loaded(vec![msg(1)]);
    state.apply(TimelineDiff::Append(vec![msg(2), msg(3)]));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m1", "m2", "m3"]);
}

#[test]
fn an_appended_message_of_the_next_day_opens_a_divider() {
    let mut state = loaded(vec![msg(1)]);
    state.apply(TimelineDiff::Append(vec![msg_on(2, 1)]));
    assert_eq!(
        shape(&state),
        vec!["D 2023-11-14", "m1", "D 2023-11-15", "m2"]
    );
}

#[test]
fn a_message_received_twice_is_shown_once() {
    let mut state = loaded(vec![msg(1), msg(2)]);
    state.apply(TimelineDiff::Append(vec![msg(2)]));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m1", "m2"]);
}

// ------------------------------------------------------------ local echo (D495)

#[test]
fn a_local_echo_shows_at_the_end_as_sending() {
    let mut state = loaded(vec![msg(1)]);
    state.apply(TimelineDiff::Append(vec![echo(1, "hello")]));
    let list = messages(&state);
    assert_eq!(list.len(), 2);
    assert_eq!(list[1].body, "hello");
    assert_eq!(list[1].send_state, SendState::Sending);
}

#[test]
fn the_server_event_takes_the_place_of_its_echo() {
    let mut state = loaded(vec![msg(1)]);
    state.apply(TimelineDiff::Append(vec![echo(1, "hello")]));
    state.apply(TimelineDiff::Append(vec![confirmed(40, 1, "hello")]));
    let list = messages(&state);
    assert_eq!(list.len(), 2, "one bubble, not two");
    assert_eq!(list[1].event_id(), Some(&event(40)));
    assert_eq!(list[1].send_state, SendState::Sent);
    assert_eq!(list[1].body, "hello");
}

#[test]
fn the_echo_is_replaced_in_the_same_row() {
    let mut state = loaded(vec![msg(1)]);
    state.apply(TimelineDiff::Append(vec![echo(2, "hello")]));
    let before = shape(&state);
    state.apply(TimelineDiff::Append(vec![confirmed(41, 2, "hello")]));
    assert_eq!(shape(&state), before);
}

#[test]
fn only_the_echo_with_the_same_transaction_is_replaced() {
    let mut state = loaded(vec![msg(1)]);
    state.apply(TimelineDiff::Append(vec![
        echo(1, "first"),
        echo(2, "second"),
    ]));
    state.apply(TimelineDiff::Append(vec![confirmed(41, 2, "second")]));
    let list = messages(&state);
    assert_eq!(list.len(), 3);
    assert_eq!(list[1].send_state, SendState::Sending);
    assert_eq!(list[1].body, "first");
    assert_eq!(list[2].send_state, SendState::Sent);
    assert_eq!(list[2].body, "second");
}

#[test]
fn a_server_event_without_echo_is_a_new_message() {
    let mut state = loaded(vec![msg(1)]);
    state.apply(TimelineDiff::Append(vec![confirmed(
        40,
        9,
        "sent from my phone",
    )]));
    assert_eq!(messages(&state).len(), 2);
}

#[test]
fn a_failed_echo_stays_and_says_so() {
    let mut state = loaded(vec![msg(1)]);
    state.apply(TimelineDiff::Append(vec![echo(1, "hello")]));
    state.apply(TimelineDiff::Update(echo(1, "hello").failed()));
    let list = messages(&state);
    assert_eq!(list.len(), 2);
    assert_eq!(list[1].send_state, SendState::Failed);
}

#[test]
fn a_retried_echo_is_sending_again() {
    let mut state = loaded(vec![msg(1)]);
    state.apply(TimelineDiff::Append(vec![echo(1, "hello").failed()]));
    state.apply(TimelineDiff::Update(echo(1, "hello")));
    assert_eq!(messages(&state)[1].send_state, SendState::Sending);
}

#[test]
fn a_discarded_echo_vanishes() {
    let mut state = loaded(vec![msg(1)]);
    state.apply(TimelineDiff::Append(vec![echo(1, "hello").failed()]));
    state.apply(TimelineDiff::Remove(MessageKey::Txn(txn(1))));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m1"]);
}

// ------------------------------------------------------------ update and remove

#[test]
fn an_edit_replaces_the_message_in_place_and_marks_it() {
    let mut state = loaded(vec![msg(1), msg(2), msg(3)]);
    let edited = Message::remote(
        event(2),
        bob(),
        Timestamp::from_millis(T0 + 2_000),
        "m2 fixed",
    )
    .edited();
    state.apply(TimelineDiff::Update(edited));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m1", "m2 fixed", "m3"]);
    assert!(messages(&state)[1].edited);
    assert!(!messages(&state)[0].edited);
}

#[test]
fn an_update_of_an_unknown_message_is_ignored() {
    let mut state = loaded(vec![msg(1)]);
    state.apply(TimelineDiff::Update(msg(9)));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m1"]);
}

#[test]
fn removing_by_event_drops_that_message_only() {
    let mut state = loaded(vec![msg(1), msg(2), msg(3)]);
    state.apply(TimelineDiff::Remove(MessageKey::Event(event(2))));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m1", "m3"]);
}

#[test]
fn removing_an_unknown_message_changes_nothing() {
    let mut state = loaded(vec![msg(1)]);
    state.apply(TimelineDiff::Remove(MessageKey::Event(event(9))));
    state.apply(TimelineDiff::Remove(MessageKey::Txn(txn(9))));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m1"]);
}

#[test]
fn removing_the_last_message_of_a_day_removes_its_divider() {
    let mut state = loaded(vec![msg(1), msg_on(2, 1)]);
    state.apply(TimelineDiff::Remove(MessageKey::Event(event(2))));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m1"]);
}

#[test]
fn removing_every_message_leaves_the_conversation_empty() {
    let mut state = loaded(vec![msg(1)]);
    state.apply(TimelineDiff::Remove(MessageKey::Event(event(1))));
    assert!(state.items().is_empty());
    assert_eq!(state.status(), TimelineStatus::Empty);
}

// ------------------------------------------------------------ pagination back (D532, D605)

#[test]
fn an_older_page_goes_before_the_first_message_in_order() {
    let mut state = loaded(vec![msg(5), msg(6)]);
    state.apply(TimelineDiff::Prepend {
        messages: vec![msg(3), msg(4)],
        reached_start: false,
    });
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m3", "m4", "m5", "m6"]);
}

#[test]
fn an_older_page_of_the_same_day_does_not_double_the_divider() {
    let mut state = loaded(vec![msg(5)]);
    state.apply(TimelineDiff::Prepend {
        messages: vec![msg(4)],
        reached_start: false,
    });
    let dividers = shape(&state).iter().filter(|r| r.starts_with("D ")).count();
    assert_eq!(dividers, 1);
}

#[test]
fn an_older_page_of_an_earlier_day_gets_its_divider_and_the_old_one_stays() {
    let mut state = loaded(vec![msg_on(5, 1)]);
    state.apply(TimelineDiff::Prepend {
        messages: vec![msg(4)],
        reached_start: false,
    });
    assert_eq!(
        shape(&state),
        vec!["D 2023-11-14", "m4", "D 2023-11-15", "m5"]
    );
}

#[test]
fn an_older_page_never_repeats_a_message_already_held() {
    let mut state = loaded(vec![msg(4), msg(5)]);
    state.apply(TimelineDiff::Prepend {
        messages: vec![msg(3), msg(4)],
        reached_start: false,
    });
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m3", "m4", "m5"]);
}

#[test]
fn the_start_of_history_is_known_once_a_page_says_so() {
    let mut state = loaded(vec![msg(5)]);
    assert!(!state.reached_start());
    state.apply(TimelineDiff::Prepend {
        messages: vec![msg(4)],
        reached_start: false,
    });
    assert!(!state.reached_start());
    state.apply(TimelineDiff::Prepend {
        messages: vec![msg(3)],
        reached_start: true,
    });
    assert!(state.reached_start());
}

#[test]
fn an_empty_last_page_can_still_mark_the_start() {
    let mut state = loaded(vec![msg(5)]);
    state.apply(TimelineDiff::Prepend {
        messages: vec![],
        reached_start: true,
    });
    assert!(state.reached_start());
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m5"]);
}

#[test]
fn new_messages_do_not_hide_the_start_of_history() {
    let mut state = loaded(vec![msg(5)]);
    state.apply(TimelineDiff::Prepend {
        messages: vec![],
        reached_start: true,
    });
    state.apply(TimelineDiff::Append(vec![msg(6)]));
    assert!(state.reached_start());
}

#[test]
fn a_reset_forgets_the_start_of_history() {
    let mut state = loaded(vec![msg(5)]);
    state.apply(TimelineDiff::Prepend {
        messages: vec![],
        reached_start: true,
    });
    state.apply(TimelineDiff::Reset(vec![msg(8)]));
    assert!(!state.reached_start());
}

#[test]
fn a_reset_that_repeats_an_event_keeps_it_once_with_the_last_content() {
    let fixed = Message::remote(
        event(2),
        bob(),
        Timestamp::from_millis(T0 + 2_000),
        "m2 again",
    );
    let state = loaded(vec![msg(1), msg(2), fixed]);
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m1", "m2 again"]);
}

#[test]
fn an_older_page_that_repeats_an_event_shows_it_once() {
    let mut state = loaded(vec![msg(5)]);
    state.apply(TimelineDiff::Prepend {
        messages: vec![msg(3), msg(4), msg(3)],
        reached_start: false,
    });
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m3", "m4", "m5"]);
}

#[test]
fn a_reset_replaces_every_message() {
    let mut state = loaded(vec![msg(1), msg(2)]);
    state.apply(TimelineDiff::Reset(vec![msg(7)]));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m7"]);
}

// ------------------------------------------------------------ new messages marker (D523)

#[test]
fn the_new_messages_marker_sits_before_the_first_unread_message() {
    let mut state = loaded(vec![msg(1), msg(2), msg(3)]);
    state.mark_new_messages_from(event(2));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m1", "NEW", "m2", "m3"]);
}

#[test]
fn the_marker_comes_after_the_day_divider_of_that_message() {
    let mut state = loaded(vec![msg(1), msg_on(2, 1)]);
    state.mark_new_messages_from(event(2));
    assert_eq!(
        shape(&state),
        vec!["D 2023-11-14", "m1", "D 2023-11-15", "NEW", "m2"]
    );
}

#[test]
fn a_marker_on_an_unknown_event_shows_nothing() {
    let mut state = loaded(vec![msg(1), msg(2)]);
    state.mark_new_messages_from(event(9));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m1", "m2"]);
}

#[test]
fn the_marker_stays_when_messages_arrive_and_pages_load() {
    let mut state = loaded(vec![msg(3), msg(4)]);
    state.mark_new_messages_from(event(4));
    state.apply(TimelineDiff::Append(vec![msg(5)]));
    state.apply(TimelineDiff::Prepend {
        messages: vec![msg(2)],
        reached_start: false,
    });
    assert_eq!(
        shape(&state),
        vec!["D 2023-11-14", "m2", "m3", "NEW", "m4", "m5"]
    );
}

#[test]
fn the_marker_waits_for_a_message_not_loaded_yet() {
    let mut state = loaded(vec![msg(3)]);
    state.mark_new_messages_from(event(4));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m3"]);
    state.apply(TimelineDiff::Append(vec![msg(4)]));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m3", "NEW", "m4"]);
}

#[test]
fn a_reset_removes_the_marker() {
    let mut state = loaded(vec![msg(1), msg(2)]);
    state.mark_new_messages_from(event(2));
    state.apply(TimelineDiff::Reset(vec![msg(1), msg(2)]));
    assert_eq!(shape(&state), vec!["D 2023-11-14", "m1", "m2"]);
}

// ------------------------------------------------------------ properties

fn any_message() -> impl Strategy<Value = Message> {
    (0usize..6, 0u64..3).prop_map(|(n, days)| msg_on(n, days))
}

fn any_diff() -> impl Strategy<Value = TimelineDiff> {
    prop_oneof![
        prop::collection::vec(any_message(), 0..5).prop_map(TimelineDiff::Reset),
        prop::collection::vec(any_message(), 0..5).prop_map(TimelineDiff::Append),
        (prop::collection::vec(any_message(), 0..5), any::<bool>()).prop_map(
            |(messages, reached_start)| TimelineDiff::Prepend {
                messages,
                reached_start
            }
        ),
        any_message().prop_map(TimelineDiff::Update),
        (0usize..6).prop_map(|n| TimelineDiff::Remove(MessageKey::Event(event(n)))),
    ]
}

proptest! {
    #[test]
    fn whatever_the_diffs_no_event_is_shown_twice(diffs in prop::collection::vec(any_diff(), 0..10)) {
        let mut state = TimelineState::new(0);
        for d in diffs { state.apply(d); }
        let mut seen = std::collections::HashSet::new();
        for m in messages(&state) {
            if let Some(e) = m.event_id() {
                prop_assert!(seen.insert(e.clone()), "event {:?} shown twice", e);
            }
        }
    }

    #[test]
    fn whatever_the_diffs_a_divider_opens_each_day_and_only_then(
        diffs in prop::collection::vec(any_diff(), 0..10)
    ) {
        let mut state = TimelineState::new(0);
        for d in diffs { state.apply(d); }
        let mut current: Option<Day> = None;
        for item in state.items() {
            match item {
                TimelineItem::DayDivider(d) => {
                    prop_assert_ne!(current, Some(d), "divider repeats the day");
                    current = Some(d);
                }
                TimelineItem::Message(m) => {
                    prop_assert_eq!(current, Some(Day::of(m.timestamp, 0)));
                }
                TimelineItem::NewMessages => {}
            }
        }
    }
}
