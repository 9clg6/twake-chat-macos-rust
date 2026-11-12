//! The catalogue a view model asks for its texts (A006: Fluent).
//!
//! Need (not the code): `docs/dev/ui-screens-api-sketch.md` section 0. Texts come from keys, never from
//! strings in the code; a missing key must not crash a screen.

use twake_i18n::{Arg, HourCycle, Locale};

const FTL: &str = "
hello = Hello
greet = Hi { $name }!
sent = { $count ->
    [one] { $count } message sent
   *[other] { $count } messages sent
}
";

fn locale() -> Locale {
    Locale::from_ftl("en", FTL, 3_600)
}

#[test]
fn a_key_gives_its_text() {
    assert_eq!(locale().tr("hello", &[]), "Hello");
}

#[test]
fn a_string_argument_is_put_in_the_text_without_isolation_marks() {
    let text = locale().tr("greet", &[("name", Arg::Str("Zoé"))]);
    assert_eq!(text, "Hi Zoé!");
}

#[test]
fn a_number_argument_picks_the_plural_form_of_the_language() {
    let one = locale().tr("sent", &[("count", Arg::Num(1))]);
    let many = locale().tr("sent", &[("count", Arg::Num(5))]);
    assert_eq!(
        (one.as_str(), many.as_str()),
        ("1 message sent", "5 messages sent")
    );
}

#[test]
fn an_unknown_key_gives_the_key_itself() {
    assert_eq!(locale().tr("no-such-key", &[]), "no-such-key");
}

#[test]
fn the_reader_has_an_offset_and_a_24_hour_clock_by_default() {
    let locale = locale();
    assert_eq!(locale.utc_offset_secs(), 3_600);
    assert_eq!(locale.hour_cycle(), HourCycle::H24);
    assert_eq!(
        locale.with_hour_cycle(HourCycle::H12).hour_cycle(),
        HourCycle::H12
    );
}
