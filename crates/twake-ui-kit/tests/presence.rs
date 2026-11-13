//! The presence dot over an avatar (`ds/PresenceBadge`), without a window.
//! R = ~/Developer/twake-space-chat. Source: `R/src/ds/PresenceBadge/PresenceBadge.tsx:18-52`.
//! Expected API (new `twake_ui_kit::presence`): `PresenceState {Online, Away, Offline}`,
//! `PresenceDot { diameter_px, ring_px, ring: Rgba8, color: Rgba8, inset_px, transition_ms }`,
//! `PresenceDot::resolve(state: Option<PresenceState>, avatar_px: f32, &Theme) -> Option<PresenceDot>`
//! (`None` = no dot: unknown, hidden or offline presence).

use twake_ui_kit::presence::{PresenceDot, PresenceState};
use twake_ui_kit::theme::{ColorScheme, Theme};

fn themes() -> [Theme; 2] {
    [Theme::new(ColorScheme::Light), Theme::new(ColorScheme::Dark)]
}

// PresenceBadge.tsx:25: `size > 40 ? 12 : 10`.
#[test]
fn the_dot_is_10px_up_to_a_40px_avatar_and_12px_beyond() {
    let t = &themes()[0];
    for (avatar, dot) in [(16.0, 10.0), (32.0, 10.0), (40.0, 10.0), (40.5, 12.0), (48.0, 12.0), (64.0, 12.0)] {
        let d = PresenceDot::resolve(Some(PresenceState::Online), avatar, t).expect("dot");
        assert_eq!(d.diameter_px, dot, "avatar {avatar}");
    }
}

// PresenceBadge.tsx:26-29: online = success.main, away = warning.main.
#[test]
fn online_is_success_and_away_is_warning() {
    for t in themes() {
        let on = PresenceDot::resolve(Some(PresenceState::Online), 48.0, &t).expect("dot");
        let away = PresenceDot::resolve(Some(PresenceState::Away), 48.0, &t).expect("dot");
        assert_eq!(on.color, t.colors().success.main);
        assert_eq!(away.color, t.colors().warning.main);
    }
}

// PresenceBadge.tsx:12,35-38: "no dot offline"; null while unknown.
#[test]
fn offline_and_unknown_presence_show_no_dot() {
    for t in themes() {
        assert_eq!(PresenceDot::resolve(Some(PresenceState::Offline), 48.0, &t), None);
        assert_eq!(PresenceDot::resolve(None, 48.0, &t), None);
    }
}

// PresenceBadge.tsx:44-46: box-shadow `0 0 0 2px background.paper`, right 0 bottom 0.
#[test]
fn the_dot_has_a_2px_ring_in_the_paper_colour_in_the_corner() {
    for t in themes() {
        let d = PresenceDot::resolve(Some(PresenceState::Online), 48.0, &t).expect("dot");
        assert_eq!(d.ring_px, 2.0);
        assert_eq!(d.ring, t.colors().background.paper);
        assert_eq!(d.inset_px, 0.0);
    }
}

// PresenceBadge.tsx:48: `background-color 150ms ease-out` (reduced motion: none, handled by the view).
#[test]
fn the_colour_change_lasts_150ms() {
    let d = PresenceDot::resolve(Some(PresenceState::Away), 48.0, &themes()[0]).expect("dot");
    assert_eq!(d.transition_ms, 150);
}
