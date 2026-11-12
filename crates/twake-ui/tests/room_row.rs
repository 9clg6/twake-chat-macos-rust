//! What a room row shows: title, preview, time, unread badge, avatar, pin and mute marks.
//!
//! Need (not the code): `docs/spec/features/roomlist.md`
//! - D31 / D190: under the name, the author of the last message in a room of several people ("You"
//!   for the user); time at the right: the hour today, a day of the week this week, a short date before.
//! - D175: no "You" in a direct row. D170: a space row has no preview.
//! - D608 / D615 / D222 / D187: the unread count at the end of the row, "99+" at most, "@" alone on a
//!   mention, grey when muted, a dot for a mark without count; while it shows, the time is hidden.
//! - D247: pin = favourite. D86: an avatar draws initials; the colour is the same for the same room.
//!
//! Public items used are listed in `docs/dev/ui-screens-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

mod common;

use common::{at, en, en_at, fr, now};
use twake_domain::read_state::UnreadCount;
use twake_domain::room_list::{LastSender, RoomKind, RoomSummary};
use twake_domain::{RoomId, Timestamp};
use twake_ui::avatar::{AVATAR_COLOR_COUNT, AvatarView};
use twake_ui::room_row::{BadgeStyle, RoomRowView};

fn room_id(n: usize) -> RoomId {
    RoomId::parse(&format!("!room{n}:example.org")).expect("valid room id")
}

fn room(name: &str, kind: RoomKind) -> RoomSummary {
    RoomSummary::new(room_id(1), name, kind)
}

fn active(ts: Timestamp) -> RoomSummary {
    room("Team", RoomKind::Group).with_last_activity(ts)
}

fn counts(unread: u32, mentions: u32, marked_unread: bool) -> UnreadCount {
    UnreadCount {
        unread,
        mentions,
        marked_unread,
    }
}

fn unread(n: u32) -> RoomSummary {
    room("Team", RoomKind::Group).with_unread(counts(n, 0, false))
}

fn time_of(summary: &RoomSummary) -> Option<String> {
    RoomRowView::from(summary, now(), &en()).time
}

// ------------------------------------------------------------ title

#[test]
fn the_title_is_the_name_of_the_room() {
    let view = RoomRowView::from(&room("Design crew", RoomKind::Group), now(), &en());
    assert_eq!(view.title, "Design crew");
}

// ------------------------------------------------------------ preview

#[test]
fn a_group_preview_names_the_author_of_the_last_message() {
    let summary = room("Team", RoomKind::Group)
        .with_last_message_preview("lunch at noon")
        .with_last_sender(LastSender::Other("Bob".into()));
    let view = RoomRowView::from(&summary, now(), &en());
    assert_eq!(view.preview.as_deref(), Some("Bob: lunch at noon"));
}

#[test]
fn a_group_preview_of_my_own_message_starts_with_you_in_the_language_of_the_reader() {
    let summary = room("Team", RoomKind::Group)
        .with_last_message_preview("on my way")
        .with_last_sender(LastSender::Me);
    let english = RoomRowView::from(&summary, now(), &en());
    let french = RoomRowView::from(&summary, now(), &fr());
    assert_eq!(english.preview.as_deref(), Some("You: on my way"));
    assert_eq!(french.preview.as_deref(), Some("Vous : on my way"));
}

#[test]
fn a_french_preview_uses_the_french_pattern_for_the_author() {
    let summary = room("Team", RoomKind::Group)
        .with_last_message_preview("salut")
        .with_last_sender(LastSender::Other("Zoé".into()));
    let view = RoomRowView::from(&summary, now(), &fr());
    assert_eq!(view.preview.as_deref(), Some("Zoé : salut"));
}

#[test]
fn a_broadcast_preview_names_the_author_too() {
    let summary = room("News", RoomKind::Broadcast)
        .with_last_message_preview("release 2")
        .with_last_sender(LastSender::Other("Lea".into()));
    let view = RoomRowView::from(&summary, now(), &en());
    assert_eq!(view.preview.as_deref(), Some("Lea: release 2"));
}

#[test]
fn a_direct_preview_is_the_text_alone_whoever_wrote_it() {
    let mine = room("Bob", RoomKind::Direct)
        .with_last_message_preview("see you")
        .with_last_sender(LastSender::Me);
    let theirs = room("Bob", RoomKind::Direct)
        .with_last_message_preview("ok then")
        .with_last_sender(LastSender::Other("Bob".into()));
    assert_eq!(
        RoomRowView::from(&mine, now(), &en()).preview.as_deref(),
        Some("see you")
    );
    assert_eq!(
        RoomRowView::from(&theirs, now(), &en()).preview.as_deref(),
        Some("ok then")
    );
}

#[test]
fn a_group_preview_with_an_unknown_author_is_the_text_alone() {
    let summary = room("Team", RoomKind::Group).with_last_message_preview("hello");
    let view = RoomRowView::from(&summary, now(), &en());
    assert_eq!(view.preview.as_deref(), Some("hello"));
}

#[test]
fn a_room_without_message_has_no_preview() {
    let view = RoomRowView::from(&room("Team", RoomKind::Group), now(), &en());
    assert_eq!(view.preview, None);
}

#[test]
fn a_space_has_no_preview() {
    let summary = room("Company", RoomKind::Space)
        .with_last_message_preview("hidden")
        .with_last_sender(LastSender::Me);
    let view = RoomRowView::from(&summary, now(), &en());
    assert_eq!(view.preview, None);
}

// ------------------------------------------------------------ time

#[test]
fn today_shows_the_hour_with_zero_padding() {
    assert_eq!(
        time_of(&active(at(2026, 11, 12, 9, 5))).as_deref(),
        Some("09:05")
    );
}

#[test]
fn midnight_starts_a_new_day_for_the_row() {
    assert_eq!(
        time_of(&active(at(2026, 11, 12, 0, 0))).as_deref(),
        Some("00:00")
    );
    assert_eq!(
        time_of(&active(at(2026, 11, 11, 23, 59))).as_deref(),
        Some("Yesterday")
    );
}

#[test]
fn yesterday_says_so_in_the_language_of_the_reader() {
    let summary = active(at(2026, 11, 11, 8, 0));
    assert_eq!(
        RoomRowView::from(&summary, now(), &en()).time.as_deref(),
        Some("Yesterday")
    );
    assert_eq!(
        RoomRowView::from(&summary, now(), &fr()).time.as_deref(),
        Some("Hier")
    );
}

#[test]
fn two_days_ago_shows_the_day_of_the_week() {
    let summary = active(at(2026, 11, 10, 8, 0));
    assert_eq!(
        RoomRowView::from(&summary, now(), &en()).time.as_deref(),
        Some("Tue")
    );
    assert_eq!(
        RoomRowView::from(&summary, now(), &fr()).time.as_deref(),
        Some("mar.")
    );
}

#[test]
fn six_days_ago_still_shows_the_day_of_the_week() {
    assert_eq!(
        time_of(&active(at(2026, 11, 6, 8, 0))).as_deref(),
        Some("Fri")
    );
}

#[test]
fn seven_days_ago_shows_the_short_date_in_the_order_of_the_language() {
    let summary = active(at(2026, 11, 5, 8, 0));
    assert_eq!(
        RoomRowView::from(&summary, now(), &en()).time.as_deref(),
        Some("11/05/26")
    );
    assert_eq!(
        RoomRowView::from(&summary, now(), &fr()).time.as_deref(),
        Some("05/11/26")
    );
}

#[test]
fn a_day_in_the_previous_month_is_yesterday_after_the_first_of_the_month() {
    let first_of_march = at(2026, 3, 1, 10, 0);
    let summary = active(at(2026, 2, 28, 12, 0));
    assert_eq!(
        RoomRowView::from(&summary, first_of_march, &en())
            .time
            .as_deref(),
        Some("Yesterday")
    );
}

#[test]
fn the_leap_day_is_yesterday_of_the_first_of_march() {
    let first_of_march = at(2024, 3, 1, 10, 0);
    let summary = active(at(2024, 2, 29, 12, 0));
    assert_eq!(
        RoomRowView::from(&summary, first_of_march, &en())
            .time
            .as_deref(),
        Some("Yesterday")
    );
}

#[test]
fn the_last_day_of_the_year_is_yesterday_of_new_year() {
    let new_year = at(2027, 1, 1, 10, 0);
    let summary = active(at(2026, 12, 31, 23, 0));
    assert_eq!(
        RoomRowView::from(&summary, new_year, &en()).time.as_deref(),
        Some("Yesterday")
    );
}

#[test]
fn the_day_of_the_week_crosses_a_month_boundary() {
    let monday = at(2026, 3, 2, 10, 0);
    let summary = active(at(2026, 2, 28, 12, 0));
    assert_eq!(
        RoomRowView::from(&summary, monday, &en()).time.as_deref(),
        Some("Sat")
    );
}

#[test]
fn the_day_is_the_one_of_the_reader_not_of_utc() {
    let summary = active(at(2026, 11, 12, 23, 30));
    let tomorrow_morning = at(2026, 11, 13, 8, 0);
    let in_utc = RoomRowView::from(&summary, tomorrow_morning, &en());
    let two_hours_east = RoomRowView::from(&summary, tomorrow_morning, &en_at(2));
    assert_eq!(in_utc.time.as_deref(), Some("Yesterday"));
    assert_eq!(two_hours_east.time.as_deref(), Some("01:30"));
}

#[test]
fn the_twelve_hour_clock_shows_a_period() {
    let locale = en().with_hour_cycle(twake_i18n::HourCycle::H12);
    let time = |h, m| RoomRowView::from(&active(at(2026, 11, 12, h, m)), now(), &locale).time;
    assert_eq!(time(9, 5).as_deref(), Some("9:05 AM"));
    assert_eq!(time(15, 30).as_deref(), Some("3:30 PM"));
    assert_eq!(time(0, 5).as_deref(), Some("12:05 AM"));
    assert_eq!(time(12, 0).as_deref(), Some("12:00 PM"));
}

// ------------------------------------------------------------ unread badge

fn badge_text(summary: &RoomSummary) -> Option<String> {
    RoomRowView::from(summary, now(), &en())
        .badge
        .map(|b| b.text)
}

#[test]
fn a_read_room_has_no_badge() {
    let view = RoomRowView::from(&unread(0), now(), &en());
    assert_eq!(view.badge, None);
}

#[test]
fn the_badge_shows_the_count() {
    assert_eq!(badge_text(&unread(7)).as_deref(), Some("7"));
}

#[test]
fn the_badge_shows_ninety_nine_but_caps_the_next_one() {
    assert_eq!(badge_text(&unread(99)).as_deref(), Some("99"));
    assert_eq!(badge_text(&unread(100)).as_deref(), Some("99+"));
    assert_eq!(badge_text(&unread(4_000)).as_deref(), Some("99+"));
}

#[test]
fn a_mention_shows_an_at_sign_alone_with_the_mention_style() {
    let summary = room("Team", RoomKind::Group).with_unread(counts(12, 2, false));
    let badge = RoomRowView::from(&summary, now(), &en())
        .badge
        .expect("a badge");
    assert_eq!(badge.text, "@");
    assert_eq!(badge.style, BadgeStyle::Mention);
}

#[test]
fn an_ordinary_count_has_the_normal_style() {
    let badge = RoomRowView::from(&unread(3), now(), &en())
        .badge
        .expect("a badge");
    assert_eq!(badge.style, BadgeStyle::Normal);
}

#[test]
fn a_muted_room_keeps_its_count_in_grey() {
    let summary = unread(5).muted();
    let badge = RoomRowView::from(&summary, now(), &en())
        .badge
        .expect("a badge");
    assert_eq!((badge.text.as_str(), badge.style), ("5", BadgeStyle::Muted));
}

#[test]
fn a_muted_room_with_a_mention_is_grey_too() {
    let summary = room("Team", RoomKind::Group)
        .with_unread(counts(5, 1, false))
        .muted();
    let badge = RoomRowView::from(&summary, now(), &en())
        .badge
        .expect("a badge");
    assert_eq!((badge.text.as_str(), badge.style), ("@", BadgeStyle::Muted));
}

#[test]
fn a_room_marked_unread_shows_a_dot() {
    let summary = room("Team", RoomKind::Group).with_unread(counts(0, 0, true));
    let badge = RoomRowView::from(&summary, now(), &en())
        .badge
        .expect("a badge");
    assert_eq!((badge.text.as_str(), badge.style), ("", BadgeStyle::Normal));
}

#[test]
fn a_mention_without_count_still_shows_the_at_sign() {
    let summary = room("Team", RoomKind::Group).with_unread(counts(0, 1, false));
    assert_eq!(badge_text(&summary).as_deref(), Some("@"));
}

#[test]
fn the_time_is_hidden_while_the_badge_shows_and_back_when_it_goes() {
    let with_badge = unread(2).with_last_activity(at(2026, 11, 12, 9, 5));
    let without = unread(0).with_last_activity(at(2026, 11, 12, 9, 5));
    assert_eq!(RoomRowView::from(&with_badge, now(), &en()).time, None);
    assert_eq!(
        RoomRowView::from(&without, now(), &en()).time.as_deref(),
        Some("09:05")
    );
}

// ------------------------------------------------------------ pin and mute

#[test]
fn a_favourite_room_shows_the_pin() {
    let pinned = RoomRowView::from(&room("Team", RoomKind::Group).favourite(), now(), &en());
    let plain = RoomRowView::from(&room("Team", RoomKind::Group), now(), &en());
    assert!(pinned.is_pinned);
    assert!(!plain.is_pinned);
}

#[test]
fn a_muted_room_shows_the_muted_mark() {
    let muted = RoomRowView::from(&room("Team", RoomKind::Group).muted(), now(), &en());
    let plain = RoomRowView::from(&room("Team", RoomKind::Group), now(), &en());
    assert!(muted.is_muted);
    assert!(!plain.is_muted);
}

#[test]
fn pin_and_mute_are_independent() {
    let summary = room("Team", RoomKind::Group).favourite();
    let view = RoomRowView::from(&summary, now(), &en());
    assert_eq!((view.is_pinned, view.is_muted), (true, false));
}

// ------------------------------------------------------------ avatar

#[test]
fn the_row_avatar_is_the_one_of_its_name_and_id() {
    let view = RoomRowView::from(&room("Alice Martin", RoomKind::Direct), now(), &en());
    assert_eq!(
        view.avatar,
        AvatarView::of("Alice Martin", room_id(1).as_str())
    );
}

#[test]
fn initials_are_the_first_letters_of_the_first_and_last_words() {
    assert_eq!(AvatarView::of("Alice Martin", "a").initials, "AM");
    assert_eq!(AvatarView::of("Jean Pierre Dupont", "a").initials, "JD");
    assert_eq!(AvatarView::of("  marie   curie ", "a").initials, "MC");
}

#[test]
fn a_single_word_gives_its_first_two_letters() {
    assert_eq!(AvatarView::of("alice", "a").initials, "AL");
    assert_eq!(AvatarView::of("Élodie", "a").initials, "ÉL");
    assert_eq!(AvatarView::of("x", "a").initials, "X");
}

#[test]
fn an_emoji_word_gives_no_initial_while_a_letter_word_remains() {
    assert_eq!(AvatarView::of("🍕 Pizza Club", "a").initials, "PC");
    assert_eq!(AvatarView::of("Team lunch 🍕", "a").initials, "TL");
}

#[test]
fn a_matrix_id_reads_as_its_words() {
    assert_eq!(AvatarView::of("@john.doe:example.org", "a").initials, "JD");
}

#[test]
fn an_empty_name_has_no_initials() {
    assert_eq!(AvatarView::of("", "a").initials, "");
}

#[test]
fn the_colour_is_a_stable_index_of_the_palette() {
    let first = AvatarView::of("Team", "!stable:example.org").color_index;
    let again = AvatarView::of("Team", "!stable:example.org").color_index;
    assert_eq!(first, again);
    assert!(first < AVATAR_COLOR_COUNT);
}

#[test]
fn the_colour_does_not_change_when_the_room_is_renamed() {
    let before = AvatarView::of("Old name", "!stable:example.org").color_index;
    let after = AvatarView::of("A different name", "!stable:example.org").color_index;
    assert_eq!(before, after);
}

#[test]
fn the_palette_has_eleven_colours() {
    assert_eq!(AVATAR_COLOR_COUNT, 11);
}

#[test]
fn many_ids_use_most_of_the_palette_and_never_leave_it() {
    let indexes: Vec<usize> = (0..200)
        .map(|n| AvatarView::of("Team", &format!("!room{n}:example.org")).color_index)
        .collect();
    let distinct: std::collections::HashSet<usize> = indexes.iter().copied().collect();
    assert!(indexes.iter().all(|i| *i < AVATAR_COLOR_COUNT));
    assert!(distinct.len() >= 8, "only {} colours used", distinct.len());
}
