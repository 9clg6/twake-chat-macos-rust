//! The day divider and the "New messages" marker.
//!
//! Need (not the code): `docs/spec/features/timeline.md`
//! - e2e "chat": the divider above the messages of today reads exactly "Today".
//! - Activity (D185): Today, Yesterday, then dates.
//! - D523: the marker reads "New messages" (role separator).
//! - The day is the calendar day of the reader; `Day` is the domain's.
//!
//! Public items used are listed in `docs/dev/ui-screens-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use common::{at, en, en_at, fr, now};
use twake_domain::{Day, Timestamp};
use twake_ui::timeline_rows::{
    DayDividerView, NewMessagesView, RowView, TimelineContext, timeline_rows,
};

fn day(year: i32, month: u8, day: u8) -> Day {
    Day { year, month, day }
}

fn label(day: Day, now: Timestamp) -> String {
    DayDividerView::from(day, now, &en()).label
}

#[test]
fn today_reads_today_in_the_language_of_the_reader() {
    let today = day(2026, 11, 12);
    assert_eq!(label(today, now()), "Today");
    assert_eq!(
        DayDividerView::from(today, now(), &fr()).label,
        "Aujourd'hui"
    );
}

#[test]
fn the_day_before_reads_yesterday_in_the_language_of_the_reader() {
    let yesterday = day(2026, 11, 11);
    assert_eq!(label(yesterday, now()), "Yesterday");
    assert_eq!(DayDividerView::from(yesterday, now(), &fr()).label, "Hier");
}

#[test]
fn two_days_before_is_a_date() {
    assert_eq!(label(day(2026, 11, 10), now()), "11/10/26");
}

#[test]
fn a_date_follows_the_order_of_the_language() {
    let view = DayDividerView::from(day(2026, 3, 4), now(), &fr());
    assert_eq!(view.label, "04/03/26");
    assert_eq!(label(day(2026, 3, 4), now()), "03/04/26");
}

#[test]
fn a_date_of_another_year_ends_with_its_two_last_digits() {
    assert_eq!(label(day(2019, 12, 25), now()), "12/25/19");
}

#[test]
fn the_last_day_of_the_previous_month_is_yesterday_on_the_first() {
    assert_eq!(label(day(2026, 2, 28), at(2026, 3, 1, 10, 0)), "Yesterday");
    assert_eq!(label(day(2024, 2, 29), at(2024, 3, 1, 10, 0)), "Yesterday");
}

#[test]
fn the_last_day_of_the_year_is_yesterday_on_new_year() {
    assert_eq!(label(day(2026, 12, 31), at(2027, 1, 1, 0, 30)), "Yesterday");
}

#[test]
fn today_is_the_day_of_the_reader_not_of_utc() {
    let late_in_utc = at(2026, 11, 12, 23, 30);
    let east = DayDividerView::from(day(2026, 11, 13), late_in_utc, &en_at(2));
    let utc = DayDividerView::from(day(2026, 11, 13), late_in_utc, &en());
    assert_eq!(east.label, "Today");
    assert_eq!(utc.label, "11/13/26");
}

#[test]
fn the_new_messages_marker_reads_new_messages_in_the_language_of_the_reader() {
    assert_eq!(NewMessagesView::from(&en()).label, "New messages");
    assert_eq!(NewMessagesView::from(&fr()).label, "Nouveaux messages");
}

#[test]
fn the_timeline_turns_a_divider_item_into_its_label() {
    let me = twake_domain::UserId::parse("@alice:example.org").expect("user id");
    let names = std::collections::HashMap::new();
    let ctx = TimelineContext {
        me: &me,
        is_direct: false,
        names: &names,
        read_by_others_up_to: None,
    };
    let items = [
        twake_domain::timeline::TimelineItem::DayDivider(day(2026, 11, 12)),
        twake_domain::timeline::TimelineItem::NewMessages,
    ];
    let rows = timeline_rows(&items, &ctx, now(), &en());
    match rows.as_slice() {
        [RowView::Day(divider), RowView::NewMessages(marker)] => {
            assert_eq!(divider.label, "Today");
            assert_eq!(marker.label, "New messages");
        }
        other => panic!("unexpected rows: {} rows", other.len()),
    }
}
