//! The suggestion layer of the composer: what the text before the cursor asks for (`@` people,
//! `#` rooms, `:` emoji, `/` bot commands), what is offered, what Enter writes, what Escape closes.
//!
//! Need (not the code): FR-054, FR-059 and `docs/spec/features/composer.md`
//! - e2e "Composer": ":smi" lists emojis, Escape closes the list and keeps the text "Salut :smi",
//!   a fresh ":smi" lists ":smile:" and Enter writes "😄"; "@bo" lists bob, Enter writes "bob ";
//!   "@@bob:localhost" lists bob; "/le" lists "Ask for days off" and Enter writes "/leave ".
//! - e2e "location-rooms-tombstone": "#target" lists the room, Enter writes `#<alias>:localhost `.
//! - D401/D431: "@room" last of a bare "@" list, first once a word of it is typed, only when the
//!   user may notify the room. D302: the Matrix name is written, never the private one.
//! - D621: Escape closes the list alone (a second one closes what is below it).
//! - "12:30", "http://x" and an address ("mail@x") ask for nothing.
//!
//! Public items used are listed in `docs/dev/composer-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_app::composer::{
    Access, ComposerMode, ComposerState, DisabledReason, EmojiEntry, EscapeOutcome, ReplyTarget,
    Sources, Suggestion, SuggestionKind, Trigger, find_trigger,
};
use twake_domain::commands::BotCommand;
use twake_domain::mention::{LinkableRoom, Member};
use twake_domain::{EventId, RoomAlias, RoomId, UserId};

// ------------------------------------------------------------ helpers

fn user(local: &str) -> UserId {
    UserId::parse(&format!("@{local}:localhost")).unwrap()
}

fn alias(local: &str) -> RoomAlias {
    RoomAlias::parse(&format!("#{local}:localhost")).unwrap()
}

fn trigger(kind: SuggestionKind, start: usize, query: &str) -> Option<Trigger> {
    Some(Trigger {
        kind,
        start,
        query: query.into(),
    })
}

fn at_end(text: &str) -> Option<Trigger> {
    find_trigger(text, text.len())
}

fn command(name: &str, description: &str) -> BotCommand {
    BotCommand {
        bot: user("bot_alice"),
        name: name.into(),
        syntax: name.into(),
        description: description.into(),
    }
}

fn emoji(shortcode: &str, glyph: &str) -> EmojiEntry {
    EmojiEntry {
        shortcode: shortcode.into(),
        glyph: glyph.into(),
    }
}

fn sources() -> Sources {
    Sources {
        members: vec![
            Member {
                user_id: user("bob"),
                name: "Bob Stone".into(),
                matrix_name: None,
            },
            Member {
                user_id: user("jd"),
                name: "Johnny".into(),
                matrix_name: Some("Jonathan Doe".into()),
            },
            Member {
                user_id: user("elo"),
                name: "Élodie".into(),
                matrix_name: None,
            },
        ],
        rooms: vec![
            LinkableRoom {
                alias: alias("target"),
                name: "Target Team".into(),
            },
            LinkableRoom {
                alias: alias("misc"),
                name: "Misc".into(),
            },
        ],
        commands: vec![
            command("help", "What I can do"),
            command("leave", "Ask for days off"),
            command("lexicon", "Words"),
        ],
        emojis: vec![
            emoji("smiley", "😃"),
            emoji("smile", "😄"),
            emoji("grin", "😁"),
            emoji("sweat_smile", "😅"),
        ],
        can_mention_room: false,
    }
}

fn composer(text: &str) -> ComposerState {
    let mut s = ComposerState::new(RoomId::parse("!r:localhost").unwrap(), user("me"));
    s.set_sources(sources());
    s.set_text(text, text.len());
    s
}

fn first(s: &mut ComposerState) {
    let suggestion = s.suggestions().remove(0);
    s.accept(suggestion);
}

fn reply() -> ReplyTarget {
    ReplyTarget {
        event_id: EventId::parse("$orig").unwrap(),
        sender: user("bob"),
        preview: "hi".into(),
    }
}

// ------------------------------------------------------------ find_trigger: people

#[test]
fn an_at_at_the_start_asks_for_people() {
    assert_eq!(at_end("@bo"), trigger(SuggestionKind::Person, 0, "bo"));
}

#[test]
fn an_at_after_a_space_asks_for_people() {
    assert_eq!(at_end("hi @bo"), trigger(SuggestionKind::Person, 3, "bo"));
}

#[test]
fn an_at_after_a_newline_asks_for_people() {
    assert_eq!(at_end("hi\n@bo"), trigger(SuggestionKind::Person, 3, "bo"));
}

#[test]
fn a_bare_at_asks_with_an_empty_query() {
    assert_eq!(at_end("hi @"), trigger(SuggestionKind::Person, 3, ""));
}

#[test]
fn an_address_asks_for_nothing() {
    assert_eq!(at_end("mail@bo"), None);
}

#[test]
fn a_space_ends_the_query() {
    assert_eq!(at_end("@bob "), None);
}

#[test]
fn a_full_matrix_id_keeps_its_second_at() {
    assert_eq!(
        at_end("@@bob:localhost"),
        trigger(SuggestionKind::Person, 0, "@bob:localhost")
    );
}

#[test]
fn the_query_stops_at_the_cursor() {
    assert_eq!(
        find_trigger("@bob hello", 3),
        trigger(SuggestionKind::Person, 0, "bo")
    );
}

#[test]
fn the_start_is_a_byte_offset_after_accents() {
    assert_eq!(at_end("é @bo"), trigger(SuggestionKind::Person, 3, "bo"));
}

#[test]
fn a_cursor_inside_a_letter_does_not_panic() {
    assert_eq!(find_trigger("é@bo", 1), None);
}

// ------------------------------------------------------------ find_trigger: rooms, emoji, commands

#[test]
fn a_hash_asks_for_rooms() {
    assert_eq!(at_end("see #tar"), trigger(SuggestionKind::Room, 4, "tar"));
}

#[test]
fn a_hash_inside_a_word_asks_for_nothing() {
    assert_eq!(at_end("issue#12"), None);
}

#[test]
fn a_colon_then_a_letter_asks_for_emoji() {
    assert_eq!(
        at_end("Salut :smi"),
        trigger(SuggestionKind::Emoji, 6, "smi")
    );
}

#[test]
fn an_emoji_name_may_hold_digits_underscores_plus_and_dashes() {
    assert_eq!(
        at_end(":a1_b+c-d"),
        trigger(SuggestionKind::Emoji, 0, "a1_b+c-d")
    );
}

#[test]
fn a_time_asks_for_nothing() {
    assert_eq!(at_end("meet at 12:30"), None);
    assert_eq!(at_end("12:30"), None);
}

#[test]
fn a_url_asks_for_nothing() {
    assert_eq!(at_end("http://x"), None);
}

#[test]
fn a_colon_alone_or_before_a_digit_asks_for_nothing() {
    assert_eq!(at_end(":"), None);
    assert_eq!(at_end(" :1"), None);
}

#[test]
fn a_slash_at_the_very_start_asks_for_commands() {
    assert_eq!(at_end("/le"), trigger(SuggestionKind::Command, 0, "le"));
    assert_eq!(at_end("/"), trigger(SuggestionKind::Command, 0, ""));
}

#[test]
fn a_slash_elsewhere_asks_for_nothing() {
    assert_eq!(at_end("hi /le"), None);
    assert_eq!(at_end(" /le"), None);
}

#[test]
fn a_command_with_arguments_asks_for_nothing_more() {
    assert_eq!(at_end("/leave 2"), None);
    assert_eq!(at_end("/usr/bin"), None);
    assert_eq!(at_end("//"), None);
}

#[test]
fn plain_text_asks_for_nothing() {
    assert_eq!(at_end("hello"), None);
    assert_eq!(at_end(""), None);
}

// ------------------------------------------------------------ suggestions

#[test]
fn people_are_offered_after_an_at() {
    let s = composer("@bo");
    assert_eq!(
        s.suggestions(),
        vec![Suggestion::Person {
            user_id: user("bob"),
            name: "Bob Stone".into()
        }]
    );
}

#[test]
fn the_matrix_name_is_what_is_offered_and_written() {
    let mut s = composer("@john");
    assert_eq!(
        s.suggestions(),
        vec![Suggestion::Person {
            user_id: user("jd"),
            name: "Jonathan Doe".into()
        }]
    );
    first(&mut s);
    assert_eq!(s.text(), "Jonathan Doe ");
}

#[test]
fn people_are_found_without_accents() {
    assert_eq!(composer("@elo").suggestions().len(), 1);
}

#[test]
fn a_full_matrix_id_lists_the_person() {
    assert_eq!(composer("@@bob:localhost").suggestions().len(), 1);
}

#[test]
fn rooms_are_offered_after_a_hash() {
    assert_eq!(
        composer("#tar").suggestions(),
        vec![Suggestion::Room {
            alias: alias("target")
        }]
    );
}

#[test]
fn commands_are_offered_after_a_slash_in_the_order_announced() {
    assert_eq!(
        composer("/le").suggestions(),
        vec![
            Suggestion::Command {
                name: "leave".into()
            },
            Suggestion::Command {
                name: "lexicon".into()
            },
        ]
    );
}

#[test]
fn a_bare_slash_offers_every_command() {
    assert_eq!(composer("/").suggestions().len(), 3);
}

#[test]
fn emoji_are_offered_the_exact_name_first_then_the_starts_then_the_rest() {
    let glyphs: Vec<String> = composer(":smile")
        .suggestions()
        .into_iter()
        .map(|s| match s {
            Suggestion::Emoji { glyph } => glyph,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(glyphs, ["😄", "😃", "😅"]);
}

#[test]
fn emoji_names_ignore_case() {
    assert_eq!(composer(":GRIN").suggestions().len(), 1);
}

#[test]
fn at_most_eight_emoji_are_offered() {
    let mut s = composer(":e");
    s.set_sources(Sources {
        emojis: (0..12).map(|i| emoji(&format!("e{i}"), "x")).collect(),
        ..Sources::default()
    });
    assert_eq!(s.suggestions().len(), 8);
}

#[test]
fn no_trigger_no_suggestions() {
    assert!(composer("hello").suggestions().is_empty());
}

#[test]
fn no_match_no_suggestions() {
    assert!(composer("@zzz").suggestions().is_empty());
}

#[test]
fn a_bare_at_ends_with_everyone_for_who_may_notify() {
    let mut s = composer("@");
    s.set_sources(Sources {
        can_mention_room: true,
        ..sources()
    });
    let list = s.suggestions();
    assert_eq!(list.len(), 4);
    assert_eq!(list[3], Suggestion::Everyone);
}

#[test]
fn a_bare_at_has_no_everyone_for_who_may_not() {
    assert!(!composer("@").suggestions().contains(&Suggestion::Everyone));
}

#[test]
fn typing_all_puts_everyone_first() {
    let mut s = composer("@all");
    s.set_sources(Sources {
        can_mention_room: true,
        ..sources()
    });
    assert_eq!(s.suggestions()[0], Suggestion::Everyone);
}

#[test]
fn a_bare_at_offers_25_people_at_most() {
    let mut s = composer("@");
    s.set_sources(Sources {
        members: (0..40)
            .map(|i| Member {
                user_id: user(&format!("u{i:02}")),
                name: format!("Person {i:02}"),
                matrix_name: None,
            })
            .collect(),
        ..Sources::default()
    });
    assert_eq!(s.suggestions().len(), 25);
}

#[test]
fn a_bare_at_ends_with_everyone_after_the_25_people() {
    let mut s = composer("@");
    s.set_sources(Sources {
        members: (0..40)
            .map(|i| Member {
                user_id: user(&format!("u{i:02}")),
                name: format!("Person {i:02}"),
                matrix_name: None,
            })
            .collect(),
        can_mention_room: true,
        ..Sources::default()
    });
    let list = s.suggestions();
    assert_eq!(list.len(), 26);
    assert_eq!(list[25], Suggestion::Everyone);
    assert!(matches!(list[24], Suggestion::Person { .. }));
}

// ------------------------------------------------------------ accept

#[test]
fn accepting_a_person_writes_the_name_and_a_space() {
    let mut s = composer("hi @bo");
    first(&mut s);
    assert_eq!(s.text(), "hi Bob Stone ");
    assert_eq!(s.selection().head, "hi Bob Stone ".len());
}

#[test]
fn accepting_keeps_the_text_after_the_cursor() {
    let mut s = composer("@boxyz");
    s.set_text("@boxyz", 3);
    first(&mut s);
    assert!(s.text().starts_with("Bob Stone"));
    assert!(s.text().ends_with("xyz"));
}

#[test]
fn accepting_closes_the_layer() {
    let mut s = composer("@bo");
    first(&mut s);
    assert_eq!(s.trigger(), None);
    assert!(s.suggestions().is_empty());
}

#[test]
fn accepting_a_room_writes_its_alias() {
    let mut s = composer("see #tar");
    first(&mut s);
    assert_eq!(s.text(), "see #target:localhost ");
}

#[test]
fn a_room_picked_is_linked_and_notifies_nobody() {
    let mut s = composer("#tar");
    first(&mut s);
    let m = s.submit().unwrap();
    assert_eq!(m.mentions.user_ids, vec![]);
    assert_eq!(
        m.formatted_body.as_deref(),
        Some("<a href=\"https://matrix.to/#/%23target%3Alocalhost\">#target:localhost</a>")
    );
}

#[test]
fn accepting_a_command_writes_its_name() {
    let mut s = composer("/le");
    first(&mut s);
    assert_eq!(s.text(), "/leave ");
    assert_eq!(s.trigger(), None);
}

#[test]
fn accepting_an_emoji_writes_its_glyph() {
    let mut s = composer("Salut :smile");
    first(&mut s);
    assert_eq!(s.text().trim_end(), "Salut 😄");
    assert_eq!(s.trigger(), None);
}

#[test]
fn accepting_with_nothing_asked_changes_nothing() {
    let mut s = composer("hello");
    s.accept(Suggestion::Everyone);
    assert_eq!(s.text(), "hello");
}

#[test]
fn accepting_does_nothing_in_a_disabled_composer() {
    let mut s = composer("@bo");
    let pick = s.suggestions().remove(0);
    s.set_access(Access::Disabled(DisabledReason::OnlyModeratorsPost));
    s.accept(pick);
    assert_eq!(s.text(), "@bo");
}

// ------------------------------------------------------------ Escape (D621)

#[test]
fn escape_closes_the_layer_and_keeps_the_text() {
    let mut s = composer("Salut :smi");
    assert!(!s.suggestions().is_empty());
    assert_eq!(s.escape(), EscapeOutcome::ClosedSuggestions);
    assert_eq!(s.text(), "Salut :smi");
    assert!(s.suggestions().is_empty());
    assert_eq!(s.trigger(), None);
}

#[test]
fn escape_closes_the_layer_before_the_reply() {
    let mut s = composer("@bo");
    s.start_reply(reply());
    assert_eq!(s.escape(), EscapeOutcome::ClosedSuggestions);
    assert!(matches!(s.mode(), ComposerMode::Reply(_)));
    assert_eq!(s.escape(), EscapeOutcome::CancelledMode);
    assert_eq!(s.mode(), &ComposerMode::Compose);
    assert_eq!(s.text(), "@bo");
}

#[test]
fn escape_without_a_layer_cancels_the_reply() {
    let mut s = composer("hello");
    s.start_reply(reply());
    assert_eq!(s.escape(), EscapeOutcome::CancelledMode);
    assert_eq!(s.mode(), &ComposerMode::Compose);
}

#[test]
fn escape_with_a_trigger_but_no_match_cancels_the_reply() {
    let mut s = composer("@zzz");
    s.start_reply(reply());
    assert_eq!(s.escape(), EscapeOutcome::CancelledMode);
}

#[test]
fn escape_with_nothing_open_does_nothing() {
    let mut s = composer("hello");
    assert_eq!(s.escape(), EscapeOutcome::Nothing);
    assert_eq!(s.text(), "hello");
}

#[test]
fn a_new_trigger_elsewhere_opens_the_layer_again() {
    let mut s = composer("@bo");
    s.escape();
    s.set_text("@bo and @el", 11);
    assert_eq!(s.suggestions().len(), 1);
}

#[test]
fn a_fresh_trigger_after_accepting_opens_the_layer_again() {
    let mut s = composer("Salut :smi");
    s.escape();
    s.set_text("", 0);
    s.set_text(":smi", 4);
    assert!(!s.suggestions().is_empty());
}

// ------------------------------------------------------------ properties

proptest! {
    #[test]
    fn a_trigger_always_starts_on_its_symbol_inside_the_text(text in "(?s:.{0,24})", cursor in 0usize..30) {
        let cursor = (0..=cursor.min(text.len())).rev().find(|c| text.is_char_boundary(*c)).unwrap_or(0);
        if let Some(t) = find_trigger(&text, cursor) {
            prop_assert!(t.start < cursor);
            prop_assert!(text.is_char_boundary(t.start));
            let symbol = text[t.start..].chars().next().unwrap();
            prop_assert!("@#:/".contains(symbol));
            prop_assert_eq!(&text[t.start + 1..cursor], t.query.as_str());
        }
    }

    #[test]
    fn find_trigger_never_panics_on_any_cursor(text in "(?s:.{0,24})", cursor in 0usize..60) {
        let _ = find_trigger(&text, cursor);
    }

    #[test]
    fn words_alone_ask_for_nothing(text in "[a-z ]{0,20}") {
        prop_assert_eq!(find_trigger(&text, text.len()), None);
    }
}
