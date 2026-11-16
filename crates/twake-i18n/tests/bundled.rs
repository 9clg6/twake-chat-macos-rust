//! The bundled catalogues: every key of the sketch exists in both languages.

use twake_i18n::{Arg, Locale};

const KEYS: [&str; 18] = [
    "room-preview-you",
    "room-preview-author",
    "time-yesterday",
    "weekday-short-mon",
    "weekday-short-tue",
    "weekday-short-wed",
    "weekday-short-thu",
    "weekday-short-fri",
    "weekday-short-sat",
    "weekday-short-sun",
    "date-short",
    "time-12h",
    "period-am",
    "period-pm",
    "day-today",
    "day-yesterday",
    "new-messages",
    "message-edited",
];

#[test]
fn every_key_is_translated_in_english_and_in_french() {
    for lang in ["en", "fr"] {
        let locale = Locale::bundled(lang, 0);
        for key in KEYS {
            assert_ne!(locale.tr(key, &[]), key, "{key} missing in {lang}");
        }
    }
}

#[test]
fn a_regional_french_and_an_unknown_language_pick_their_catalogue() {
    assert_eq!(Locale::bundled("fr-CA", 0).tr("day-yesterday", &[]), "Hier");
    assert_eq!(
        Locale::bundled("de", 0).tr("day-yesterday", &[]),
        "Yesterday"
    );
    let args = [("text", Arg::Str("ok"))];
    assert_eq!(
        Locale::bundled("fr", 0).tr("room-preview-you", &args),
        "Vous : ok"
    );
}
