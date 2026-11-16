//! Geometry and colours of a room row and of its unread badge, as pure functions (no window).
//! Need: lot 1 c) of `2026-11-13-design-match.md`, from `RoomRow.tsx` of the React app:
//! row 56 px, avatar 48, gap 8, name 14 / 500; rest transparent, hover `action.hover`, selected
//! `action.selected`, selected + hover `action.focus`; badge = pill 16 px high, 4 px at the sides,
//! radius 8, text 10 / 500, blue (`primary.main` / `primary.contrastText`) for a count AND for a
//! mention, grey (`action.disabledBackground` / `text.primary`) when muted; a dot is 8 px and round;
//! the list is 360 px wide. Public items: `twake_ui::room_row_style`.

use twake_ui::room_row::BadgeStyle;
use twake_ui::room_row_style::{BadgeSpec, LIST_WIDTH_PX, RowState, RowStyle};
use twake_ui_kit::Rgba8;
use twake_ui_kit::theme::{ColorScheme, Theme};

fn light() -> Theme {
    Theme::new(ColorScheme::Light)
}

fn dark() -> Theme {
    Theme::new(ColorScheme::Dark)
}

const REST: RowState = RowState {
    selected: false,
    hovered: false,
};
const HOVER: RowState = RowState {
    selected: false,
    hovered: true,
};
const SELECTED: RowState = RowState {
    selected: true,
    hovered: false,
};
const BOTH: RowState = RowState {
    selected: true,
    hovered: true,
};

#[test]
fn the_list_is_360_px_wide() {
    assert_eq!(LIST_WIDTH_PX, 360.0);
}

#[test]
fn a_row_is_56_high_with_a_48_avatar_and_8_between() {
    let row = RowStyle::resolve(REST, &light());
    assert_eq!(row.height_px, 56.0);
    assert_eq!(row.avatar_px, 48.0);
    assert_eq!(row.gap_px, 8.0);
}

#[test]
fn the_name_is_14_medium() {
    let row = RowStyle::resolve(REST, &light());
    assert_eq!(row.name_font.font_size, 14.0);
    assert_eq!(row.name_font.font_weight, 500);
}

#[test]
fn a_resting_row_has_no_fill() {
    assert_eq!(
        RowStyle::resolve(REST, &light()).background,
        Rgba8::new(0, 0, 0, 0)
    );
    assert_eq!(
        RowStyle::resolve(REST, &dark()).background,
        Rgba8::new(0, 0, 0, 0)
    );
}

#[test]
fn hover_selected_and_both_use_the_action_colours_of_each_scheme() {
    for theme in [light(), dark()] {
        let scheme = theme.colors();
        assert_eq!(
            RowStyle::resolve(HOVER, &theme).background,
            scheme.action.hover
        );
        assert_eq!(
            RowStyle::resolve(SELECTED, &theme).background,
            scheme.action.selected
        );
        assert_eq!(
            RowStyle::resolve(BOTH, &theme).background,
            scheme.action.focus
        );
    }
}

#[test]
fn the_four_states_are_four_different_fills() {
    let theme = light();
    let fills =
        [REST, HOVER, SELECTED, BOTH].map(|state| RowStyle::resolve(state, &theme).background);
    for (i, a) in fills.iter().enumerate() {
        for b in &fills[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn a_count_is_a_blue_pill_16_high_with_10_medium_text() {
    let badge = BadgeSpec::resolve(BadgeStyle::Normal, false, &light());
    assert_eq!(badge.height_px, 16.0);
    assert_eq!(badge.min_width_px, 16.0);
    assert_eq!(badge.padding_x_px, 4.0);
    assert_eq!(badge.radius_px, 8.0);
    assert_eq!(badge.font_size_px, 10.0);
    assert_eq!(badge.font_weight, 500);
    assert_eq!(badge.background, light().colors().primary.main);
    assert_eq!(badge.text, light().colors().primary.contrast_text);
}

#[test]
fn a_mention_is_blue_like_a_count_not_red() {
    for theme in [light(), dark()] {
        let mention = BadgeSpec::resolve(BadgeStyle::Mention, false, &theme);
        let count = BadgeSpec::resolve(BadgeStyle::Normal, false, &theme);
        assert_eq!(mention.background, count.background);
        assert_eq!(mention.background, theme.colors().primary.main);
        assert_ne!(mention.background, theme.colors().error.main);
    }
}

#[test]
fn a_muted_room_is_grey_with_the_primary_text() {
    for theme in [light(), dark()] {
        let badge = BadgeSpec::resolve(BadgeStyle::Muted, false, &theme);
        assert_eq!(badge.background, theme.colors().action.disabled_background);
        assert_eq!(badge.text, theme.colors().text.primary);
        assert_eq!(badge.height_px, 16.0);
    }
}

#[test]
fn a_dot_is_8_px_round_and_takes_the_colour_of_its_style() {
    let dot = BadgeSpec::resolve(BadgeStyle::Normal, true, &light());
    assert_eq!((dot.height_px, dot.min_width_px), (8.0, 8.0));
    assert_eq!(dot.radius_px, 4.0);
    assert_eq!(dot.padding_x_px, 0.0);
    assert_eq!(dot.background, light().colors().primary.main);
    let muted = BadgeSpec::resolve(BadgeStyle::Muted, true, &light());
    assert_eq!(
        muted.background,
        light().colors().action.disabled_background
    );
}
