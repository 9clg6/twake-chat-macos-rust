//! The menu of a message and the bar of a selection: which entries, in which order, when absent.
//!
//! Need: FR-075, FR-076, FR-079, FR-080; D274 (menu order, lines between groups, "unavailable
//! actions absent", Delete last in red), D77 (Delete offered on others' messages when the power
//! levels allow `redact`), D130 (no "Copy message" for a message without text), D183 and D260
//! (selection bar, Pin / Unpin for the whole selection), D109 and D90 (a room where only
//! moderators post), D588 (Translate, for the reader alone, others' text, Scribe offered),
//! D494 and D495 (a failed message: retry or discard; nothing while sending). Specified by
//! `docs/dev/message-actions-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_app::message_actions::{
    Action, BulkAction, MessageFacts, RoomCtx, bulk_delete_targets, menu_sections, message_actions,
    selection_actions,
};
use twake_domain::power_levels::PowerLevels;
use twake_domain::timeline::Message;
use twake_domain::{EventId, Timestamp, TxnId, UserId};

use Action::*;

fn user(name: &str) -> UserId {
    UserId::parse(&format!("@{name}:example.org")).unwrap()
}

fn me() -> UserId {
    user("me")
}

fn ev(n: usize) -> EventId {
    EventId::parse(&format!("$msg{n}")).unwrap()
}

fn ts() -> Timestamp {
    Timestamp::from_millis(1_700_000_000_000)
}

fn mine(body: &str) -> Message {
    Message::remote(ev(1), me(), ts(), body)
}

fn theirs(body: &str) -> Message {
    Message::remote(ev(2), user("bob"), ts(), body)
}

fn room() -> RoomCtx {
    RoomCtx {
        power: PowerLevels::new(),
        scribe: false,
    }
}

fn moderator() -> RoomCtx {
    RoomCtx {
        power: PowerLevels::new().with_user(me(), 50),
        scribe: false,
    }
}

fn facts() -> MessageFacts {
    MessageFacts::default()
}

fn menu(m: &Message, f: &MessageFacts, r: &RoomCtx) -> Vec<Action> {
    message_actions(m, f, r, &me())
}

// ------------------------------------------------------------ the whole menu, in order

#[test]
fn a_moderator_sees_every_entry_of_a_message_of_someone_else_in_menu_order() {
    let r = RoomCtx {
        scribe: true,
        ..moderator()
    };
    assert_eq!(
        menu(&theirs("hello"), &facts(), &r),
        [
            React, Reply, CopyText, Pin, Select, Thread, Translate, CopyLink, Report, Delete
        ]
    );
}

#[test]
fn my_own_message_offers_edit_and_delete_and_no_report_nor_translate() {
    let r = RoomCtx {
        scribe: true,
        ..room()
    };
    assert_eq!(
        menu(&mine("hello"), &facts(), &r),
        [
            React, Reply, CopyText, Select, Thread, Edit, CopyLink, Delete
        ]
    );
}

#[test]
fn an_ordinary_member_cannot_pin_nor_delete_someone_elses_message() {
    let got = menu(&theirs("hello"), &facts(), &room());
    assert_eq!(
        got,
        [React, Reply, CopyText, Select, Thread, CopyLink, Report]
    );
}

#[test]
fn delete_is_last_and_pin_never_comes_with_unpin() {
    for r in [room(), moderator()] {
        for m in [mine("a"), theirs("b")] {
            for pinned in [false, true] {
                let got = menu(&m, &MessageFacts { pinned, ..facts() }, &r);
                if let Some(at) = got.iter().position(|a| *a == Delete) {
                    assert_eq!(at, got.len() - 1, "{got:?}");
                }
                assert!(!(got.contains(&Pin) && got.contains(&Unpin)), "{got:?}");
            }
        }
    }
}

// ------------------------------------------------------------ pin

#[test]
fn a_pinned_message_offers_unpin_instead_of_pin() {
    let f = MessageFacts {
        pinned: true,
        ..facts()
    };
    let got = menu(&theirs("hello"), &f, &moderator());
    assert!(got.contains(&Unpin));
    assert!(!got.contains(&Pin));
}

#[test]
fn a_member_who_may_not_pin_sees_neither_pin_nor_unpin() {
    let f = MessageFacts {
        pinned: true,
        ..facts()
    };
    let got = menu(&theirs("hello"), &f, &room());
    assert!(!got.contains(&Pin) && !got.contains(&Unpin));
}

#[test]
fn pin_moves_with_the_right_to_send_that_state() {
    let r = RoomCtx {
        power: PowerLevels::new().with_event("m.room.pinned_events", 0),
        scribe: false,
    };
    assert!(menu(&theirs("hello"), &facts(), &r).contains(&Pin));
}

// ------------------------------------------------------------ delete

#[test]
fn delete_of_someone_else_needs_the_redact_level() {
    let r = RoomCtx {
        power: PowerLevels::new().with_user(me(), 49),
        scribe: false,
    };
    assert!(!menu(&theirs("hi"), &facts(), &r).contains(&Delete));
    let r = RoomCtx {
        power: PowerLevels::new().with_user(me(), 50),
        scribe: false,
    };
    assert!(menu(&theirs("hi"), &facts(), &r).contains(&Delete));
    let r = RoomCtx {
        power: PowerLevels::new().with_user(me(), 50).with_redact(75),
        scribe: false,
    };
    assert!(!menu(&theirs("hi"), &facts(), &r).contains(&Delete));
}

#[test]
fn i_can_always_delete_my_own_message() {
    let r = RoomCtx {
        power: PowerLevels::new().with_redact(100),
        scribe: false,
    };
    assert!(menu(&mine("hi"), &facts(), &r).contains(&Delete));
}

// ------------------------------------------------------------ edit

#[test]
fn edit_is_for_my_own_text_only() {
    assert!(menu(&mine("hi"), &facts(), &room()).contains(&Edit));
    assert!(!menu(&theirs("hi"), &facts(), &moderator()).contains(&Edit));
}

#[test]
fn an_old_message_can_still_be_edited() {
    let old = Message::remote(ev(1), me(), Timestamp::from_millis(1_000), "ancient");
    assert!(menu(&old, &facts(), &room()).contains(&Edit));
}

#[test]
fn a_message_that_cannot_be_read_cannot_be_edited() {
    let f = MessageFacts {
        undecryptable: true,
        ..facts()
    };
    assert!(!menu(&mine("hi"), &f, &room()).contains(&Edit));
}

// ------------------------------------------------------------ reply, thread, react: rights

#[test]
fn where_only_moderators_write_a_member_neither_replies_nor_edits_nor_threads_nor_reacts() {
    let r = RoomCtx {
        power: PowerLevels::new().with_events_default(50),
        scribe: false,
    };
    let got = menu(&mine("hi"), &facts(), &r);
    assert_eq!(got, [CopyText, Select, CopyLink, Delete]);
}

#[test]
fn a_moderator_in_that_room_does_everything() {
    let r = RoomCtx {
        power: PowerLevels::new()
            .with_events_default(50)
            .with_user(me(), 50),
        scribe: false,
    };
    let got = menu(&mine("hi"), &facts(), &r);
    assert!(got.contains(&Reply) && got.contains(&Edit) && got.contains(&Thread));
    assert!(got.contains(&React));
}

#[test]
fn reactions_can_be_closed_while_replies_stay_open() {
    let r = RoomCtx {
        power: PowerLevels::new().with_event("m.reaction", 50),
        scribe: false,
    };
    let got = menu(&theirs("hi"), &facts(), &r);
    assert!(!got.contains(&React) && got.contains(&Reply));
}

#[test]
fn a_reply_inside_a_thread_cannot_start_another_thread() {
    let f = MessageFacts {
        in_thread: true,
        ..facts()
    };
    let got = menu(&theirs("hi"), &f, &room());
    assert!(!got.contains(&Thread) && got.contains(&Reply));
}

#[test]
fn a_message_that_cannot_be_read_cannot_start_a_thread() {
    let f = MessageFacts {
        undecryptable: true,
        ..facts()
    };
    assert!(!menu(&theirs("hi"), &f, &room()).contains(&Thread));
}

// ------------------------------------------------------------ copy, translate

#[test]
fn a_message_without_text_has_no_copy_text_but_keeps_copy_link() {
    let got = menu(&theirs(""), &facts(), &room());
    assert!(!got.contains(&CopyText));
    assert!(got.contains(&CopyLink));
}

#[test]
fn an_undecryptable_message_has_no_copy_text_and_no_translate() {
    let f = MessageFacts {
        undecryptable: true,
        ..facts()
    };
    let r = RoomCtx {
        scribe: true,
        ..room()
    };
    let got = menu(&theirs("???"), &f, &r);
    assert!(!got.contains(&CopyText) && !got.contains(&Translate));
    assert!(got.contains(&Select) && got.contains(&CopyLink));
}

#[test]
fn translate_is_offered_on_someone_elses_text_where_the_scribe_is() {
    let on = RoomCtx {
        scribe: true,
        ..room()
    };
    assert!(menu(&theirs("bonjour"), &facts(), &on).contains(&Translate));
    assert!(!menu(&theirs("bonjour"), &facts(), &room()).contains(&Translate));
    assert!(!menu(&mine("bonjour"), &facts(), &on).contains(&Translate));
    assert!(!menu(&theirs(""), &facts(), &on).contains(&Translate));
}

// ------------------------------------------------------------ sending, failed, deleted

#[test]
fn a_message_still_sending_has_no_menu() {
    let echo = Message::local(TxnId::parse("txn1").unwrap(), me(), ts(), "hello");
    assert!(menu(&echo, &facts(), &moderator()).is_empty());
}

#[test]
fn a_failed_message_offers_retry_and_discard_and_nothing_else() {
    let failed = Message::local(TxnId::parse("txn2").unwrap(), me(), ts(), "hello").failed();
    assert_eq!(menu(&failed, &facts(), &moderator()), [Retry, Discard]);
}

#[test]
fn a_deleted_message_has_no_menu() {
    let f = MessageFacts {
        deleted: true,
        ..facts()
    };
    assert!(menu(&mine("gone"), &f, &moderator()).is_empty());
}

// ------------------------------------------------------------ the lines of the menu (D274)

#[test]
fn the_menu_is_cut_in_the_lines_of_the_design_and_react_stays_above() {
    let all = [
        React, Reply, CopyText, Pin, Select, Thread, Edit, Translate, CopyLink, Report, Delete,
    ];
    assert_eq!(
        menu_sections(&all),
        vec![
            vec![Reply, CopyText, Pin, Select],
            vec![Thread, Edit],
            vec![Translate],
            vec![CopyLink],
            vec![Report, Delete],
        ]
    );
}

#[test]
fn a_line_with_no_entry_is_not_drawn() {
    assert_eq!(
        menu_sections(&[React, Reply, Select, CopyLink, Delete]),
        vec![vec![Reply, Select], vec![CopyLink], vec![Delete]]
    );
}

#[test]
fn unpin_sits_with_pin_and_retry_with_discard() {
    assert_eq!(
        menu_sections(&[Reply, Unpin, Select]),
        vec![vec![Reply, Unpin, Select]]
    );
    assert_eq!(menu_sections(&[Retry, Discard]), vec![vec![Retry, Discard]]);
}

#[test]
fn no_entry_means_no_line() {
    assert!(menu_sections(&[]).is_empty());
    assert!(menu_sections(&[React]).is_empty());
}

// ------------------------------------------------------------ the bar of a selection

fn pair<'a>(m: &'a Message, f: &'a MessageFacts) -> (&'a Message, &'a MessageFacts) {
    (m, f)
}

#[test]
fn nothing_ticked_offers_nothing() {
    assert!(selection_actions(&[], &moderator(), &me()).is_empty());
}

#[test]
fn own_messages_offer_copy_and_delete_and_a_member_cannot_pin() {
    let (a, b, f) = (mine("a"), Message::remote(ev(3), me(), ts(), "b"), facts());
    let got = selection_actions(&[pair(&a, &f), pair(&b, &f)], &room(), &me());
    assert_eq!(got, [BulkAction::Copy, BulkAction::Delete]);
}

#[test]
fn a_moderator_pins_the_selection_and_deletes_it_even_others_messages() {
    let (a, b, f) = (mine("a"), theirs("b"), facts());
    let got = selection_actions(&[pair(&a, &f), pair(&b, &f)], &moderator(), &me());
    assert_eq!(got, [BulkAction::Copy, BulkAction::Pin, BulkAction::Delete]);
}

#[test]
fn bulk_unpin_replaces_pin_only_when_every_ticked_message_is_pinned() {
    let (a, b) = (theirs("a"), Message::remote(ev(3), user("bob"), ts(), "b"));
    let (on, off) = (
        MessageFacts {
            pinned: true,
            ..facts()
        },
        facts(),
    );
    let all = selection_actions(&[pair(&a, &on), pair(&b, &on)], &moderator(), &me());
    assert!(all.contains(&BulkAction::Unpin) && !all.contains(&BulkAction::Pin));
    let some = selection_actions(&[pair(&a, &on), pair(&b, &off)], &moderator(), &me());
    assert!(some.contains(&BulkAction::Pin) && !some.contains(&BulkAction::Unpin));
}

#[test]
fn bulk_delete_is_offered_when_at_least_one_message_may_be_deleted() {
    let (a, b, f) = (mine("a"), theirs("b"), facts());
    let got = selection_actions(&[pair(&a, &f), pair(&b, &f)], &room(), &me());
    assert!(got.contains(&BulkAction::Delete));
    assert!(got.contains(&BulkAction::Copy));
}

#[test]
fn bulk_delete_is_absent_when_no_message_may_be_deleted() {
    let (a, b, f) = (
        theirs("a"),
        Message::remote(ev(3), user("carol"), ts(), "b"),
        facts(),
    );
    let got = selection_actions(&[pair(&a, &f), pair(&b, &f)], &room(), &me());
    assert_eq!(got, [BulkAction::Copy]);
}

#[test]
fn bulk_delete_targets_only_the_allowed_messages_in_the_order_given() {
    let (a, b, c, f) = (
        mine("a"),
        theirs("b"),
        Message::remote(ev(3), me(), ts(), "c"),
        facts(),
    );
    let items = [pair(&a, &f), pair(&b, &f), pair(&c, &f)];
    assert_eq!(bulk_delete_targets(&items, &room(), &me()), [ev(1), ev(3)]);
}

#[test]
fn a_moderator_deletes_every_ticked_message() {
    let (a, b, f) = (mine("a"), theirs("b"), facts());
    let items = [pair(&a, &f), pair(&b, &f)];
    assert_eq!(
        bulk_delete_targets(&items, &moderator(), &me()),
        [ev(1), ev(2)]
    );
}

#[test]
fn nothing_ticked_or_nothing_allowed_targets_nothing() {
    assert!(bulk_delete_targets(&[], &moderator(), &me()).is_empty());
    let (b, f) = (theirs("b"), facts());
    assert!(bulk_delete_targets(&[pair(&b, &f)], &room(), &me()).is_empty());
}

#[test]
fn a_one_message_selection_follows_that_message() {
    let (b, f) = (theirs("b"), facts());
    let got = selection_actions(&[pair(&b, &f)], &moderator(), &me());
    assert_eq!(got, [BulkAction::Copy, BulkAction::Pin, BulkAction::Delete]);
}

proptest! {
    #[test]
    fn bulk_delete_is_offered_exactly_when_it_has_a_target(
        own in prop::collection::vec(any::<bool>(), 1..6),
        level in prop::sample::select(vec![0i64, 49, 50, 100]),
    ) {
        let r = RoomCtx { power: PowerLevels::new().with_user(me(), level), scribe: false };
        let msgs: Vec<Message> = own.iter().enumerate()
            .map(|(i, o)| Message::remote(ev(i), if *o { me() } else { user("bob") }, ts(), "x"))
            .collect();
        let f = facts();
        let items: Vec<_> = msgs.iter().map(|m| (m, &f)).collect();
        let targets = bulk_delete_targets(&items, &r, &me());
        let want: usize = if level >= 50 { own.len() } else { own.iter().filter(|o| **o).count() };
        prop_assert_eq!(targets.len(), want);
        prop_assert_eq!(
            selection_actions(&items, &r, &me()).contains(&BulkAction::Delete),
            want > 0
        );
    }
}
