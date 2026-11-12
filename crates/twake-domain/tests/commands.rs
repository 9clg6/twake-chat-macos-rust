//! Slash commands of the client, commands of the bots of the room, and their suggestions.
//!
//! Need (not the code): FR-061 and `docs/spec/features/composer.md`
//! - D27 + e2e "Composer": `/foo bar` is refused ("This command does not exist"), `//foo bar`
//!   sends `/foo bar`; a bot command is sent as `!leave 2 dentist X`; typing `/le` offers the
//!   commands whose name starts with it, in the order the bot announced them.
//! - FR-061: `/me`, `/shrug`, `//`; `/discardsession` runs on the device and sends nothing.
//!
//! Public items used are listed in `docs/dev/composer-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_domain::UserId;
use twake_domain::commands::{BotCommand, LocalCommand, Prepared, match_commands, prepare};

const SHRUG: &str = "¯\\_(ツ)_/¯";

fn bot() -> UserId {
    UserId::parse("@bot_alice:example.org").unwrap()
}

fn command(name: &str, syntax: &str) -> BotCommand {
    BotCommand {
        bot: bot(),
        name: name.into(),
        syntax: syntax.into(),
        description: format!("does {name}"),
    }
}

fn bot_commands() -> Vec<BotCommand> {
    vec![
        command("help", "help"),
        command("leave", "leave {days} {reason...}"),
        command("delete", "delete"),
        command("lexicon", "lexicon"),
    ]
}

fn message(body: &str) -> Prepared {
    Prepared::Message { body: body.into() }
}

fn names(found: Vec<&BotCommand>) -> Vec<&str> {
    found.into_iter().map(|c| c.name.as_str()).collect()
}

// ------------------------------------------------------------ plain text

#[test]
fn plain_text_is_a_message() {
    assert_eq!(prepare("hello", &[]), message("hello"));
}

#[test]
fn a_slash_alone_is_a_message() {
    assert_eq!(prepare("/", &[]), message("/"));
}

#[test]
fn a_slash_then_a_space_is_a_message() {
    assert_eq!(prepare("/ hello", &[]), message("/ hello"));
}

#[test]
fn a_path_is_a_message() {
    assert_eq!(prepare("/usr/bin", &[]), message("/usr/bin"));
}

#[test]
fn a_slash_in_the_middle_is_a_message() {
    assert_eq!(prepare("see /help", &bot_commands()), message("see /help"));
}

// ------------------------------------------------------------ double slash

#[test]
fn a_double_slash_sends_one_slash() {
    assert_eq!(prepare("//foo bar", &[]), message("/foo bar"));
}

#[test]
fn a_double_slash_alone_sends_one_slash() {
    assert_eq!(prepare("//", &[]), message("/"));
}

#[test]
fn a_double_slash_escapes_a_bot_command() {
    assert_eq!(prepare("//help", &bot_commands()), message("/help"));
}

// ------------------------------------------------------------ /me and /shrug

#[test]
fn me_is_an_emote() {
    assert_eq!(
        prepare("/me waves hello", &[]),
        Prepared::Emote {
            body: "waves hello".into()
        }
    );
}

#[test]
fn me_ignores_case_and_extra_spaces() {
    assert_eq!(
        prepare("/ME   waves", &[]),
        Prepared::Emote {
            body: "waves".into()
        }
    );
}

#[test]
fn me_alone_is_refused() {
    assert_eq!(prepare("/me", &[]), Prepared::Unknown("me".into()));
}

#[test]
fn shrug_alone_sends_the_shrug() {
    assert_eq!(prepare("/shrug", &[]), message(SHRUG));
}

#[test]
fn shrug_follows_the_text() {
    assert_eq!(
        prepare("/shrug no idea", &[]),
        message(&format!("no idea {SHRUG}"))
    );
}

// ------------------------------------------------------------ unknown

#[test]
fn an_unknown_command_is_refused_with_its_name() {
    assert_eq!(prepare("/foo bar", &[]), Prepared::Unknown("foo".into()));
}

#[test]
fn the_unknown_name_is_lower_case() {
    assert_eq!(
        prepare("/FooBar x", &[]),
        Prepared::Unknown("foobar".into())
    );
}

#[test]
fn a_command_of_another_room_is_unknown() {
    assert_eq!(
        prepare("/leave 2", &[command("help", "help")]),
        Prepared::Unknown("leave".into())
    );
}

// ------------------------------------------------------------ bot commands

#[test]
fn a_bot_command_goes_out_with_a_bang() {
    let commands = bot_commands();
    assert_eq!(
        prepare("/leave 2 dentist X", &commands),
        Prepared::Bot {
            command: commands[1].clone(),
            body: "!leave 2 dentist X".into()
        }
    );
}

#[test]
fn a_bot_command_without_arguments() {
    let commands = bot_commands();
    assert_eq!(
        prepare("/delete", &commands),
        Prepared::Bot {
            command: commands[2].clone(),
            body: "!delete".into()
        }
    );
}

#[test]
fn a_bot_command_name_ignores_case_and_is_a_whole_word() {
    let commands = bot_commands();
    assert!(matches!(prepare("/HELP", &commands), Prepared::Bot { .. }));
    assert_eq!(
        prepare("/helpme", &commands),
        Prepared::Unknown("helpme".into())
    );
}

#[test]
fn a_bot_command_wins_over_the_client_ones() {
    let commands = vec![command("me", "me")];
    assert!(matches!(
        prepare("/me waves", &commands),
        Prepared::Bot { .. }
    ));
}

// ------------------------------------------------------------ local

#[test]
fn discardsession_runs_on_the_device() {
    assert_eq!(
        prepare("/discardsession", &[]),
        Prepared::Local(LocalCommand::DiscardSession)
    );
}

#[test]
fn discardsession_ignores_case_and_what_follows() {
    assert_eq!(
        prepare("/DiscardSession now please", &[]),
        Prepared::Local(LocalCommand::DiscardSession)
    );
}

// ------------------------------------------------------------ suggestions after "/"

#[test]
fn a_bare_slash_offers_every_command_in_announced_order() {
    let commands = bot_commands();
    assert_eq!(
        names(match_commands(&commands, "")),
        ["help", "leave", "delete", "lexicon"]
    );
}

#[test]
fn a_prefix_keeps_the_commands_that_start_with_it() {
    let commands = bot_commands();
    assert_eq!(names(match_commands(&commands, "le")), ["leave", "lexicon"]);
}

#[test]
fn the_prefix_ignores_case() {
    let commands = bot_commands();
    assert_eq!(names(match_commands(&commands, "LEA")), ["leave"]);
}

#[test]
fn a_word_inside_a_name_is_not_a_prefix() {
    let commands = bot_commands();
    assert!(match_commands(&commands, "ave").is_empty());
}

// ------------------------------------------------------------ properties

proptest! {
    #[test]
    fn prepare_never_panics_and_never_makes_an_empty_message_of_text(text in "(?s:.{0,40})") {
        if text.trim().is_empty() {
            return Ok(());
        }
        if let Prepared::Message { body } = prepare(text.trim(), &[]) {
            prop_assert!(!body.is_empty());
        }
    }

    #[test]
    fn text_without_a_leading_slash_is_untouched(text in "[^/](?s:.{0,40})") {
        prop_assert_eq!(prepare(&text, &bot_commands()), Prepared::Message { body: text.clone() });
    }

    #[test]
    fn a_double_slash_drops_exactly_one(rest in "(?s:.{0,40})") {
        let text = format!("//{rest}");
        prop_assert_eq!(prepare(&text, &bot_commands()), Prepared::Message { body: format!("/{rest}") });
    }
}
