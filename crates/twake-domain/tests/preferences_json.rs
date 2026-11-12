//! The preferences document: stable, tolerant, versioned.
//!
//! Need (not the code): the React `readDisplayPreferences` reads a stored object field by field
//! and takes the default of each field that is missing or invalid; a broken store gives the
//! defaults, never a failure (a settings screen must open). D67: an unknown language counts as
//! none. The v1 -> v2 rules are in `docs/dev/settings-api-sketch.md` (section 1).
//!
//! Public items used are listed in `docs/dev/settings-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use serde_json::{Value, json};
use twake_domain::preferences::{
    BubbleAlign, Density, GlobalNotificationLevel, HourCycle, Language, Preferences, RoomOrder,
    SCHEMA_VERSION, SUPPORTED_LANGUAGES, TextSize, ThemeChoice, Wallpaper,
};

/// Every field away from its default, with values that differ from each other.
fn every_choice() -> Preferences {
    Preferences {
        theme: ThemeChoice::Dark,
        density: Density::Compact,
        language: Language::Code("vi"),
        time_format: HourCycle::H12,
        text_size: TextSize::Small,
        room_order: RoomOrder::Sections,
        wallpaper: Wallpaper::None,
        bubble_align: BubbleAlign::Left,
        grayscale: true,
        high_contrast: Some(true),
        do_not_disturb: true,
        autoplay: Some(false),
        private_receipts: true,
        direct_messages: GlobalNotificationLevel::Mentions,
        groups: GlobalNotificationLevel::All,
        message_sound: false,
        notification_text: false,
    }
}

/// Direct messages at every message, groups at mentions: the other way round from `every_choice`.
fn groups_quiet() -> Preferences {
    Preferences {
        direct_messages: GlobalNotificationLevel::All,
        groups: GlobalNotificationLevel::Mentions,
        ..every_choice()
    }
}

fn doc(p: &Preferences) -> Value {
    serde_json::from_str(&p.to_json()).unwrap()
}

fn read(v: Value) -> Preferences {
    Preferences::from_json(&v.to_string())
}

// ---------------------------------------------------------------- shape

#[test]
fn the_schema_version_is_two() {
    assert_eq!(SCHEMA_VERSION, 2);
}

#[test]
fn the_document_names_every_choice_with_the_documented_words() {
    assert_eq!(
        doc(&every_choice()),
        json!({
            "version": 2,
            "theme": "dark",
            "density": "compact",
            "language": "vi",
            "time_format": "12h",
            "text_size": "small",
            "room_order": "sections",
            "wallpaper": "none",
            "bubble_align": "left",
            "grayscale": true,
            "high_contrast": true,
            "do_not_disturb": true,
            "autoplay": false,
            "private_receipts": true,
            "direct_messages": "mentions",
            "groups": "all",
            "message_sound": false,
            "notification_text": false
        })
    );
}

#[test]
fn the_default_document_uses_the_other_words() {
    assert_eq!(
        doc(&Preferences::default()),
        json!({
            "version": 2,
            "theme": "system",
            "density": "comfortable",
            "language": "system",
            "time_format": "system",
            "text_size": "normal",
            "room_order": "recent",
            "wallpaper": "default",
            "bubble_align": "own-right",
            "grayscale": false,
            "high_contrast": null,
            "do_not_disturb": false,
            "autoplay": null,
            "private_receipts": false,
            "direct_messages": "all",
            "groups": "all",
            "message_sound": true,
            "notification_text": true
        })
    );
}

#[test]
fn the_remaining_words_round_trip() {
    let other = Preferences {
        theme: ThemeChoice::Light,
        time_format: HourCycle::H24,
        text_size: TextSize::Large,
        high_contrast: Some(false),
        autoplay: Some(true),
        ..Preferences::default()
    };
    let d = doc(&other);
    assert_eq!(d["theme"], "light");
    assert_eq!(d["time_format"], "24h");
    assert_eq!(d["text_size"], "large");
    assert_eq!(d["high_contrast"], false);
    assert_eq!(d["autoplay"], true);
    assert_eq!(Preferences::from_json(&other.to_json()), other);
}

#[test]
fn the_same_value_gives_the_same_text() {
    assert_eq!(every_choice().to_json(), every_choice().to_json());
}

// ---------------------------------------------------------------- tolerance

#[test]
fn text_that_is_not_json_gives_the_defaults() {
    assert_eq!(Preferences::from_json("{oops"), Preferences::default());
    assert_eq!(Preferences::from_json(""), Preferences::default());
}

#[test]
fn json_that_is_not_an_object_gives_the_defaults() {
    for text in ["null", "[]", "42", "\"dark\"", "true"] {
        assert_eq!(
            Preferences::from_json(text),
            Preferences::default(),
            "{text}"
        );
    }
}

#[test]
fn an_empty_object_gives_the_defaults() {
    assert_eq!(Preferences::from_json("{}"), Preferences::default());
}

#[test]
fn unknown_keys_are_ignored() {
    let mut d = doc(&every_choice());
    d["from_the_future"] = json!({"a": [1, 2]});
    d["clock"] = json!("24h");
    assert_eq!(read(d), every_choice());
}

#[test]
fn a_missing_key_takes_the_default_of_that_field_alone() {
    let mut d = doc(&every_choice());
    d.as_object_mut().unwrap().remove("density");
    assert_eq!(
        read(d),
        Preferences {
            density: Density::Comfortable,
            ..every_choice()
        }
    );
}

#[test]
fn an_invalid_value_takes_the_default_of_that_field_alone() {
    let cases: [(&str, Value, Preferences); 5] = [
        (
            "density",
            json!(42),
            Preferences {
                density: Density::Comfortable,
                ..every_choice()
            },
        ),
        (
            "theme",
            json!("sepia"),
            Preferences {
                theme: ThemeChoice::System,
                ..every_choice()
            },
        ),
        (
            "grayscale",
            json!("yes"),
            Preferences {
                grayscale: false,
                ..every_choice()
            },
        ),
        (
            "message_sound",
            json!(null),
            Preferences {
                message_sound: true,
                ..every_choice()
            },
        ),
        (
            "high_contrast",
            json!("more"),
            Preferences {
                high_contrast: None,
                ..every_choice()
            },
        ),
    ];
    for (key, bad, want) in cases {
        let mut d = doc(&every_choice());
        d[key] = bad;
        assert_eq!(read(d), want, "{key}");
    }
}

#[test]
fn an_unsupported_language_is_the_system_one() {
    for bad in [
        json!("xx"),
        json!("FR"),
        json!("fr-FR"),
        json!(7),
        json!(""),
    ] {
        let mut d = doc(&every_choice());
        d["language"] = bad.clone();
        assert_eq!(read(d).language, Language::System, "{bad}");
    }
}

#[test]
fn every_supported_language_is_read_back() {
    for code in SUPPORTED_LANGUAGES {
        let mut d = doc(&every_choice());
        d["language"] = json!(code);
        assert_eq!(read(d).language, Language::Code(code));
    }
}

// ---------------------------------------------------------------- versions

fn v1() -> Value {
    json!({
        "version": 1,
        "clock": "24h",
        "room_order": "category",
        "density": "compact",
        "theme": "dark"
    })
}

#[test]
fn a_v1_document_is_read_with_the_renamed_keys_and_words() {
    let p = read(v1());
    assert_eq!(p.time_format, HourCycle::H24);
    assert_eq!(p.room_order, RoomOrder::Sections);
    assert_eq!(p.density, Density::Compact);
    assert_eq!(p.theme, ThemeChoice::Dark);
}

#[test]
fn a_v1_document_gets_the_defaults_of_the_other_fields() {
    let p = read(v1());
    assert_eq!(
        p,
        Preferences {
            theme: ThemeChoice::Dark,
            density: Density::Compact,
            time_format: HourCycle::H24,
            room_order: RoomOrder::Sections,
            ..Preferences::default()
        }
    );
}

#[test]
fn a_document_without_a_version_is_v1() {
    let mut d = v1();
    d.as_object_mut().unwrap().remove("version");
    assert_eq!(read(d), read(v1()));
}

#[test]
fn the_v1_auto_clock_and_the_12_hour_clock_read_as_the_v2_words() {
    let auto = read(json!({"version": 1, "clock": "auto"}));
    assert_eq!(auto.time_format, HourCycle::System);
    let h12 = read(json!({"version": 1, "clock": "12h"}));
    assert_eq!(h12.time_format, HourCycle::H12);
}

#[test]
fn a_migrated_document_is_written_back_as_v2() {
    let migrated = doc(&read(v1()));
    assert_eq!(migrated["version"], 2);
    assert_eq!(migrated["time_format"], "24h");
    assert_eq!(migrated["room_order"], "sections");
    assert!(migrated.get("clock").is_none());
}

#[test]
fn in_v2_the_old_key_and_the_old_word_mean_nothing() {
    let p = read(json!({"version": 2, "clock": "24h", "room_order": "category"}));
    assert_eq!(p.time_format, HourCycle::System);
    assert_eq!(p.room_order, RoomOrder::Recent);
}

#[test]
fn in_v1_the_v2_key_is_not_read() {
    let p = read(json!({"version": 1, "time_format": "24h"}));
    assert_eq!(p.time_format, HourCycle::System);
}

#[test]
fn a_document_from_a_future_version_is_read_with_the_v2_rules() {
    let mut d = doc(&every_choice());
    d["version"] = json!(3);
    d["extra"] = json!(true);
    assert_eq!(read(d), every_choice());
}

// ---------------------------------------------------------------- notification levels (D22)

#[test]
fn an_invalid_level_takes_the_default_of_that_level_alone() {
    let mut d = doc(&every_choice());
    d["direct_messages"] = json!("none");
    assert_eq!(read(d).direct_messages, GlobalNotificationLevel::All);
    let mut d = doc(&groups_quiet());
    d["groups"] = json!(3);
    let p = read(d);
    assert_eq!(p.groups, GlobalNotificationLevel::All);
    assert_eq!(p.direct_messages, GlobalNotificationLevel::All);
}

#[test]
fn the_two_levels_round_trip_each_on_its_own() {
    for p in [every_choice(), groups_quiet()] {
        assert_eq!(Preferences::from_json(&p.to_json()), p);
    }
    assert_eq!(doc(&groups_quiet())["direct_messages"], "all");
    assert_eq!(doc(&groups_quiet())["groups"], "mentions");
}

#[test]
fn a_missing_level_takes_the_default_of_that_level_alone() {
    let mut d = doc(&every_choice());
    d.as_object_mut().unwrap().remove("direct_messages");
    assert_eq!(read(d).direct_messages, GlobalNotificationLevel::All);
    let mut d = doc(&groups_quiet());
    d.as_object_mut().unwrap().remove("groups");
    let p = read(d);
    assert_eq!(p.groups, GlobalNotificationLevel::All);
    assert_eq!(p.direct_messages, GlobalNotificationLevel::All);
}

#[test]
fn the_former_single_level_sets_both_levels() {
    let mut d = doc(&Preferences::default());
    let fields = d.as_object_mut().unwrap();
    fields.remove("direct_messages");
    fields.remove("groups");
    fields.insert("notifications".into(), json!("mentions"));
    let p = read(d);
    assert_eq!(p.direct_messages, GlobalNotificationLevel::Mentions);
    assert_eq!(p.groups, GlobalNotificationLevel::Mentions);
}

#[test]
fn the_former_single_level_is_migrated_in_a_v1_document_too() {
    let p = read(json!({"version": 1, "notifications": "mentions"}));
    assert_eq!(p.direct_messages, GlobalNotificationLevel::Mentions);
    assert_eq!(p.groups, GlobalNotificationLevel::Mentions);
}

#[test]
fn a_level_of_its_own_wins_over_the_former_single_level() {
    let mut d = doc(&groups_quiet());
    d["notifications"] = json!("all");
    assert_eq!(read(d), groups_quiet());
    let mut d = doc(&every_choice());
    d["notifications"] = json!("mentions");
    assert_eq!(read(d), every_choice());
}

#[test]
fn an_invalid_level_takes_its_default_even_when_the_former_key_is_there() {
    let p = read(json!({
        "version": 2,
        "notifications": "mentions",
        "direct_messages": "none",
        "groups": 3
    }));
    assert_eq!(p.direct_messages, GlobalNotificationLevel::All);
    assert_eq!(p.groups, GlobalNotificationLevel::All);
    let p = read(json!({
        "version": 2,
        "notifications": "mentions",
        "direct_messages": "mentions",
        "groups": "none"
    }));
    assert_eq!(p.direct_messages, GlobalNotificationLevel::Mentions);
    assert_eq!(p.groups, GlobalNotificationLevel::All);
}

#[test]
fn an_invalid_former_single_level_gives_the_defaults() {
    let p = read(json!({"version": 2, "notifications": "none"}));
    assert_eq!(p.direct_messages, GlobalNotificationLevel::All);
    assert_eq!(p.groups, GlobalNotificationLevel::All);
}

#[test]
fn a_migrated_document_is_written_without_the_former_key() {
    let migrated = doc(&read(json!({"version": 2, "notifications": "mentions"})));
    assert!(migrated.get("notifications").is_none());
    assert_eq!(migrated["direct_messages"], "mentions");
    assert_eq!(migrated["groups"], "mentions");
    assert_eq!(migrated["version"], 2);
}

// ---------------------------------------------------------------- properties

fn choice<T: Clone + std::fmt::Debug + 'static>(all: &[T]) -> impl Strategy<Value = T> {
    prop::sample::select(all.to_vec())
}

fn any_preferences() -> impl Strategy<Value = Preferences> {
    let language = prop_oneof![
        Just(Language::System),
        prop::sample::select(SUPPORTED_LANGUAGES.to_vec()).prop_map(Language::Code),
    ];
    let tri = || prop::option::of(any::<bool>());
    let a = (
        choice(&[ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark]),
        choice(&[Density::Comfortable, Density::Compact]),
        language,
        choice(&[HourCycle::System, HourCycle::H12, HourCycle::H24]),
        choice(&[TextSize::Small, TextSize::Normal, TextSize::Large]),
        choice(&[RoomOrder::Recent, RoomOrder::Sections]),
        choice(&[Wallpaper::Default, Wallpaper::None]),
        choice(&[BubbleAlign::OwnRight, BubbleAlign::Left]),
    );
    let b = (
        any::<bool>(),
        tri(),
        any::<bool>(),
        tri(),
        any::<bool>(),
        choice(&[
            GlobalNotificationLevel::All,
            GlobalNotificationLevel::Mentions,
        ]),
        any::<bool>(),
        any::<bool>(),
        choice(&[
            GlobalNotificationLevel::All,
            GlobalNotificationLevel::Mentions,
        ]),
    );
    (a, b).prop_map(|(a, b)| Preferences {
        theme: a.0,
        density: a.1,
        language: a.2,
        time_format: a.3,
        text_size: a.4,
        room_order: a.5,
        wallpaper: a.6,
        bubble_align: a.7,
        grayscale: b.0,
        high_contrast: b.1,
        do_not_disturb: b.2,
        autoplay: b.3,
        private_receipts: b.4,
        direct_messages: b.5,
        groups: b.8,
        message_sound: b.6,
        notification_text: b.7,
    })
}

fn any_json() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::from),
        any::<i64>().prop_map(Value::from),
        any::<String>().prop_map(Value::from),
        prop::sample::select(vec![
            "system", "light", "dark", "compact", "24h", "12h", "fr", "category", "none", "left",
        ])
        .prop_map(Value::from),
    ];
    leaf.prop_recursive(2, 8, 3, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..3).prop_map(Value::Array),
            prop::collection::hash_map(any::<String>(), inner, 0..3)
                .prop_map(|m| Value::Object(m.into_iter().collect())),
        ]
    })
}

fn any_document() -> impl Strategy<Value = Value> {
    let keys = prop::sample::select(vec![
        "version",
        "clock",
        "theme",
        "density",
        "language",
        "time_format",
        "text_size",
        "room_order",
        "wallpaper",
        "bubble_align",
        "grayscale",
        "high_contrast",
        "do_not_disturb",
        "autoplay",
        "private_receipts",
        "notifications",
        "direct_messages",
        "groups",
        "message_sound",
        "notification_text",
    ]);
    prop::collection::hash_map(keys.prop_map(String::from), any_json(), 0..12)
        .prop_map(|m| Value::Object(m.into_iter().collect()))
}

proptest! {
    #[test]
    fn what_is_written_is_read_back(p in any_preferences()) {
        prop_assert_eq!(Preferences::from_json(&p.to_json()), p);
    }

    #[test]
    fn any_text_reads_without_panic(text in any::<String>()) {
        let _ = Preferences::from_json(&text);
    }

    #[test]
    fn any_document_reads_without_panic_and_settles(d in any_document()) {
        let once = Preferences::from_json(&d.to_string());
        prop_assert_eq!(Preferences::from_json(&once.to_json()), once);
    }
}
