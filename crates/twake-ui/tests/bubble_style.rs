//! Colours and geometry of a message bubble, as a pure function (no window).
//! Need: lot 1 b) of `2026-11-13-design-match.md`, from `MessageBubble.tsx` of the React app:
//! own message = 12 % of `primary.main` over `background.paper`, text `text.primary`; a message of
//! someone else = `background.paper`; radius 16, padding 16 x 8, text 15 / 20, widest bubble
//! `min(75 %, 520 px)` of the column. Public items: `twake_ui::bubble_style::BubbleStyle::resolve`.
//! Not covered here: the 1 px shadow and the tail (shadows come with the token export, lot 1 d).

use twake_ui::bubble_style::BubbleStyle;
use twake_ui::timeline_rows::Side;
use twake_ui_kit::theme::{ColorScheme, Theme};

fn light() -> Theme {
    Theme::new(ColorScheme::Light)
}

fn dark() -> Theme {
    Theme::new(ColorScheme::Dark)
}

#[test]
fn an_own_bubble_is_the_pale_blue_of_twelve_percent_primary_over_the_paper() {
    let style = BubbleStyle::resolve(Side::Own, &light(), 800.0);
    let c = style.background;
    assert_eq!((c.r, c.g, c.b, c.a), (226, 240, 255, 255));
}

#[test]
fn an_own_bubble_keeps_the_primary_text_colour_not_white_on_blue() {
    assert_eq!(
        BubbleStyle::resolve(Side::Own, &light(), 800.0).text,
        light().colors().text.primary
    );
    assert_eq!(
        BubbleStyle::resolve(Side::Own, &dark(), 800.0).text,
        dark().colors().text.primary
    );
}

#[test]
fn an_own_bubble_in_the_dark_scheme_mixes_over_the_dark_paper() {
    let style = BubbleStyle::resolve(Side::Own, &dark(), 800.0);
    let c = style.background;
    assert_eq!((c.r, c.g, c.b, c.a), (83, 93, 103, 255));
}

#[test]
fn a_bubble_of_someone_else_is_the_paper() {
    assert_eq!(
        BubbleStyle::resolve(Side::Other, &light(), 800.0).background,
        light().colors().background.paper
    );
    assert_eq!(
        BubbleStyle::resolve(Side::Other, &dark(), 800.0).background,
        dark().colors().background.paper
    );
    assert_eq!(
        BubbleStyle::resolve(Side::Other, &light(), 800.0).text,
        light().colors().text.primary
    );
}

#[test]
fn both_sides_share_one_geometry() {
    for side in [Side::Own, Side::Other] {
        let style = BubbleStyle::resolve(side, &light(), 800.0);
        assert_eq!(style.radius_px, 16.0);
        assert_eq!(style.padding_x_px, 16.0);
        assert_eq!(style.padding_y_px, 8.0);
        assert_eq!(style.font_size_px, 15.0);
        assert_eq!(style.line_height_px, 20.0);
    }
}

#[test]
fn the_widest_bubble_is_three_quarters_of_a_narrow_column() {
    assert_eq!(
        BubbleStyle::resolve(Side::Other, &light(), 400.0).max_width_px,
        300.0
    );
    assert_eq!(
        BubbleStyle::resolve(Side::Other, &light(), 600.0).max_width_px,
        450.0
    );
}

#[test]
fn the_widest_bubble_stops_at_520_px_in_a_wide_column() {
    assert_eq!(
        BubbleStyle::resolve(Side::Other, &light(), 1200.0).max_width_px,
        520.0
    );
    // the switch is where 75 % reaches 520
    let edge = 520.0 / 0.75;
    assert!((BubbleStyle::resolve(Side::Own, &light(), edge).max_width_px - 520.0).abs() < 0.01);
    assert!(BubbleStyle::resolve(Side::Own, &light(), edge - 30.0).max_width_px < 520.0);
}

#[test]
fn a_column_of_no_width_gives_a_bubble_of_no_width() {
    assert_eq!(
        BubbleStyle::resolve(Side::Own, &light(), 0.0).max_width_px,
        0.0
    );
}
