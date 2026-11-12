//! The preferences: defaults, and how the "as the system" choices resolve.
//!
//! Need (not the code): FR-128 (theme light/dark/system, density, text size, room order,
//! wallpaper, high contrast, 24 h clock), FR-129 (private receipts, autoplay), settings.md
//! D38 (theme), D67 (`fr-FR` is French, an unknown language counts as none), D240 (sound on by
//! default), D259 (autoplay follows reduced motion until chosen), D261/D272 (text size 0.9/1/1.15),
//! D349 (every choice applies everywhere: one derived view), D437 (high contrast: plain paper
//! wallpaper). Defaults are those of the React `readDisplayPreferences`.
//!
//! Public items used are listed in `docs/dev/settings-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use twake_domain::preferences::{
    BubbleAlign, ColorScheme, Density, GlobalNotificationLevel, HourCycle, Language, Preferences,
    RoomOrder, SUPPORTED_LANGUAGES, SystemEnv, TextSize, ThemeChoice, ViewSettings, Wallpaper,
};

fn prefs() -> Preferences {
    Preferences::default()
}

#[test]
fn a_new_user_gets_the_defaults_of_the_spec() {
    let p = prefs();
    assert_eq!(p.theme, ThemeChoice::System);
    assert_eq!(p.density, Density::Comfortable);
    assert_eq!(p.language, Language::System);
    assert_eq!(p.time_format, HourCycle::System);
    assert_eq!(p.text_size, TextSize::Normal);
    assert_eq!(p.room_order, RoomOrder::Recent);
    assert_eq!(p.wallpaper, Wallpaper::Default);
    assert_eq!(p.bubble_align, BubbleAlign::OwnRight);
}

#[test]
fn the_switches_start_as_the_spec_says() {
    let p = prefs();
    assert!(!p.grayscale);
    assert_eq!(p.high_contrast, None);
    assert!(!p.do_not_disturb);
    assert_eq!(p.autoplay, None);
    assert!(!p.private_receipts);
    assert_eq!(p.direct_messages, GlobalNotificationLevel::All);
    assert_eq!(p.groups, GlobalNotificationLevel::All);
    assert!(p.message_sound);
    assert!(p.notification_text);
}

#[test]
fn the_supported_languages_are_those_of_the_react_app() {
    assert_eq!(
        SUPPORTED_LANGUAGES,
        ["en", "fr", "de", "es", "it", "ru", "vi"]
    );
}

// ---------------------------------------------------------------- theme

#[test]
fn the_system_theme_follows_the_os() {
    assert_eq!(prefs().resolve_theme(true), ColorScheme::Dark);
    assert_eq!(prefs().resolve_theme(false), ColorScheme::Light);
}

#[test]
fn a_chosen_theme_ignores_the_os() {
    let light = Preferences {
        theme: ThemeChoice::Light,
        ..prefs()
    };
    let dark = Preferences {
        theme: ThemeChoice::Dark,
        ..prefs()
    };
    assert_eq!(light.resolve_theme(true), ColorScheme::Light);
    assert_eq!(dark.resolve_theme(false), ColorScheme::Dark);
}

// ---------------------------------------------------------------- contrast, autoplay

#[test]
fn high_contrast_follows_the_system_until_chosen() {
    assert!(prefs().resolve_high_contrast(true));
    assert!(!prefs().resolve_high_contrast(false));
}

#[test]
fn a_chosen_high_contrast_wins_over_the_system_either_way() {
    let off = Preferences {
        high_contrast: Some(false),
        ..prefs()
    };
    let on = Preferences {
        high_contrast: Some(true),
        ..prefs()
    };
    assert!(!off.resolve_high_contrast(true));
    assert!(on.resolve_high_contrast(false));
}

#[test]
fn autoplay_is_off_while_the_user_reduces_motion_and_has_not_chosen() {
    assert!(prefs().resolve_autoplay(false));
    assert!(!prefs().resolve_autoplay(true));
}

#[test]
fn a_chosen_autoplay_wins_over_reduced_motion_either_way() {
    let on = Preferences {
        autoplay: Some(true),
        ..prefs()
    };
    let off = Preferences {
        autoplay: Some(false),
        ..prefs()
    };
    assert!(on.resolve_autoplay(true));
    assert!(!off.resolve_autoplay(false));
}

// ---------------------------------------------------------------- language

#[test]
fn a_chosen_language_is_used_whatever_the_os_says() {
    let p = Preferences {
        language: Language::Code("vi"),
        ..prefs()
    };
    assert_eq!(p.resolve_language("fr-FR"), "vi");
}

#[test]
fn the_system_language_is_its_base_code_when_supported() {
    assert_eq!(prefs().resolve_language("fr-FR"), "fr");
    assert_eq!(prefs().resolve_language("de_AT"), "de");
    assert_eq!(prefs().resolve_language("ES"), "es");
    assert_eq!(prefs().resolve_language("ru"), "ru");
}

#[test]
fn an_unsupported_or_empty_system_language_is_english() {
    assert_eq!(prefs().resolve_language("ja-JP"), "en");
    assert_eq!(prefs().resolve_language("pt-BR"), "en");
    assert_eq!(prefs().resolve_language(""), "en");
}

#[test]
fn a_code_that_only_starts_like_a_supported_one_is_not_supported() {
    assert_eq!(prefs().resolve_language("fra"), "en");
    assert_eq!(prefs().resolve_language("english"), "en");
}

// ---------------------------------------------------------------- clock, text size

#[test]
fn the_clock_follows_the_os_until_chosen() {
    assert!(prefs().resolve_hour_24(true));
    assert!(!prefs().resolve_hour_24(false));
}

#[test]
fn a_chosen_clock_wins_over_the_os() {
    let h12 = Preferences {
        time_format: HourCycle::H12,
        ..prefs()
    };
    let h24 = Preferences {
        time_format: HourCycle::H24,
        ..prefs()
    };
    assert!(!h12.resolve_hour_24(true));
    assert!(h24.resolve_hour_24(false));
}

#[test]
fn text_sizes_scale_by_0_9_1_and_1_15() {
    assert_eq!(TextSize::Small.scale(), 0.9);
    assert_eq!(TextSize::Normal.scale(), 1.0);
    assert_eq!(TextSize::Large.scale(), 1.15);
}

// ---------------------------------------------------------------- the one view

fn every_choice() -> Preferences {
    Preferences {
        theme: ThemeChoice::Dark,
        density: Density::Compact,
        language: Language::Code("it"),
        time_format: HourCycle::H12,
        text_size: TextSize::Large,
        room_order: RoomOrder::Sections,
        wallpaper: Wallpaper::Default,
        bubble_align: BubbleAlign::Left,
        grayscale: true,
        high_contrast: Some(false),
        do_not_disturb: true,
        autoplay: Some(false),
        ..prefs()
    }
}

#[test]
fn the_view_carries_every_choice_of_the_user() {
    assert_eq!(
        every_choice().view(&SystemEnv::default()),
        ViewSettings {
            scheme: ColorScheme::Dark,
            density: Density::Compact,
            text_scale: 1.15,
            language: "it",
            hour_24: false,
            high_contrast: false,
            grayscale: true,
            autoplay: false,
            wallpaper: Wallpaper::Default,
            room_order: RoomOrder::Sections,
            bubble_align: BubbleAlign::Left,
            do_not_disturb: true,
            appearance_locked: false,
            common_settings_url: None,
        }
    );
}

#[test]
fn the_view_reads_the_os_where_the_user_chose_nothing() {
    let env = SystemEnv {
        dark: true,
        high_contrast: true,
        reduced_motion: true,
        hour_24: true,
        language: "de-DE".into(),
        common: None,
    };
    let v = prefs().view(&env);
    assert_eq!(v.scheme, ColorScheme::Dark);
    assert!(v.high_contrast);
    assert!(!v.autoplay);
    assert!(v.hour_24);
    assert_eq!(v.language, "de");
}

#[test]
fn high_contrast_draws_the_plain_paper_wallpaper() {
    let on = Preferences {
        high_contrast: Some(true),
        ..prefs()
    };
    assert_eq!(on.view(&SystemEnv::default()).wallpaper, Wallpaper::None);
    let from_os = SystemEnv {
        high_contrast: true,
        ..SystemEnv::default()
    };
    assert_eq!(prefs().view(&from_os).wallpaper, Wallpaper::None);
}

#[test]
fn without_high_contrast_the_chosen_wallpaper_stays() {
    let none = Preferences {
        wallpaper: Wallpaper::None,
        ..prefs()
    };
    assert_eq!(none.view(&SystemEnv::default()).wallpaper, Wallpaper::None);
    assert_eq!(
        prefs().view(&SystemEnv::default()).wallpaper,
        Wallpaper::Default
    );
}

#[test]
fn a_density_change_shows_in_the_view_and_nowhere_else_changes() {
    let env = SystemEnv::default();
    let before = every_choice().view(&env);
    let compact_off = Preferences {
        density: Density::Comfortable,
        ..every_choice()
    };
    let after = compact_off.view(&env);
    assert_eq!(after.density, Density::Comfortable);
    assert_eq!(
        after,
        ViewSettings {
            density: Density::Comfortable,
            ..before
        }
    );
}

#[test]
fn the_view_is_the_same_for_every_screen_that_asks() {
    let env = SystemEnv {
        dark: true,
        ..SystemEnv::default()
    };
    assert_eq!(every_choice().view(&env), every_choice().view(&env));
}
