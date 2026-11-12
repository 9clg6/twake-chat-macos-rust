//! The settings state: every change applies at once, is dirty until stored, can be cancelled, and
//! reaches every screen through the one derived view.
//!
//! Need (not the code): settings.md D261/D272 ("each applied at once"), D349 ("every
//! personalisation choice applies everywhere"; lists, rows, cards, times read the same choices),
//! FR-128, FR-133. A save is asynchronous: what the user keeps editing meanwhile stays unsaved.
//!
//! Public items used are listed in `docs/dev/settings-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use twake_app::settings::SettingsState;
use twake_domain::preferences::{
    BubbleAlign, ColorScheme, CommonSettings, CommonTheme, Density, GlobalNotificationLevel,
    HourCycle, Language, Preferences, RoomOrder, SystemEnv, TextSize, ThemeChoice, Wallpaper,
};

fn fresh() -> SettingsState {
    SettingsState::new(Preferences::default())
}

fn compact() -> Preferences {
    Preferences {
        density: Density::Compact,
        ..Preferences::default()
    }
}

#[test]
fn a_new_state_shows_what_was_stored_and_is_clean() {
    let stored = Preferences {
        theme: ThemeChoice::Dark,
        ..Preferences::default()
    };
    let s = SettingsState::new(stored.clone());
    assert_eq!(s.preferences(), &stored);
    assert_eq!(s.saved(), &stored);
    assert!(!s.is_dirty());
}

#[test]
fn a_change_applies_at_once_and_makes_the_state_dirty() {
    let mut s = fresh();
    s.set_density(Density::Compact);
    assert_eq!(s.preferences().density, Density::Compact);
    assert_eq!(s.saved().density, Density::Comfortable);
    assert!(s.is_dirty());
}

#[test]
fn every_setter_changes_its_own_field_only() {
    let mut s = fresh();
    s.set_theme(ThemeChoice::Dark);
    s.set_density(Density::Compact);
    s.set_language(Language::Code("de"));
    s.set_time_format(HourCycle::H12);
    s.set_text_size(TextSize::Large);
    s.set_room_order(RoomOrder::Sections);
    s.set_wallpaper(Wallpaper::None);
    s.set_bubble_align(BubbleAlign::Left);
    s.set_grayscale(true);
    s.set_high_contrast(Some(true));
    s.set_do_not_disturb(true);
    s.set_autoplay(Some(false));
    s.set_private_receipts(true);
    s.set_direct_messages(GlobalNotificationLevel::Mentions);
    s.set_groups(GlobalNotificationLevel::All);
    s.set_message_sound(false);
    s.set_notification_text(false);
    assert_eq!(
        s.preferences(),
        &Preferences {
            theme: ThemeChoice::Dark,
            density: Density::Compact,
            language: Language::Code("de"),
            time_format: HourCycle::H12,
            text_size: TextSize::Large,
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
    );
}

#[test]
fn setting_the_value_it_already_has_is_not_a_change() {
    let mut s = fresh();
    s.set_density(Density::Comfortable);
    s.set_theme(ThemeChoice::System);
    assert!(!s.is_dirty());
}

#[test]
fn changing_back_to_the_stored_value_is_clean_again() {
    let mut s = fresh();
    s.set_density(Density::Compact);
    s.set_density(Density::Comfortable);
    assert!(!s.is_dirty());
}

#[test]
fn cancel_gives_back_the_stored_choices() {
    let mut s = fresh();
    s.set_theme(ThemeChoice::Dark);
    s.set_text_size(TextSize::Small);
    s.cancel();
    assert_eq!(s.preferences(), &Preferences::default());
    assert!(!s.is_dirty());
}

#[test]
fn cancel_on_a_clean_state_changes_nothing() {
    let mut s = SettingsState::new(compact());
    s.cancel();
    assert_eq!(s.preferences(), &compact());
    assert_eq!(s.saved(), &compact());
}

// ---------------------------------------------------------------- saving

#[test]
fn once_the_document_is_stored_the_state_is_clean() {
    let mut s = fresh();
    s.set_density(Density::Compact);
    let written = s.preferences().clone();
    s.mark_saved(written);
    assert!(!s.is_dirty());
    assert_eq!(s.saved(), &compact());
    assert_eq!(s.preferences(), &compact());
}

#[test]
fn cancel_after_a_save_keeps_what_was_saved() {
    let mut s = fresh();
    s.set_density(Density::Compact);
    s.mark_saved(compact());
    s.set_theme(ThemeChoice::Dark);
    s.cancel();
    assert_eq!(s.preferences(), &compact());
}

#[test]
fn an_edit_made_while_the_save_runs_stays_unsaved() {
    let mut s = fresh();
    s.set_density(Density::Compact);
    let written = s.preferences().clone();
    s.set_theme(ThemeChoice::Dark);
    s.mark_saved(written);
    assert!(s.is_dirty());
    assert_eq!(s.saved(), &compact());
    assert_eq!(s.preferences().theme, ThemeChoice::Dark);
    assert_eq!(s.preferences().density, Density::Compact);
}

// ---------------------------------------------------------------- another writer

#[test]
fn a_document_stored_elsewhere_replaces_a_clean_state() {
    let mut s = fresh();
    s.apply_external(compact());
    assert_eq!(s.preferences(), &compact());
    assert_eq!(s.saved(), &compact());
    assert!(!s.is_dirty());
}

#[test]
fn a_document_stored_elsewhere_does_not_overwrite_pending_edits() {
    let mut s = fresh();
    s.set_theme(ThemeChoice::Dark);
    s.apply_external(compact());
    assert_eq!(s.preferences().theme, ThemeChoice::Dark);
    assert_eq!(s.saved(), &compact());
    assert!(s.is_dirty());
}

#[test]
fn the_echo_of_our_own_save_leaves_the_state_clean() {
    let mut s = fresh();
    s.set_density(Density::Compact);
    s.mark_saved(compact());
    s.apply_external(compact());
    assert!(!s.is_dirty());
    assert_eq!(s.preferences(), &compact());
}

#[test]
fn a_stored_document_equal_to_the_pending_edits_makes_the_state_clean() {
    let mut s = fresh();
    s.set_density(Density::Compact);
    s.apply_external(compact());
    assert!(!s.is_dirty());
}

// ---------------------------------------------------------------- one view for every screen

#[test]
fn the_view_is_derived_from_the_current_choices() {
    let mut s = fresh();
    let env = SystemEnv::default();
    s.set_density(Density::Compact);
    s.set_theme(ThemeChoice::Dark);
    assert_eq!(s.view(&env), s.preferences().view(&env));
    assert_eq!(s.view(&env).density, Density::Compact);
    assert_eq!(s.view(&env).scheme, ColorScheme::Dark);
}

#[test]
fn a_density_change_reaches_the_view_before_anything_is_saved() {
    let mut s = fresh();
    let env = SystemEnv::default();
    assert_eq!(s.view(&env).density, Density::Comfortable);
    s.set_density(Density::Compact);
    assert!(s.is_dirty());
    assert_eq!(s.view(&env).density, Density::Compact);
}

#[test]
fn cancelling_takes_the_view_back_too() {
    let mut s = fresh();
    let env = SystemEnv::default();
    s.set_density(Density::Compact);
    s.cancel();
    assert_eq!(s.view(&env).density, Density::Comfortable);
}

#[test]
fn every_screen_that_asks_gets_the_same_view() {
    let mut s = fresh();
    let env = SystemEnv::default();
    s.set_density(Density::Compact);
    let room_list = s.view(&env);
    let contacts = s.view(&env);
    let settings = s.view(&env);
    assert_eq!(room_list, contacts);
    assert_eq!(contacts, settings);
    assert_eq!(settings.density, Density::Compact);
}

#[test]
fn the_os_going_dark_changes_the_view_not_the_choices() {
    let s = fresh();
    let light = SystemEnv::default();
    let dark = SystemEnv {
        dark: true,
        ..SystemEnv::default()
    };
    assert_eq!(s.view(&light).scheme, ColorScheme::Light);
    assert_eq!(s.view(&dark).scheme, ColorScheme::Dark);
    assert!(!s.is_dirty());
}

#[test]
fn a_document_from_another_writer_reaches_the_view() {
    let mut s = fresh();
    s.apply_external(compact());
    assert_eq!(s.view(&SystemEnv::default()).density, Density::Compact);
}

// ---------------------------------------------------------------- common settings (D38)

fn common(theme: CommonTheme) -> SystemEnv {
    SystemEnv {
        common: Some(CommonSettings {
            theme: Some(theme),
            language: None,
            url: None,
        }),
        ..SystemEnv::default()
    }
}

fn common_language(language: &str) -> SystemEnv {
    SystemEnv {
        common: Some(CommonSettings {
            theme: None,
            language: Some(language.to_string()),
            url: None,
        }),
        ..SystemEnv::default()
    }
}

#[test]
fn a_language_picked_while_the_common_language_wins_is_kept_and_comes_back() {
    let mut s = fresh();
    s.set_language(Language::Code("de"));
    let held = common_language("fr-FR");
    assert_eq!(s.view(&held).language, "fr");
    assert!(s.view(&held).language_locked);
    assert_eq!(s.preferences().language, Language::Code("de"));
    let gone = SystemEnv::default();
    assert_eq!(s.view(&gone).language, "de");
    assert!(!s.view(&gone).language_locked);
}

#[test]
fn a_theme_picked_while_the_common_theme_wins_is_kept_but_not_shown() {
    let mut s = fresh();
    s.set_theme(ThemeChoice::Dark);
    let env = common(CommonTheme::Light);
    assert_eq!(s.view(&env).scheme, ColorScheme::Light);
    assert!(s.view(&env).appearance_locked);
    assert_eq!(s.preferences().theme, ThemeChoice::Dark);
    assert!(s.is_dirty());
}

#[test]
fn the_local_theme_comes_back_when_the_common_theme_goes() {
    let mut s = fresh();
    s.set_theme(ThemeChoice::Dark);
    assert_eq!(
        s.view(&common(CommonTheme::Light)).scheme,
        ColorScheme::Light
    );
    let gone = SystemEnv::default();
    assert_eq!(s.view(&gone).scheme, ColorScheme::Dark);
    assert!(!s.view(&gone).appearance_locked);
}

// ---------------------------------------------------------------- the two account defaults (D22)

#[test]
fn the_direct_default_changes_alone() {
    let mut s = fresh();
    s.set_direct_messages(GlobalNotificationLevel::Mentions);
    assert_eq!(
        s.preferences(),
        &Preferences {
            direct_messages: GlobalNotificationLevel::Mentions,
            ..Preferences::default()
        }
    );
    assert!(s.is_dirty());
}

#[test]
fn the_group_default_changes_alone() {
    let mut s = fresh();
    s.set_groups(GlobalNotificationLevel::Mentions);
    assert_eq!(
        s.preferences(),
        &Preferences {
            groups: GlobalNotificationLevel::Mentions,
            ..Preferences::default()
        }
    );
    assert!(s.is_dirty());
}
