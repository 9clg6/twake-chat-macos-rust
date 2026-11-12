//! The theme of the common settings wins over the local choice.
//!
//! Need (not the code): DECISION.md D38 of the React app (`twake-workplace-common-settings`):
//! ToM gives `theme` (light, dark or auto) in `user_info` to the user themself, read only. It
//! wins over the choice made on the device; the choices of Appearance are then disabled, with a
//! link to the page of the common settings when there is one (http(s) only, `{username}`
//! replaced: `commonSettingsUrlFor` of `twakeServices.ts`); `auto` is the theme of the system;
//! the local choice is set aside and comes back once the common theme goes; ToM unreachable is
//! not "no theme": the one in place stays. Without a theme there, nothing changes.
//!
//! Public items used are listed in `docs/dev/settings-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use twake_domain::preferences::{
    ColorScheme, CommonFetch, CommonSettings, CommonTheme, Language, Preferences, SystemEnv,
    ThemeChoice, common_settings_url,
};

fn common(theme: Option<CommonTheme>, url: Option<&str>) -> CommonSettings {
    CommonSettings {
        theme,
        language: None,
        url: url.map(String::from),
    }
}

fn spoken(language: Option<&str>, url: Option<&str>) -> CommonSettings {
    CommonSettings {
        language: language.map(String::from),
        ..common(None, url)
    }
}

fn local_language(code: &'static str) -> Preferences {
    Preferences {
        language: Language::Code(code),
        ..Preferences::default()
    }
}

// ---------------------------------------------------------------- the common language (D67)

#[test]
fn a_common_language_wins_over_a_local_one_and_locks_the_picker() {
    let e = env(false, Some(spoken(Some("fr"), None)));
    let v = local_language("de").view(&e);
    assert_eq!(v.language, "fr");
    assert!(v.language_locked);
}

#[test]
fn a_region_or_case_does_not_matter() {
    for (given, want) in [("fr-FR", "fr"), ("en-US", "en"), ("DE", "de"), ("vi_VN", "vi")] {
        let e = env(false, Some(spoken(Some(given), None)));
        assert_eq!(local_language("it").view(&e).language, want, "{given}");
    }
}

#[test]
fn a_language_the_app_lacks_counts_as_none() {
    for given in ["ja-JP", "pt-BR", "xx", "", "f"] {
        let e = env(false, Some(spoken(Some(given), Some(PAGE))));
        let v = local_language("de").view(&e);
        assert_eq!(v.language, "de", "{given:?}");
        assert!(!v.language_locked, "{given:?}");
        assert_eq!(v.common_settings_url, None, "{given:?}");
    }
}

#[test]
fn the_first_two_letters_decide_as_in_the_react_app() {
    let e = env(false, Some(spoken(Some("fra"), None)));
    assert_eq!(local_language("de").view(&e).language, "fr");
}

#[test]
fn a_common_settings_without_a_language_leave_the_local_one() {
    let e = env(false, Some(common(Some(CommonTheme::Dark), None)));
    let v = local_language("de").view(&e);
    assert_eq!(v.language, "de");
    assert!(!v.language_locked);
}

#[test]
fn the_common_language_beats_the_system_one_too() {
    let e = SystemEnv {
        language: "de-DE".into(),
        common: Some(spoken(Some("es"), None)),
        ..SystemEnv::default()
    };
    assert_eq!(Preferences::default().view(&e).language, "es");
}

#[test]
fn the_language_and_the_theme_lock_on_their_own() {
    let only_language = env(false, Some(spoken(Some("fr"), None)));
    let v = Preferences::default().view(&only_language);
    assert!(v.language_locked && !v.appearance_locked);
    let only_theme = env(false, Some(common(Some(CommonTheme::Dark), None)));
    let v = Preferences::default().view(&only_theme);
    assert!(v.appearance_locked && !v.language_locked);
}

#[test]
fn the_link_shows_while_the_language_alone_is_locked() {
    let e = env(false, Some(spoken(Some("fr"), Some(PAGE))));
    assert_eq!(
        Preferences::default().view(&e).common_settings_url,
        Some(PAGE.to_string())
    );
}

#[test]
fn the_local_language_is_not_overwritten_and_comes_back() {
    let prefs = local_language("de");
    let held = env(false, Some(spoken(Some("fr"), None)));
    assert_eq!(prefs.view(&held).language, "fr");
    assert_eq!(prefs.language, Language::Code("de"));
    assert_eq!(prefs.view(&env(false, None)).language, "de");
}

#[test]
fn tom_unreachable_keeps_the_common_language_in_place() {
    let before = env(false, Some(spoken(Some("fr"), None)));
    let after = before.clone().with_common_fetch(CommonFetch::Unreachable);
    assert_eq!(after, before);
    assert_eq!(local_language("de").view(&after).language, "fr");
}

#[test]
fn an_answer_without_a_language_unlocks_it() {
    let before = env(false, Some(spoken(Some("fr"), None)));
    let after = before.with_common_fetch(CommonFetch::Reached(None));
    assert_eq!(local_language("de").view(&after).language, "de");
}

fn env(dark: bool, common: Option<CommonSettings>) -> SystemEnv {
    SystemEnv {
        dark,
        common,
        ..SystemEnv::default()
    }
}

fn local(theme: ThemeChoice) -> Preferences {
    Preferences {
        theme,
        ..Preferences::default()
    }
}

const PAGE: &str = "https://settings.example.org/u/me";

// ---------------------------------------------------------------- the common theme wins

#[test]
fn a_common_light_theme_wins_over_a_local_dark_one() {
    let e = env(true, Some(common(Some(CommonTheme::Light), None)));
    assert_eq!(local(ThemeChoice::Dark).view(&e).scheme, ColorScheme::Light);
}

#[test]
fn a_common_dark_theme_wins_over_a_local_light_one() {
    let e = env(false, Some(common(Some(CommonTheme::Dark), None)));
    assert_eq!(local(ThemeChoice::Light).view(&e).scheme, ColorScheme::Dark);
}

#[test]
fn a_common_auto_theme_is_the_theme_of_the_system() {
    let auto = Some(common(Some(CommonTheme::Auto), None));
    let dark = env(true, auto.clone());
    let light = env(false, auto);
    assert_eq!(
        local(ThemeChoice::Light).view(&dark).scheme,
        ColorScheme::Dark
    );
    assert_eq!(
        local(ThemeChoice::Dark).view(&light).scheme,
        ColorScheme::Light
    );
}

#[test]
fn a_common_theme_locks_the_choices_of_appearance() {
    for theme in [CommonTheme::Light, CommonTheme::Dark, CommonTheme::Auto] {
        let e = env(false, Some(common(Some(theme), None)));
        assert!(Preferences::default().view(&e).appearance_locked);
    }
}

// ---------------------------------------------------------------- nothing there, nothing changes

#[test]
fn without_common_settings_the_local_choice_stays_and_nothing_is_locked() {
    let v = local(ThemeChoice::Dark).view(&env(false, None));
    assert_eq!(v.scheme, ColorScheme::Dark);
    assert!(!v.appearance_locked);
    assert_eq!(v.common_settings_url, None);
}

#[test]
fn common_settings_without_a_theme_leave_the_local_choice_and_unlock() {
    let e = env(false, Some(common(None, Some(PAGE))));
    let v = local(ThemeChoice::Dark).view(&e);
    assert_eq!(v.scheme, ColorScheme::Dark);
    assert!(!v.appearance_locked);
    assert_eq!(v.common_settings_url, None);
}

#[test]
fn the_local_choice_is_set_aside_not_overwritten_and_comes_back() {
    let prefs = local(ThemeChoice::Dark);
    let held = env(false, Some(common(Some(CommonTheme::Light), None)));
    assert_eq!(prefs.view(&held).scheme, ColorScheme::Light);
    assert_eq!(prefs.theme, ThemeChoice::Dark);
    assert_eq!(prefs.view(&env(false, None)).scheme, ColorScheme::Dark);
}

// ---------------------------------------------------------------- the link

#[test]
fn the_link_shows_while_locked_when_there_is_a_url() {
    let e = env(false, Some(common(Some(CommonTheme::Dark), Some(PAGE))));
    assert_eq!(
        Preferences::default().view(&e).common_settings_url,
        Some(PAGE.to_string())
    );
}

#[test]
fn a_locked_view_without_a_url_has_no_link() {
    let e = env(false, Some(common(Some(CommonTheme::Dark), None)));
    assert_eq!(Preferences::default().view(&e).common_settings_url, None);
}

// ---------------------------------------------------------------- ToM unreachable (D38)

#[test]
fn an_answer_replaces_the_common_settings() {
    let before = env(true, Some(common(Some(CommonTheme::Dark), Some(PAGE))));
    let after = before
        .clone()
        .with_common_fetch(CommonFetch::Reached(Some(common(
            Some(CommonTheme::Light),
            None,
        ))));
    assert_eq!(after.common, Some(common(Some(CommonTheme::Light), None)));
    assert!(after.dark, "the rest of the environment is untouched");
}

#[test]
fn an_answer_with_nothing_removes_the_common_settings() {
    let before = env(false, Some(common(Some(CommonTheme::Dark), Some(PAGE))));
    let after = before.with_common_fetch(CommonFetch::Reached(None));
    assert_eq!(after.common, None);
    assert_eq!(
        local(ThemeChoice::Light).view(&after).scheme,
        ColorScheme::Light
    );
}

#[test]
fn tom_unreachable_is_not_no_theme_the_one_in_place_stays() {
    let before = env(false, Some(common(Some(CommonTheme::Dark), Some(PAGE))));
    let after = before.clone().with_common_fetch(CommonFetch::Unreachable);
    assert_eq!(after, before);
    assert_eq!(
        local(ThemeChoice::Light).view(&after).scheme,
        ColorScheme::Dark
    );
}

#[test]
fn tom_unreachable_with_no_theme_in_place_stays_without() {
    let before = env(false, None);
    assert_eq!(
        before.clone().with_common_fetch(CommonFetch::Unreachable),
        before
    );
}

// ---------------------------------------------------------------- common_settings_url (D38)

#[test]
fn the_username_is_the_local_part_of_the_matrix_id() {
    assert_eq!(
        common_settings_url(
            "https://settings.example.org/u/{username}/theme",
            "@jean.dupont:example.org"
        ),
        Some("https://settings.example.org/u/jean.dupont/theme".to_string())
    );
}

#[test]
fn the_username_stops_at_the_first_colon() {
    assert_eq!(
        common_settings_url("https://h.example/{username}/x", "@jean:example.org:8448"),
        Some("https://h.example/jean/x".to_string())
    );
}

#[test]
fn every_placeholder_is_replaced() {
    assert_eq!(
        common_settings_url("https://h.example/{username}/x?u={username}", "@ana:s.org"),
        Some("https://h.example/ana/x?u=ana".to_string())
    );
}

#[test]
fn the_username_is_percent_encoded() {
    assert_eq!(
        common_settings_url("https://h.example/{username}/x", "@a b:s.org"),
        Some("https://h.example/a%20b/x".to_string())
    );
    assert_eq!(
        common_settings_url("https://h.example/{username}/x", "@a/b:s.org"),
        Some("https://h.example/a%2Fb/x".to_string())
    );
    assert_eq!(
        common_settings_url("https://h.example/{username}/x", "@\u{e9}:s.org"),
        Some("https://h.example/%C3%A9/x".to_string())
    );
}

#[test]
fn http_is_allowed_as_well_as_https() {
    assert_eq!(
        common_settings_url("http://h.example/{username}/x", "@ana:s.org"),
        Some("http://h.example/ana/x".to_string())
    );
}

#[test]
fn any_other_scheme_gives_no_link() {
    for template in [
        "javascript:alert('{username}')",
        "ftp://h.example/{username}",
        "data:text/html,{username}",
        "file:///tmp/{username}",
    ] {
        assert_eq!(
            common_settings_url(template, "@ana:s.org"),
            None,
            "{template}"
        );
    }
}

#[test]
fn a_url_without_the_placeholder_gives_no_link() {
    assert_eq!(
        common_settings_url("https://h.example/settings", "@ana:s.org"),
        None
    );
}

#[test]
fn a_matrix_id_without_a_username_gives_no_link() {
    for user_id in ["ana:s.org", "@ana", "@:s.org", ""] {
        assert_eq!(
            common_settings_url("https://h.example/{username}/x", user_id),
            None,
            "{user_id:?}"
        );
    }
}

#[test]
fn a_text_that_is_no_url_gives_no_link() {
    assert_eq!(
        common_settings_url("not a url {username}", "@ana:s.org"),
        None
    );
    assert_eq!(common_settings_url("", "@ana:s.org"), None);
}

#[test]
fn the_characters_encodeuricomponent_keeps_stay_as_they_are() {
    assert_eq!(
        common_settings_url("https://h.example/{username}/x", "@a(b)*'!~-_.c:s.org"),
        Some("https://h.example/a(b)*'!~-_.c/x".to_string())
    );
}

#[test]
fn the_link_is_the_normalized_url() {
    assert_eq!(
        common_settings_url("HTTPS://Settings.Example.org/{username}", "@ana:s.org"),
        Some("https://settings.example.org/ana".to_string())
    );
    assert_eq!(
        common_settings_url("https://h.example?u={username}", "@ana:s.org"),
        Some("https://h.example/?u=ana".to_string())
    );
}
