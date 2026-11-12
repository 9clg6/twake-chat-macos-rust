//! `ComposerState`: the text, the cursor, the reply and edit banner, the round button, Enter, and
//! the composer that cannot post.
//!
//! Need (not the code): FR-050 to FR-057, FR-061 and `docs/spec/features/composer.md`
//! - D181 + e2e "Composer": the round button is the microphone while the field is empty, Send once
//!   there is text, the microphone again when emptied.
//! - FR-051, D88, D276: a reply shows a banner in the composer; sent, it answers the original
//!   (`m.in_reply_to`) and mentions its author (spec, rich replies) unless that is the sender.
//! - FR-052, D487: an edit is an `m.replace`; it keeps the mentions whose name the new text still
//!   holds, plus the newly picked; only the newly named ones are notified.
//! - FR-055, D382, D512: a draft is the compose text and its picked mentions; Enter empties it.
//! - FR-056, D194: "Only moderators can post": the composer is disabled with its reason.
//! - e2e "Composer": `/foo bar` is refused and the text stays; `//foo bar` sends `/foo bar`; a bot
//!   command is sent as `!leave 2 dentist X`.
//! - Nothing empty is ever sent: spaces and newlines alone are refused.
//!
//! Public items used are listed in `docs/dev/composer-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_app::composer::{
    Access, ComposerMode, ComposerState, DisabledReason, Draft, EditTarget, MessageKind,
    OutgoingMessage, PrimaryAction, Relation, ReplyTarget, Selection, Sources, SubmitError,
    Suggestion, can_post,
};
use twake_domain::commands::{BotCommand, LocalCommand};
use twake_domain::mention::{Member, Mention, Mentions};
use twake_domain::{EventId, RoomId, UserId};

// ------------------------------------------------------------ helpers

fn room() -> RoomId {
    RoomId::parse("!room:example.org").unwrap()
}

fn user(local: &str) -> UserId {
    UserId::parse(&format!("@{local}:example.org")).unwrap()
}

fn event(n: &str) -> EventId {
    EventId::parse(&format!("${n}")).unwrap()
}

fn state() -> ComposerState {
    ComposerState::new(room(), user("me"))
}

fn typed(text: &str) -> ComposerState {
    let mut s = state();
    s.set_text(text, text.len());
    s
}

fn member(local: &str, name: &str) -> Member {
    Member {
        user_id: user(local),
        name: name.into(),
        matrix_name: None,
    }
}

fn with_people(text: &str) -> ComposerState {
    let mut s = state();
    s.set_sources(Sources {
        members: vec![
            member("bob", "Bob"),
            member("carol", "Carol"),
            member("dave", "Dave"),
        ],
        ..Sources::default()
    });
    s.set_text(text, text.len());
    s
}

fn pick(s: &mut ComposerState, index: usize) {
    let suggestion = s.suggestions().remove(index);
    s.accept(suggestion);
}

fn reply_to(id: &str, author: &str) -> ReplyTarget {
    ReplyTarget {
        event_id: event(id),
        sender: user(author),
        preview: format!("said by {author}"),
    }
}

fn sent(mut s: ComposerState) -> OutgoingMessage {
    s.submit().unwrap()
}

fn users(m: &Mentions) -> Vec<UserId> {
    m.user_ids.clone()
}

fn bot_leave() -> BotCommand {
    BotCommand {
        bot: user("bot_alice"),
        name: "leave".into(),
        syntax: "leave {days} {reason...}".into(),
        description: "Ask for days off".into(),
    }
}

// ------------------------------------------------------------ the round button (FR-050)

#[test]
fn an_empty_field_shows_the_microphone() {
    assert_eq!(state().action(), PrimaryAction::Mic);
}

#[test]
fn the_first_letter_turns_it_into_send() {
    assert_eq!(typed("a").action(), PrimaryAction::Send);
}

#[test]
fn emptied_again_the_microphone_returns() {
    let mut s = typed("a");
    s.set_text("", 0);
    assert_eq!(s.action(), PrimaryAction::Mic);
}

#[test]
fn a_sent_message_gives_the_microphone_back() {
    let mut s = typed("hello");
    s.submit().unwrap();
    assert_eq!(s.action(), PrimaryAction::Mic);
}

// ------------------------------------------------------------ text and cursor

#[test]
fn a_new_composer_is_empty_and_plain() {
    let s = state();
    assert_eq!(s.text(), "");
    assert_eq!(s.selection(), Selection { anchor: 0, head: 0 });
    assert_eq!(s.mode(), &ComposerMode::Compose);
    assert_eq!(s.access(), Access::Enabled);
}

#[test]
fn typing_sets_the_text_and_the_cursor() {
    let mut s = state();
    s.set_text("hello", 2);
    assert_eq!(s.text(), "hello");
    assert_eq!(s.selection(), Selection { anchor: 2, head: 2 });
}

#[test]
fn a_cursor_past_the_end_goes_to_the_end() {
    let mut s = state();
    s.set_text("hello", 99);
    assert_eq!(s.selection(), Selection { anchor: 5, head: 5 });
}

#[test]
fn a_cursor_inside_a_letter_goes_back_to_its_start() {
    let mut s = state();
    s.set_text("aéb", 2);
    assert_eq!(s.selection().head, 1);
}

#[test]
fn a_selection_is_kept_as_given() {
    let mut s = typed("hello world");
    s.select(6, 11);
    assert_eq!(
        s.selection(),
        Selection {
            anchor: 6,
            head: 11
        }
    );
}

#[test]
fn a_selection_is_clamped_to_the_text() {
    let mut s = typed("héllo");
    s.select(2, 99);
    assert_eq!(s.selection(), Selection { anchor: 1, head: 6 });
}

#[test]
fn inserting_replaces_the_selection() {
    let mut s = typed("hello world");
    s.select(0, 5);
    s.insert("bye");
    assert_eq!(s.text(), "bye world");
    assert_eq!(s.selection(), Selection { anchor: 3, head: 3 });
}

#[test]
fn inserting_with_the_selection_backwards_replaces_it_too() {
    let mut s = typed("hello world");
    s.select(5, 0);
    s.insert("bye");
    assert_eq!(s.text(), "bye world");
}

#[test]
fn inserting_at_the_cursor_adds() {
    let mut s = state();
    s.set_text("ac", 1);
    s.insert("b");
    assert_eq!(s.text(), "abc");
    assert_eq!(s.selection().head, 2);
}

// ------------------------------------------------------------ Enter: what is sent

#[test]
fn an_empty_field_cannot_be_sent() {
    assert_eq!(state().submit(), Err(SubmitError::Empty));
}

#[test]
fn spaces_and_newlines_alone_cannot_be_sent_and_stay() {
    let mut s = typed("  \n \t ");
    assert_eq!(s.submit(), Err(SubmitError::Empty));
    assert_eq!(s.text(), "  \n \t ");
}

#[test]
fn a_text_is_sent_trimmed_as_plain_text() {
    let m = sent(typed("  hello there \n"));
    assert_eq!(
        m,
        OutgoingMessage {
            kind: MessageKind::Text,
            body: "hello there".into(),
            formatted_body: None,
            mentions: Mentions::default(),
            notify: Mentions::default(),
            relation: Relation::None,
            bot_command: None,
        }
    );
}

#[test]
fn markdown_is_sent_as_html_with_the_text_as_body() {
    let m = sent(typed("**loud** `x`"));
    assert_eq!(m.body, "**loud** `x`");
    assert_eq!(
        m.formatted_body.as_deref(),
        Some("<strong>loud</strong> <code>x</code>")
    );
}

#[test]
fn sending_empties_the_field() {
    let mut s = typed("hello");
    s.submit().unwrap();
    assert_eq!(s.text(), "");
    assert_eq!(s.selection(), Selection { anchor: 0, head: 0 });
    assert_eq!(s.submit(), Err(SubmitError::Empty));
}

#[test]
fn a_me_command_is_an_emote() {
    let m = sent(typed("/me waves"));
    assert_eq!((m.kind, m.body.as_str()), (MessageKind::Emote, "waves"));
}

#[test]
fn an_unknown_command_is_refused_and_the_text_stays() {
    let mut s = typed("/foo bar");
    assert_eq!(s.submit(), Err(SubmitError::UnknownCommand("foo".into())));
    assert_eq!(s.text(), "/foo bar");
}

#[test]
fn a_double_slash_sends_one_slash() {
    assert_eq!(sent(typed("//foo bar")).body, "/foo bar");
}

#[test]
fn a_shrug_is_sent_as_text() {
    assert_eq!(sent(typed("/shrug")).body, "¯\\_(ツ)_/¯");
}

#[test]
fn a_bot_command_is_sent_with_a_bang_and_its_command() {
    let mut s = typed("/leave 2 dentist X");
    s.set_sources(Sources {
        commands: vec![bot_leave()],
        ..Sources::default()
    });
    let m = s.submit().unwrap();
    assert_eq!(m.body, "!leave 2 dentist X");
    assert_eq!(m.kind, MessageKind::Text);
    assert_eq!(m.bot_command, Some(bot_leave()));
}

#[test]
fn a_local_command_sends_nothing_and_empties_the_field() {
    let mut s = typed("/discardsession");
    assert_eq!(
        s.submit(),
        Err(SubmitError::LocalCommand(LocalCommand::DiscardSession))
    );
    assert_eq!(s.text(), "");
}

#[test]
fn every_submit_error_says_something_different() {
    let errors = [
        SubmitError::Empty,
        SubmitError::UnknownCommand("x".into()),
        SubmitError::Disabled(DisabledReason::OnlyModeratorsPost),
        SubmitError::LocalCommand(LocalCommand::DiscardSession),
    ];
    let texts: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert!(texts.iter().all(|t| !t.is_empty()));
    for (i, a) in texts.iter().enumerate() {
        for b in &texts[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

// ------------------------------------------------------------ mentions picked and sent

#[test]
fn a_picked_person_is_linked_and_mentioned() {
    let mut s = with_people("hi @bo");
    pick(&mut s, 0);
    assert_eq!(s.text(), "hi Bob ");
    let m = sent(s);
    assert_eq!(users(&m.mentions), vec![user("bob")]);
    assert_eq!(users(&m.notify), vec![user("bob")]);
    assert_eq!(
        m.formatted_body.as_deref(),
        Some("hi <a href=\"https://matrix.to/#/%40bob%3Aexample.org\">Bob</a>")
    );
}

#[test]
fn a_picked_name_erased_notifies_nobody() {
    let mut s = with_people("@bo");
    pick(&mut s, 0);
    s.set_text("hello", 5);
    let m = sent(s);
    assert_eq!(m.mentions, Mentions::default());
    assert_eq!(m.formatted_body, None);
}

#[test]
fn everyone_is_mentioned_without_a_link() {
    let mut s = state();
    s.set_sources(Sources {
        members: vec![member("bob", "Bob")],
        can_mention_room: true,
        ..Sources::default()
    });
    s.set_text("@all", 4);
    s.accept(Suggestion::Everyone);
    assert_eq!(s.text(), "@room ");
    let m = sent(s);
    assert!(m.mentions.room);
    assert!(m.notify.room);
    assert_eq!(m.mentions.user_ids, vec![]);
    assert_eq!(m.formatted_body, None);
}

// ------------------------------------------------------------ reply (FR-051)

#[test]
fn starting_a_reply_shows_the_banner_and_keeps_the_text() {
    let mut s = typed("draft");
    s.start_reply(reply_to("orig", "bob"));
    assert_eq!(s.mode(), &ComposerMode::Reply(reply_to("orig", "bob")));
    assert_eq!(s.text(), "draft");
}

#[test]
fn a_new_reply_replaces_the_previous_one() {
    let mut s = state();
    s.start_reply(reply_to("one", "bob"));
    s.start_reply(reply_to("two", "carol"));
    assert_eq!(s.mode(), &ComposerMode::Reply(reply_to("two", "carol")));
}

#[test]
fn cancelling_a_reply_keeps_the_text() {
    let mut s = typed("draft");
    s.start_reply(reply_to("orig", "bob"));
    s.cancel_mode();
    assert_eq!(s.mode(), &ComposerMode::Compose);
    assert_eq!(s.text(), "draft");
}

#[test]
fn a_reply_answers_the_original_and_mentions_its_author() {
    let mut s = typed("agreed");
    s.start_reply(reply_to("orig", "bob"));
    let m = s.submit().unwrap();
    assert_eq!(m.relation, Relation::Reply { to: event("orig") });
    assert_eq!(users(&m.mentions), vec![user("bob")]);
    assert_eq!(users(&m.notify), vec![user("bob")]);
    assert_eq!(s.mode(), &ComposerMode::Compose);
}

#[test]
fn a_reply_to_oneself_mentions_nobody() {
    let mut s = typed("note");
    s.start_reply(reply_to("mine", "me"));
    let m = s.submit().unwrap();
    assert_eq!(m.relation, Relation::Reply { to: event("mine") });
    assert_eq!(m.mentions, Mentions::default());
}

#[test]
fn the_author_picked_and_replied_to_is_mentioned_once() {
    let mut s = with_people("@bo");
    pick(&mut s, 0);
    s.start_reply(reply_to("orig", "bob"));
    assert_eq!(users(&s.submit().unwrap().mentions), vec![user("bob")]);
}

#[test]
fn an_empty_reply_is_refused_and_the_banner_stays() {
    let mut s = state();
    s.start_reply(reply_to("orig", "bob"));
    assert_eq!(s.submit(), Err(SubmitError::Empty));
    assert!(matches!(s.mode(), ComposerMode::Reply(_)));
}

// ------------------------------------------------------------ edit (FR-052, D487)

fn edit_of(body: &str, mentions: Vec<Mention>) -> EditTarget {
    EditTarget {
        event_id: event("mine"),
        body: body.into(),
        mentions,
    }
}

#[test]
fn editing_loads_the_message_with_the_cursor_at_its_end() {
    let mut s = typed("half typed");
    s.start_edit(edit_of("héllo", vec![]));
    assert_eq!(s.text(), "héllo");
    assert_eq!(s.selection().head, "héllo".len());
    assert!(matches!(s.mode(), ComposerMode::Edit(_)));
}

#[test]
fn an_edit_replaces_the_message() {
    let mut s = state();
    s.start_edit(edit_of("old", vec![]));
    s.set_text("new", 3);
    let m = s.submit().unwrap();
    assert_eq!(
        m.relation,
        Relation::Replace {
            target: event("mine")
        }
    );
    assert_eq!((m.kind, m.body.as_str()), (MessageKind::Text, "new"));
}

#[test]
fn after_an_edit_the_text_typed_before_comes_back() {
    let mut s = typed("half typed");
    s.start_edit(edit_of("old", vec![]));
    s.set_text("new", 3);
    s.submit().unwrap();
    assert_eq!(s.mode(), &ComposerMode::Compose);
    assert_eq!(s.text(), "half typed");
}

#[test]
fn cancelling_an_edit_gives_the_text_typed_before_back() {
    let mut s = typed("half typed");
    s.start_edit(edit_of("old", vec![]));
    s.cancel_mode();
    assert_eq!(s.mode(), &ComposerMode::Compose);
    assert_eq!(s.text(), "half typed");
}

#[test]
fn an_edit_emptied_is_refused_and_stays_an_edit() {
    let mut s = state();
    s.start_edit(edit_of("old", vec![]));
    s.set_text("", 0);
    assert_eq!(s.submit(), Err(SubmitError::Empty));
    assert!(matches!(s.mode(), ComposerMode::Edit(_)));
}

#[test]
fn an_edit_keeps_the_original_mention_without_notifying_again() {
    let mut s = state();
    s.start_edit(edit_of("hi Bob", vec![Mention::user(user("bob"), "Bob")]));
    s.set_text("hi Bob!", 7);
    let m = s.submit().unwrap();
    assert_eq!(users(&m.mentions), vec![user("bob")]);
    assert_eq!(m.notify, Mentions::default());
    assert!(
        m.formatted_body
            .unwrap()
            .contains("matrix.to/#/%40bob%3Aexample.org")
    );
}

#[test]
fn an_edit_drops_an_original_mention_whose_name_is_gone() {
    let mut s = state();
    s.start_edit(edit_of("hi Bob", vec![Mention::user(user("bob"), "Bob")]));
    s.set_text("hi", 2);
    let m = s.submit().unwrap();
    assert_eq!(m.mentions, Mentions::default());
    assert_eq!(m.notify, Mentions::default());
}

#[test]
fn an_edit_notifies_only_the_newly_picked() {
    let mut s = with_people("");
    s.start_edit(edit_of(
        "hi Bob and ",
        vec![Mention::user(user("bob"), "Bob")],
    ));
    s.set_text("hi Bob and @ca", 14);
    pick(&mut s, 0);
    let m = s.submit().unwrap();
    assert_eq!(users(&m.mentions), vec![user("bob"), user("carol")]);
    assert_eq!(users(&m.notify), vec![user("carol")]);
}

#[test]
fn the_draft_during_an_edit_is_the_text_kept_aside() {
    let mut s = typed("half typed");
    s.start_edit(edit_of("old", vec![]));
    s.set_text("edited", 6);
    assert_eq!(s.draft().text, "half typed");
}

// ------------------------------------------------------------ drafts (FR-055)

#[test]
fn the_draft_holds_the_text_and_the_picked_mentions() {
    let mut s = with_people("@bo");
    pick(&mut s, 0);
    assert_eq!(
        s.draft(),
        Draft {
            text: "Bob ".into(),
            mentions: vec![Mention::user(user("bob"), "Bob")]
        }
    );
}

#[test]
fn an_empty_composer_has_an_empty_draft() {
    assert_eq!(state().draft(), Draft::default());
}

#[test]
fn restoring_a_draft_puts_the_cursor_at_its_end() {
    let mut s = state();
    s.restore(Draft {
        text: "héllo".into(),
        mentions: vec![],
    });
    assert_eq!(s.text(), "héllo");
    assert_eq!(s.selection(), Selection { anchor: 6, head: 6 });
    assert_eq!(s.action(), PrimaryAction::Send);
}

#[test]
fn a_restored_mention_is_still_sent() {
    let mut s = state();
    s.restore(Draft {
        text: "hi Bob".into(),
        mentions: vec![Mention::user(user("bob"), "Bob")],
    });
    assert_eq!(users(&s.submit().unwrap().mentions), vec![user("bob")]);
}

#[test]
fn sending_empties_the_draft() {
    let mut s = with_people("@bo");
    pick(&mut s, 0);
    s.submit().unwrap();
    assert_eq!(s.draft(), Draft::default());
}

// ------------------------------------------------------------ a composer that cannot post (FR-056)

#[test]
fn a_member_below_the_level_cannot_post() {
    assert_eq!(
        can_post(0, 50),
        Access::Disabled(DisabledReason::OnlyModeratorsPost)
    );
    assert_eq!(
        can_post(49, 50),
        Access::Disabled(DisabledReason::OnlyModeratorsPost)
    );
}

#[test]
fn a_member_at_or_above_the_level_can_post() {
    assert_eq!(can_post(50, 50), Access::Enabled);
    assert_eq!(can_post(100, 50), Access::Enabled);
    assert_eq!(can_post(0, 0), Access::Enabled);
}

#[test]
fn a_disabled_composer_says_why_and_refuses_to_send() {
    let mut s = typed("hello");
    s.set_access(Access::Disabled(DisabledReason::OnlyModeratorsPost));
    assert_eq!(
        s.access(),
        Access::Disabled(DisabledReason::OnlyModeratorsPost)
    );
    assert_eq!(
        s.submit(),
        Err(SubmitError::Disabled(DisabledReason::OnlyModeratorsPost))
    );
}

#[test]
fn a_disabled_composer_takes_no_text() {
    let mut s = state();
    s.set_access(Access::Disabled(DisabledReason::OnlyModeratorsPost));
    s.set_text("hello", 5);
    s.insert("x");
    assert_eq!(s.text(), "");
    assert_eq!(s.action(), PrimaryAction::Mic);
}

#[test]
fn a_disabled_composer_takes_no_reply_nor_edit() {
    let mut s = state();
    s.set_access(Access::Disabled(DisabledReason::OnlyModeratorsPost));
    s.start_reply(reply_to("orig", "bob"));
    s.start_edit(edit_of("old", vec![]));
    assert_eq!(s.mode(), &ComposerMode::Compose);
}

#[test]
fn enabled_again_it_takes_text() {
    let mut s = state();
    s.set_access(Access::Disabled(DisabledReason::OnlyModeratorsPost));
    s.set_access(Access::Enabled);
    s.set_text("hello", 5);
    assert_eq!(s.text(), "hello");
}

// ------------------------------------------------------------ properties

#[derive(Debug, Clone)]
enum Op {
    Type(String, usize),
    Select(usize, usize),
    Insert(String),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        ("(?s:.{0,12})", 0usize..30).prop_map(|(t, c)| Op::Type(t, c)),
        (0usize..30, 0usize..30).prop_map(|(a, h)| Op::Select(a, h)),
        "(?s:.{0,6})".prop_map(Op::Insert),
    ]
}

proptest! {
    #[test]
    fn the_cursor_always_sits_on_a_letter_boundary_inside_the_text(ops in proptest::collection::vec(op(), 0..12)) {
        let mut s = state();
        for op in ops {
            match op {
                Op::Type(t, c) => s.set_text(&t, c),
                Op::Select(a, h) => s.select(a, h),
                Op::Insert(t) => s.insert(&t),
            }
            let sel = s.selection();
            for at in [sel.anchor, sel.head] {
                prop_assert!(at <= s.text().len());
                prop_assert!(s.text().is_char_boundary(at));
            }
        }
    }

    #[test]
    fn a_message_is_never_empty(text in "(?s:.{0,30})", reply in any::<bool>()) {
        let mut s = typed(&text);
        if reply {
            s.start_reply(reply_to("orig", "bob"));
        }
        if let Ok(m) = s.submit() {
            prop_assert!(!m.body.trim().is_empty());
        }
    }

    #[test]
    fn blank_text_is_never_sent(text in "[ \t\n\r]{0,12}") {
        prop_assert_eq!(typed(&text).submit(), Err(SubmitError::Empty));
    }

    #[test]
    fn what_is_sent_never_carries_a_script(text in "(?s:.{0,30})") {
        let mut s = typed(&format!("**x** <script>{text}"));
        let m = s.submit().unwrap();
        prop_assert!(!m.formatted_body.unwrap_or_default().contains("<script"));
    }

    #[test]
    fn a_failed_submit_changes_nothing(text in "/[a-z]{1,6} [a-z ]{0,6}") {
        let mut s = typed(&text);
        if s.submit().is_err() {
            prop_assert_eq!(s.text(), text.as_str());
        }
    }
}
