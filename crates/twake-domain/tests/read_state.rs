//! Unread counts and read receipts, as pure functions over immutable values.
//!
//! Need (not the code): React `src/domain/readState.ts` + `readState.spec.ts`,
//! and the decisions D60 (muted thread counts nothing), D175 (ignored person
//! counts for nothing), D222 (mentions-only room), D452, D483 + D510 (furthest
//! own receipt wins; a receipt on a message not held counts for nothing, so
//! everything held is unread), D539 (only a message the user wrote reads the
//! room; a state event they send does not), D558/D575 (a receipt never goes
//! back), D615 (marked unread is a flag, never a count), inventory R21, TH4.
//! Public items used are listed in `docs/dev/api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_domain::read_state::{
    NotificationLevel, ReadContext, ReceiptTarget, RoomFlags, RoomReadState, ThreadId,
    TimelineEvent, UnreadCount, choose_receipt_event, receipt_targets, room_unread, silent_unread,
    thread_read_state, timeline_read_state, total_unread,
};
use twake_domain::{EventId, UserId};

// ------------------------------------------------------------ helpers

fn ev(n: usize) -> EventId {
    EventId::parse(&format!("$e{n}")).unwrap()
}
fn user(name: &str) -> UserId {
    UserId::parse(&format!("@{name}:twake.app")).unwrap()
}
fn me() -> UserId {
    user("me")
}
fn bob() -> UserId {
    user("bob")
}
fn carol() -> UserId {
    user("carol")
}
/// A message of `sender`, id `$e<n>`.
fn msg(n: usize, sender: &UserId) -> TimelineEvent {
    TimelineEvent::message(ev(n), sender.clone())
}
/// Something that is not a message (reaction, edit, state event).
fn other(n: usize, sender: &UserId) -> TimelineEvent {
    TimelineEvent::other(ev(n), sender.clone())
}
fn ctx() -> ReadContext {
    ReadContext::new(me())
}
fn counts(unread: u32, mentions: u32) -> UnreadCount {
    UnreadCount {
        unread,
        mentions,
        marked_unread: false,
    }
}
fn flags(level: NotificationLevel, marked_unread: bool) -> RoomFlags {
    RoomFlags {
        notification_level: level,
        marked_unread,
    }
}
fn all() -> RoomFlags {
    flags(NotificationLevel::All, false)
}
fn thread_root(n: usize) -> EventId {
    EventId::parse(&format!("$root{n}")).unwrap()
}

// ------------------------------------------------------------ main timeline

#[test]
fn without_any_receipt_every_message_of_others_is_unread() {
    let events = [msg(1, &bob()), msg(2, &carol()), msg(3, &bob())];
    let state = timeline_read_state(&events, &ctx());
    assert_eq!(state.unread_count, 3);
    assert_eq!(state.mention_count, 0);
}

#[test]
fn an_empty_timeline_has_nothing_unread_and_no_latest_event() {
    let state = timeline_read_state(&[], &ctx());
    assert_eq!(state.unread_count, 0);
    assert_eq!(state.mention_count, 0);
    assert_eq!(state.latest_event_id, None);
}

#[test]
fn own_messages_are_never_unread() {
    let events = [msg(1, &me()), msg(2, &me())];
    assert_eq!(timeline_read_state(&events, &ctx()).unread_count, 0);
}

#[test]
fn writing_reads_everything_before_it_but_not_what_comes_after() {
    let events = [
        msg(1, &bob()),
        msg(2, &bob()),
        msg(3, &me()),
        msg(4, &bob()),
    ];
    // $e1 and $e2 are read by the answer of the user; only $e4 waits.
    assert_eq!(timeline_read_state(&events, &ctx()).unread_count, 1);
}

#[test]
fn a_receipt_moves_the_read_point_to_the_event_it_points_at() {
    let events = [
        msg(1, &bob()),
        msg(2, &bob()),
        msg(3, &bob()),
        msg(4, &bob()),
    ];
    let at_2 = ctx().with_receipts(vec![ev(2)]);
    assert_eq!(timeline_read_state(&events, &at_2).unread_count, 2);
    let at_4 = ctx().with_receipts(vec![ev(4)]);
    assert_eq!(timeline_read_state(&events, &at_4).unread_count, 0);
}

#[test]
fn of_several_own_receipts_the_furthest_counts_not_the_last_given() {
    // D510: public and private receipts; no pick by receipt time.
    let events = [
        msg(1, &bob()),
        msg(2, &bob()),
        msg(3, &bob()),
        msg(4, &bob()),
    ];
    let furthest_first = ctx().with_receipts(vec![ev(3), ev(1)]);
    let furthest_last = ctx().with_receipts(vec![ev(1), ev(3)]);
    assert_eq!(
        timeline_read_state(&events, &furthest_first).unread_count,
        1
    );
    assert_eq!(timeline_read_state(&events, &furthest_last).unread_count, 1);
}

#[test]
fn a_receipt_on_an_event_not_held_counts_for_nothing_so_everything_held_is_unread() {
    // D483 + D510.
    let events = [msg(1, &bob()), msg(2, &bob())];
    let unheld = ctx().with_receipts(vec![ev(99)]);
    assert_eq!(timeline_read_state(&events, &unheld).unread_count, 2);
}

#[test]
fn a_receipt_not_held_does_not_cancel_one_that_is_held() {
    let events = [msg(1, &bob()), msg(2, &bob()), msg(3, &bob())];
    let mixed = ctx().with_receipts(vec![ev(99), ev(2)]);
    assert_eq!(timeline_read_state(&events, &mixed).unread_count, 1);
}

#[test]
fn a_receipt_on_a_reaction_or_an_edit_still_reads_the_messages_before_it() {
    // Another client may put its receipt on an event that is not a message:
    // the window before it must not come back as unread.
    let events = [
        msg(1, &bob()),
        msg(2, &bob()),
        other(3, &bob()),
        msg(4, &bob()),
    ];
    let state = timeline_read_state(&events, &ctx().with_receipts(vec![ev(3)]));
    assert_eq!(state.unread_count, 1);
}

#[test]
fn reactions_edits_and_state_events_of_others_are_not_unread_messages() {
    let events = [other(1, &bob()), other(2, &carol())];
    assert_eq!(timeline_read_state(&events, &ctx()).unread_count, 0);
}

#[test]
fn a_state_event_the_user_sends_does_not_mark_the_room_read() {
    // D539: a rename, a topic or a power-level change by the user is not a read.
    let events = [msg(1, &bob()), other(2, &me()), msg(3, &bob())];
    assert_eq!(timeline_read_state(&events, &ctx()).unread_count, 2);
}

#[test]
fn messages_of_an_ignored_person_count_for_nothing() {
    // D175.
    let events = [msg(1, &bob()), msg(2, &carol()), msg(3, &bob())];
    let ignoring_bob = ctx().ignoring(bob());
    assert_eq!(timeline_read_state(&events, &ignoring_bob).unread_count, 1);
}

#[test]
fn a_mention_counts_as_unread_and_as_a_mention() {
    let events = [
        msg(1, &bob()),
        msg(2, &bob()).mentioning_me(),
        msg(3, &carol()).mentioning_me(),
    ];
    let state = timeline_read_state(&events, &ctx());
    assert_eq!(state.unread_count, 3);
    assert_eq!(state.mention_count, 2);
}

#[test]
fn a_mention_already_read_no_longer_counts() {
    let events = [msg(1, &bob()).mentioning_me(), msg(2, &bob())];
    let state = timeline_read_state(&events, &ctx().with_receipts(vec![ev(1)]));
    assert_eq!(state.unread_count, 1);
    assert_eq!(state.mention_count, 0);
}

#[test]
fn a_mention_by_an_ignored_person_counts_for_nothing() {
    let events = [msg(1, &bob()).mentioning_me()];
    let state = timeline_read_state(&events, &ctx().ignoring(bob()));
    assert_eq!((state.unread_count, state.mention_count), (0, 0));
}

#[test]
fn the_latest_event_is_the_last_message_whoever_wrote_it() {
    // It is where a read receipt goes.
    let own_last = [msg(1, &bob()), msg(2, &me()), other(3, &bob())];
    assert_eq!(
        timeline_read_state(&own_last, &ctx()).latest_event_id,
        Some(ev(2))
    );
    let bob_last = [msg(1, &me()), msg(2, &bob())];
    assert_eq!(
        timeline_read_state(&bob_last, &ctx()).latest_event_id,
        Some(ev(2))
    );
}

// ------------------------------------------------------------ threads

#[test]
fn a_reply_in_a_thread_the_user_follows_is_unread() {
    let replies = [msg(10, &bob())];
    let thread = thread_read_state(thread_root(1), &replies, true, false, &ctx());
    assert_eq!(thread.state.unread_count, 1);
    assert!(thread.is_participating);
}

#[test]
fn an_own_reply_in_a_thread_reads_the_replies_before_it() {
    let replies = [msg(10, &bob()), msg(11, &me()), msg(12, &bob())];
    let thread = thread_read_state(thread_root(1), &replies, true, false, &ctx());
    assert_eq!(thread.state.unread_count, 1);
}

#[test]
fn a_thread_receipt_reads_the_replies_up_to_it() {
    let replies = [msg(10, &bob()), msg(11, &bob()), msg(12, &bob())];
    let c = ctx().with_receipts(vec![ev(11)]);
    let thread = thread_read_state(thread_root(1), &replies, true, false, &c);
    assert_eq!(thread.state.unread_count, 1);
}

#[test]
fn a_muted_thread_counts_nothing_mentions_included() {
    // D60.
    let replies = [msg(10, &bob()).mentioning_me(), msg(11, &bob())];
    let thread = thread_read_state(thread_root(1), &replies, true, true, &ctx());
    assert_eq!(
        (thread.state.unread_count, thread.state.mention_count),
        (0, 0)
    );
    assert!(thread.is_muted);
}

#[test]
fn a_thread_keeps_its_latest_reply_for_the_receipt() {
    let replies = [msg(10, &bob()), msg(11, &me())];
    let thread = thread_read_state(thread_root(7), &replies, true, false, &ctx());
    assert_eq!(thread.state.latest_event_id, Some(ev(11)));
    assert_eq!(thread.root, thread_root(7));
}

// ------------------------------------------------------------ one count per room

fn room(main: &[TimelineEvent], threads: &[(usize, &[TimelineEvent], bool)]) -> RoomReadState {
    let c = ctx();
    RoomReadState {
        main: timeline_read_state(main, &c),
        threads: threads
            .iter()
            .map(|(root, replies, followed)| {
                thread_read_state(thread_root(*root), replies, *followed, false, &c)
            })
            .collect(),
    }
}

#[test]
fn a_room_counts_its_main_timeline_plus_the_threads_the_user_follows() {
    let main = [msg(1, &bob()), msg(2, &bob().clone()).mentioning_me()];
    let a = [msg(10, &bob()), msg(11, &bob()), msg(12, &bob())];
    let b = [msg(20, &carol()).mentioning_me()];
    let state = room(&main, &[(1, &a, true), (2, &b, true)]);
    assert_eq!(room_unread(&state, &all()), counts(6, 2));
}

#[test]
fn a_thread_reply_alone_makes_the_room_unread() {
    let a = [msg(10, &bob())];
    let state = room(&[], &[(1, &a, true)]);
    let unread = room_unread(&state, &all());
    assert_eq!(unread, counts(1, 0));
    assert!(unread.is_unread());
}

#[test]
fn replies_of_a_thread_not_followed_do_not_count() {
    let a = [msg(10, &bob()), msg(11, &bob())];
    let state = room(&[], &[(1, &a, false)]);
    let unread = room_unread(&state, &all());
    assert_eq!(unread, counts(0, 0));
    assert!(!unread.is_unread());
}

#[test]
fn a_mention_counts_even_in_a_thread_not_followed() {
    let a = [msg(10, &bob()).mentioning_me(), msg(11, &bob())];
    let state = room(&[], &[(1, &a, false)]);
    assert_eq!(room_unread(&state, &all()), counts(0, 1));
}

// ------------------------------------------------------------ flags of the user

fn busy_room() -> RoomReadState {
    // 4 unread, 1 of them a mention
    let main = [
        msg(1, &bob()),
        msg(2, &bob()),
        msg(3, &bob()),
        msg(4, &bob()).mentioning_me(),
    ];
    room(&main, &[])
}

#[test]
fn a_mentions_only_room_counts_its_mentions_alone() {
    let level = flags(NotificationLevel::MentionsOnly, false);
    assert_eq!(room_unread(&busy_room(), &level), counts(0, 1));
}

#[test]
fn a_room_that_says_nothing_counts_nothing_mentions_included() {
    let level = flags(NotificationLevel::Nothing, false);
    assert_eq!(room_unread(&busy_room(), &level), counts(0, 0));
}

#[test]
fn a_room_marked_unread_with_nothing_new_shows_a_mark_and_no_count() {
    // D615: a dot, no number.
    let unread = room_unread(&room(&[], &[]), &flags(NotificationLevel::All, true));
    assert_eq!((unread.unread, unread.mentions), (0, 0));
    assert!(unread.marked_unread);
    assert!(unread.is_unread());
}

#[test]
fn a_muted_room_marked_unread_keeps_the_mark_without_a_count() {
    let unread = room_unread(
        &room(&[], &[]),
        &flags(NotificationLevel::MentionsOnly, true),
    );
    assert_eq!(
        (unread.unread, unread.mentions, unread.marked_unread),
        (0, 0, true)
    );
}

#[test]
fn a_room_marked_unread_that_has_real_messages_counts_them_and_needs_no_mark() {
    let unread = room_unread(&busy_room(), &flags(NotificationLevel::All, true));
    assert_eq!((unread.unread, unread.mentions), (4, 1));
    assert!(!unread.marked_unread, "the count says it already");
    assert!(unread.is_unread());
}

#[test]
fn a_room_with_nothing_new_and_no_mark_is_read() {
    assert!(!room_unread(&room(&[], &[]), &all()).is_unread());
}

#[test]
fn totals_add_counts_and_ignore_marks() {
    // The Chat badge and the folded space sum their rooms (D182, D483); a
    // mark is no count (D615).
    let marked = room_unread(&room(&[], &[]), &flags(NotificationLevel::All, true));
    let total = total_unread(&[counts(2, 0), counts(1, 1), marked]);
    assert_eq!((total.unread, total.mentions), (3, 1));
    assert!(!total.marked_unread);
}

#[test]
fn the_total_of_no_room_is_zero() {
    assert_eq!(total_unread(&[]), counts(0, 0));
}

#[test]
fn a_mentions_only_room_keeps_its_messages_in_a_silent_count_for_its_grey_row() {
    // D222: shown in grey on the row, counted by no total.
    assert_eq!(
        silent_unread(&busy_room(), &flags(NotificationLevel::MentionsOnly, false)),
        4
    );
    assert_eq!(silent_unread(&busy_room(), &all()), 0);
    assert_eq!(
        silent_unread(&busy_room(), &flags(NotificationLevel::Nothing, false)),
        0
    );
}

// ------------------------------------------------------------ mark as read

#[test]
fn marking_a_room_read_sends_a_receipt_for_main_and_for_every_unread_thread() {
    let main = [msg(1, &bob()), msg(2, &bob())];
    let a = [msg(10, &bob()), msg(11, &bob())];
    let read_thread = [msg(20, &me())];
    let not_followed = [msg(30, &carol())];
    let state = room(
        &main,
        &[
            (1, &a, true),
            (2, &read_thread, true),
            (3, &not_followed, false),
        ],
    );
    assert_eq!(
        receipt_targets(&state),
        vec![
            ReceiptTarget {
                thread: ThreadId::Main,
                event: ev(2)
            },
            ReceiptTarget {
                thread: ThreadId::Thread(thread_root(1)),
                event: ev(11)
            },
            // Followed or not: no counter may come back later.
            ReceiptTarget {
                thread: ThreadId::Thread(thread_root(3)),
                event: ev(30)
            },
        ]
    );
}

#[test]
fn a_room_with_nothing_unread_has_no_receipt_to_send() {
    let main = [msg(1, &me())];
    assert_eq!(receipt_targets(&room(&main, &[])), vec![]);
    assert_eq!(receipt_targets(&room(&[], &[])), vec![]);
}

#[test]
fn a_room_that_only_has_a_mention_in_a_thread_gets_a_receipt_for_that_thread() {
    let a = [msg(10, &bob()).mentioning_me()];
    let state = room(&[], &[(1, &a, false)]);
    assert_eq!(
        receipt_targets(&state),
        vec![ReceiptTarget {
            thread: ThreadId::Thread(thread_root(1)),
            event: ev(10)
        }]
    );
}

#[test]
fn once_the_receipts_are_applied_the_room_is_read_and_sends_nothing_more() {
    let main = [
        msg(1, &bob()),
        msg(2, &bob()).mentioning_me(),
        other(3, &bob()),
    ];
    let a = [msg(10, &bob()), msg(11, &carol())];
    let b = [msg(20, &carol()).mentioning_me()];
    let before = room(&main, &[(1, &a, true), (2, &b, false)]);
    assert!(room_unread(&before, &all()).is_unread());

    // The server echoes every receipt sent: the next computation sees them.
    let sent: Vec<EventId> = receipt_targets(&before)
        .into_iter()
        .map(|t| t.event)
        .collect();
    let c = ctx().with_receipts(sent);
    let after = RoomReadState {
        main: timeline_read_state(&main, &c),
        threads: vec![
            thread_read_state(thread_root(1), &a, true, false, &c),
            thread_read_state(thread_root(2), &b, false, false, &c),
        ],
    };
    assert!(!room_unread(&after, &all()).is_unread());
    assert_eq!(receipt_targets(&after), vec![]);
}

// ------------------------------------------------------------ what a receipt may point at

#[test]
fn a_shown_unread_message_is_the_one_read_up_to() {
    let ids = [ev(1), ev(2), ev(3), ev(4)];
    assert_eq!(choose_receipt_event(&ids, 2, Some(&ev(3))), Some(ev(3)));
    assert_eq!(choose_receipt_event(&ids, 2, Some(&ev(4))), Some(ev(4)));
    assert_eq!(choose_receipt_event(&ids, 1, Some(&ev(4))), Some(ev(4)));
}

#[test]
fn a_receipt_never_goes_back_to_what_was_read() {
    let ids = [ev(1), ev(2), ev(3), ev(4)];
    assert_eq!(choose_receipt_event(&ids, 2, Some(&ev(2))), None);
}

#[test]
fn nothing_unread_or_nothing_shown_sends_no_receipt() {
    let ids = [ev(1), ev(2)];
    assert_eq!(choose_receipt_event(&ids, 0, Some(&ev(2))), None);
    assert_eq!(choose_receipt_event(&ids, 2, None), None);
}

#[test]
fn more_unread_than_held_makes_every_held_message_eligible() {
    let ids = [ev(1), ev(2), ev(3), ev(4)];
    assert_eq!(choose_receipt_event(&ids, 9, Some(&ev(1))), Some(ev(1)));
}

#[test]
fn a_shown_message_that_is_not_held_sends_no_receipt() {
    let ids = [ev(1), ev(2)];
    assert_eq!(choose_receipt_event(&ids, 9, Some(&ev(99))), None);
}

// ------------------------------------------------------------ properties

#[derive(Clone, Debug)]
struct Spec {
    sender: u8, // 0 me, 1 bob, 2 carol (ignored in some runs)
    is_message: bool,
    mention: bool,
}

fn build(specs: &[Spec], offset: usize) -> Vec<TimelineEvent> {
    specs
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let who = [me(), bob(), carol()][usize::from(s.sender % 3)].clone();
            let id = ev(offset + i);
            match (s.is_message, s.mention) {
                (true, true) => TimelineEvent::message(id, who).mentioning_me(),
                (true, false) => TimelineEvent::message(id, who),
                (false, _) => TimelineEvent::other(id, who),
            }
        })
        .collect()
}

fn specs() -> impl Strategy<Value = Vec<Spec>> {
    prop::collection::vec(
        (0u8..3, any::<bool>(), any::<bool>()).prop_map(|(sender, is_message, mention)| Spec {
            sender,
            is_message,
            mention,
        }),
        0..30,
    )
}

proptest! {
    /// Sending the receipts that `receipt_targets` lists always leaves the room read.
    #[test]
    fn mark_as_read_always_leaves_nothing_unread(
        main in specs(), thread_a in specs(), thread_b in specs(), ignore_carol in any::<bool>(),
        followed_a in any::<bool>(), followed_b in any::<bool>(),
    ) {
        let main_events = build(&main, 0);
        let a_events = build(&thread_a, 1000);
        let b_events = build(&thread_b, 2000);
        let mut c = ctx();
        if ignore_carol { c = c.ignoring(carol()); }
        let compute = |c: &ReadContext| RoomReadState {
            main: timeline_read_state(&main_events, c),
            threads: vec![
                thread_read_state(thread_root(1), &a_events, followed_a, false, c),
                thread_read_state(thread_root(2), &b_events, followed_b, false, c),
            ],
        };
        let before = compute(&c);
        let sent: Vec<EventId> = receipt_targets(&before).into_iter().map(|t| t.event).collect();
        let after = compute(&c.clone().with_receipts(sent));
        prop_assert!(!room_unread(&after, &all()).is_unread());
        prop_assert!(receipt_targets(&after).is_empty());
    }

    /// A receipt only ever reads more: it never raises a count.
    #[test]
    fn a_receipt_never_raises_the_count(main in specs(), pick in 0usize..30) {
        let events = build(&main, 0);
        let without = timeline_read_state(&events, &ctx());
        let with = timeline_read_state(&events, &ctx().with_receipts(vec![ev(pick)]));
        prop_assert!(with.unread_count <= without.unread_count);
        prop_assert!(with.mention_count <= without.mention_count);
    }

    /// Mentions are a subset of the unread messages; unread is bounded by the
    /// messages of others the timeline holds.
    #[test]
    fn mentions_never_exceed_unread_and_unread_never_exceeds_messages(main in specs()) {
        let events = build(&main, 0);
        let state = timeline_read_state(&events, &ctx());
        let others = main.iter().filter(|s| s.is_message && s.sender % 3 != 0).count();
        prop_assert!(state.mention_count <= state.unread_count);
        prop_assert!(usize::try_from(state.unread_count).unwrap() <= others);
    }

    /// A receipt never goes back: whatever is chosen is within the unread tail.
    #[test]
    fn the_chosen_receipt_is_always_in_the_unread_tail(
        len in 0usize..20, unread in 0u32..25, shown in proptest::option::of(0usize..25),
    ) {
        let ids: Vec<EventId> = (0..len).map(ev).collect();
        let shown_id = shown.map(ev);
        if let Some(chosen) = choose_receipt_event(&ids, unread, shown_id.as_ref()) {
            let pos = ids.iter().rposition(|i| *i == chosen).unwrap();
            let tail = usize::try_from(unread).unwrap().min(ids.len());
            prop_assert!(unread > 0 && pos >= ids.len() - tail);
            prop_assert_eq!(Some(chosen), shown_id);
        }
    }
}
