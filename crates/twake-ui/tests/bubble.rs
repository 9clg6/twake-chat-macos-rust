//! What a message bubble shows and how consecutive messages group.
//!
//! Need (not the code): `docs/spec/features/timeline.md`
//! - D183 / D297 / D500 / D519: own messages on the right; in a run by one author the name stands above
//!   the first bubble and the avatar at the foot of the last; a direct room shows no name, the avatar on
//!   the last of each run; time inside the bubble.
//! - D495 / D470: a message in flight shows a clock, then one tick (the server has it), then two ticks
//!   once read; someone else's message has no tick. A failed message says "Not sent".
//! - "(edited)" after an edit; D138: a reply quotes the answered message, cut when long.
//! - D523: the day dividers and the "New messages" marker end a run.
//!
//! Public items used are listed in `docs/dev/ui-screens-api-sketch.md` (grouping delay: open question 2).

mod common;

use std::collections::HashMap;

use common::{at, en, en_at, fr, now};
use twake_domain::timeline::{Message, TimelineItem};
use twake_domain::{Day, EventId, Timestamp, TxnId, UserId};
use twake_ui::avatar::AvatarView;
use twake_ui::timeline_rows::{
    BubbleView, GROUP_GAP_MS, QUOTE_MAX_CHARS, RowView, Side, Ticks, TimelineContext, timeline_rows,
};

fn user(local: &str) -> UserId {
    UserId::parse(&format!("@{local}:example.org")).expect("valid user id")
}

fn event(n: usize) -> EventId {
    EventId::parse(&format!("$event{n}")).expect("valid event id")
}

fn names() -> HashMap<UserId, String> {
    HashMap::from([
        (user("alice"), "Alice".to_string()),
        (user("bob"), "Bob".to_string()),
        (user("carol"), "Carol".to_string()),
    ])
}

/// A message of `who` at `ms` milliseconds after 09:00 UTC, 12 November 2026.
fn say(n: usize, who: &str, offset_ms: u64) -> Message {
    let start = at(2026, 11, 12, 9, 0).as_millis();
    Message::remote(
        event(n),
        user(who),
        Timestamp::from_millis(start + offset_ms),
        format!("text {n}"),
    )
}

fn msg(m: Message) -> TimelineItem {
    TimelineItem::Message(m)
}

fn build(items: &[TimelineItem], direct: bool, read_up_to: Option<&EventId>) -> Vec<RowView> {
    let me = user("alice");
    let names = names();
    let ctx = TimelineContext {
        me: &me,
        is_direct: direct,
        names: &names,
        read_by_others_up_to: read_up_to,
    };
    timeline_rows(items, &ctx, now(), &en())
}

fn bubbles(rows: Vec<RowView>) -> Vec<BubbleView> {
    rows.into_iter()
        .filter_map(|row| match row {
            RowView::Bubble(bubble) => Some(bubble),
            _ => None,
        })
        .collect()
}

fn group(items: &[TimelineItem]) -> Vec<BubbleView> {
    bubbles(build(items, false, None))
}

fn name_flags(views: &[BubbleView]) -> Vec<bool> {
    views.iter().map(|b| b.shows_name).collect()
}

fn avatar_flags(views: &[BubbleView]) -> Vec<bool> {
    views.iter().map(|b| b.shows_avatar).collect()
}

fn ticks_of(m: Message) -> Option<Ticks> {
    group(&[msg(m)]).remove(0).ticks
}

// ------------------------------------------------------------ one bubble

#[test]
fn my_message_is_on_the_right_and_the_others_on_the_left() {
    let views = group(&[msg(say(1, "alice", 0)), msg(say(2, "bob", 1_000))]);
    assert_eq!(views[0].side, Side::Own);
    assert_eq!(views[1].side, Side::Other);
}

#[test]
fn the_bubble_shows_the_text_and_the_name_of_its_author() {
    let view = group(&[msg(say(1, "bob", 0))]).remove(0);
    assert_eq!(view.body, "text 1");
    assert_eq!(view.author, "Bob");
}

#[test]
fn an_unknown_author_is_shown_by_its_matrix_id() {
    let view = group(&[msg(say(1, "dave", 0))]).remove(0);
    assert_eq!(view.author, "@dave:example.org");
}

#[test]
fn the_avatar_is_the_one_of_the_author() {
    let view = group(&[msg(say(1, "bob", 0))]).remove(0);
    assert_eq!(view.avatar, AvatarView::of("Bob", "@bob:example.org"));
}

#[test]
fn the_time_is_the_clock_of_the_reader() {
    let at_0905 = msg(say(1, "bob", 5 * 60_000));
    assert_eq!(group(&[at_0905.clone()])[0].time, "09:05");
    let late = Message::remote(event(2), user("bob"), at(2026, 11, 12, 23, 30), "late");
    let rows = timeline_rows(
        &[msg(late)],
        &TimelineContext {
            me: &user("alice"),
            is_direct: false,
            names: &names(),
            read_by_others_up_to: None,
        },
        now(),
        &en_at(2),
    );
    assert_eq!(bubbles(rows)[0].time, "01:30");
}

// ------------------------------------------------------------ runs

#[test]
fn in_a_run_the_name_stands_on_the_first_bubble_and_the_avatar_on_the_last() {
    let items: Vec<_> = (1..=3)
        .map(|n| msg(say(n, "bob", n as u64 * 10_000)))
        .collect();
    let views = group(&items);
    assert_eq!(name_flags(&views), [true, false, false]);
    assert_eq!(avatar_flags(&views), [false, false, true]);
}

#[test]
fn a_run_of_one_keeps_both_the_name_and_the_avatar() {
    let views = group(&[msg(say(1, "bob", 0))]);
    assert_eq!((views[0].shows_name, views[0].shows_avatar), (true, true));
}

#[test]
fn a_message_exactly_at_the_gap_stays_in_the_run() {
    let views = group(&[msg(say(1, "bob", 0)), msg(say(2, "bob", GROUP_GAP_MS))]);
    assert_eq!(name_flags(&views), [true, false]);
}

#[test]
fn a_message_one_millisecond_past_the_gap_starts_a_new_run() {
    let views = group(&[msg(say(1, "bob", 0)), msg(say(2, "bob", GROUP_GAP_MS + 1))]);
    assert_eq!(name_flags(&views), [true, true]);
    assert_eq!(avatar_flags(&views), [true, true]);
}

#[test]
fn the_gap_is_measured_from_the_message_before_not_from_the_head_of_the_run() {
    let step = GROUP_GAP_MS - 1_000;
    let items = [
        msg(say(1, "bob", 0)),
        msg(say(2, "bob", step)),
        msg(say(3, "bob", 2 * step)),
    ];
    assert_eq!(name_flags(&group(&items)), [true, false, false]);
}

#[test]
fn another_author_ends_the_run() {
    let items = [
        msg(say(1, "bob", 0)),
        msg(say(2, "carol", 1_000)),
        msg(say(3, "bob", 2_000)),
    ];
    let views = group(&items);
    assert_eq!(name_flags(&views), [true, true, true]);
    assert_eq!(avatar_flags(&views), [true, true, true]);
}

#[test]
fn a_day_divider_ends_the_run() {
    let divider = TimelineItem::DayDivider(Day {
        year: 2026,
        month: 11,
        day: 12,
    });
    let items = [msg(say(1, "bob", 0)), divider, msg(say(2, "bob", 1_000))];
    let views = group(&items);
    assert_eq!(name_flags(&views), [true, true]);
    assert_eq!(avatar_flags(&views), [true, true]);
}

#[test]
fn the_new_messages_marker_ends_the_run() {
    let items = [
        msg(say(1, "bob", 0)),
        TimelineItem::NewMessages,
        msg(say(2, "bob", 1_000)),
    ];
    assert_eq!(name_flags(&group(&items)), [true, true]);
}

#[test]
fn my_own_messages_never_show_a_name_or_an_avatar() {
    let items = [msg(say(1, "alice", 0)), msg(say(2, "alice", 1_000))];
    let views = group(&items);
    assert_eq!(name_flags(&views), [false, false]);
    assert_eq!(avatar_flags(&views), [false, false]);
}

#[test]
fn in_a_direct_room_the_other_person_has_no_name_but_an_avatar_on_the_last_of_the_run() {
    let items: Vec<_> = (1..=3)
        .map(|n| msg(say(n, "bob", n as u64 * 10_000)))
        .collect();
    let views = bubbles(build(&items, true, None));
    assert_eq!(name_flags(&views), [false, false, false]);
    assert_eq!(avatar_flags(&views), [false, false, true]);
}

// ------------------------------------------------------------ rows

#[test]
fn every_item_gives_one_row_in_the_same_order() {
    let items = [
        TimelineItem::DayDivider(Day {
            year: 2026,
            month: 11,
            day: 12,
        }),
        msg(say(1, "bob", 0)),
        TimelineItem::NewMessages,
        msg(say(2, "alice", 1_000)),
    ];
    let rows = build(&items, false, None);
    assert!(matches!(
        rows.as_slice(),
        [
            RowView::Day(_),
            RowView::Bubble(_),
            RowView::NewMessages(_),
            RowView::Bubble(_)
        ]
    ));
}

#[test]
fn no_item_gives_no_row() {
    assert!(build(&[], false, None).is_empty());
}

// ------------------------------------------------------------ ticks

#[test]
fn a_message_in_flight_shows_a_clock() {
    let echo = Message::local(
        TxnId::parse("txn1").expect("txn"),
        user("alice"),
        now(),
        "hi",
    );
    assert_eq!(ticks_of(echo), Some(Ticks::Clock));
}

#[test]
fn a_message_the_server_holds_shows_one_tick() {
    assert_eq!(ticks_of(say(1, "alice", 0)), Some(Ticks::Sent));
}

#[test]
fn a_failed_message_says_so() {
    assert_eq!(ticks_of(say(1, "alice", 0).failed()), Some(Ticks::Failed));
}

#[test]
fn someone_elses_message_has_no_tick() {
    assert_eq!(ticks_of(say(1, "bob", 0)), None);
}

fn two_own_and_one_from_bob() -> [TimelineItem; 3] {
    [
        msg(say(1, "alice", 0)),
        msg(say(2, "bob", 1_000)),
        msg(say(3, "alice", 2_000)),
    ]
}

#[test]
fn a_message_read_by_someone_shows_two_ticks_and_the_newer_ones_one() {
    let read = event(2);
    let views = bubbles(build(&two_own_and_one_from_bob(), false, Some(&read)));
    assert_eq!(views[0].ticks, Some(Ticks::Read));
    assert_eq!(views[2].ticks, Some(Ticks::Sent));
}

#[test]
fn the_message_a_reader_stopped_at_is_read_too() {
    let read = event(3);
    let views = bubbles(build(&two_own_and_one_from_bob(), false, Some(&read)));
    assert_eq!(
        (views[0].ticks, views[2].ticks),
        (Some(Ticks::Read), Some(Ticks::Read))
    );
}

#[test]
fn without_a_receipt_nothing_is_read() {
    let views = bubbles(build(&two_own_and_one_from_bob(), false, None));
    assert_eq!(
        (views[0].ticks, views[2].ticks),
        (Some(Ticks::Sent), Some(Ticks::Sent))
    );
}

#[test]
fn a_failed_message_before_the_receipt_stays_failed() {
    let items = [msg(say(1, "alice", 0).failed()), msg(say(2, "bob", 1_000))];
    let read = event(2);
    assert_eq!(
        bubbles(build(&items, false, Some(&read)))[0].ticks,
        Some(Ticks::Failed)
    );
}

// ------------------------------------------------------------ edited

#[test]
fn an_edited_message_carries_the_label_in_the_language_of_the_reader() {
    let edited = [msg(say(1, "bob", 0).edited())];
    let ctx_me = user("alice");
    let names = names();
    let ctx = TimelineContext {
        me: &ctx_me,
        is_direct: false,
        names: &names,
        read_by_others_up_to: None,
    };
    let english = bubbles(timeline_rows(&edited, &ctx, now(), &en()));
    let french = bubbles(timeline_rows(&edited, &ctx, now(), &fr()));
    assert_eq!(english[0].edited.as_deref(), Some("(edited)"));
    assert_eq!(french[0].edited.as_deref(), Some("(modifié)"));
}

#[test]
fn a_message_never_edited_has_no_label() {
    assert_eq!(group(&[msg(say(1, "bob", 0))])[0].edited, None);
}

// ------------------------------------------------------------ quote

fn reply_to_bob(text: &str) -> Vec<BubbleView> {
    let original = Message::remote(event(1), user("bob"), at(2026, 11, 12, 9, 0), text);
    let answer = say(2, "alice", 60_000).with_reply_to(event(1));
    group(&[msg(original), msg(answer)])
}

#[test]
fn a_reply_quotes_the_author_and_the_text_of_the_answered_message() {
    let quote = reply_to_bob("Where do we meet?")
        .remove(1)
        .quote
        .expect("a quote");
    assert_eq!(quote.author, "Bob");
    assert_eq!(quote.text, "Where do we meet?");
    assert_eq!(quote.target, event(1));
}

#[test]
fn a_message_that_answers_nothing_has_no_quote() {
    assert_eq!(reply_to_bob("hello")[0].quote, None);
}

#[test]
fn a_quote_of_exactly_the_limit_is_not_cut() {
    let text = "é".repeat(QUOTE_MAX_CHARS);
    let quote = reply_to_bob(&text).remove(1).quote.expect("a quote");
    assert_eq!(quote.text, text);
}

#[test]
fn a_longer_quote_is_cut_to_the_limit_and_ends_with_an_ellipsis() {
    let text = "é".repeat(QUOTE_MAX_CHARS + 1);
    let quote = reply_to_bob(&text).remove(1).quote.expect("a quote");
    assert_eq!(quote.text.chars().count(), QUOTE_MAX_CHARS);
    assert!(quote.text.ends_with('…'));
    assert!(quote.text.starts_with(&"é".repeat(QUOTE_MAX_CHARS - 1)));
}
