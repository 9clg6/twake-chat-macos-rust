//! `Icon::to_svg` and `by_id`: gpui renders SVG documents, so an icon must become a complete
//! one, tinted by the text colour. Public items used: `docs/dev/icons-api-sketch.md`.

use twake_icons::{ALL, Icon, ViewBox, by_id};

fn icon(view_box: ViewBox, body: &'static str) -> Icon {
    Icon {
        id: "sample",
        view_box,
        body,
    }
}

fn square() -> ViewBox {
    ViewBox {
        min_x: 0.0,
        min_y: 0.0,
        width: 16.0,
        height: 16.0,
    }
}

#[test]
fn to_svg_is_a_complete_document_tinted_by_the_current_colour() {
    let svg = icon(square(), "<path d=\"M1 2\"/>").to_svg();
    assert_eq!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 16 16\" fill=\"currentColor\"><path d=\"M1 2\"/></svg>"
    );
}

#[test]
fn to_svg_writes_the_four_viewbox_numbers_in_order() {
    let view_box = ViewBox {
        min_x: 0.5,
        min_y: -10.0,
        width: 24.0,
        height: 12.5,
    };
    let svg = icon(view_box, "<g/>").to_svg();
    assert!(svg.contains("viewBox=\"0.5 -10 24 12.5\""), "{svg}");
}

#[test]
fn to_svg_keeps_the_body_untouched() {
    let body = "<a b=\"1\"/><c d='2'/>&amp;\\";
    let svg = icon(square(), body).to_svg();
    assert!(svg.ends_with(&format!("{body}</svg>")), "{svg}");
}

#[test]
fn the_set_is_not_empty() {
    assert!(!ALL.is_empty());
}

#[test]
fn by_id_finds_every_icon_of_the_set() {
    for icon in ALL {
        let found = by_id(icon.id);
        assert!(found.is_some_and(|f| std::ptr::eq(f, icon)), "{}", icon.id);
    }
}

#[test]
fn by_id_finds_the_first_and_the_last_icon() {
    let (first, last) = (ALL.first(), ALL.last());
    assert_eq!(by_id(first.map_or("", |i| i.id)), first);
    assert_eq!(by_id(last.map_or("", |i| i.id)), last);
}

#[test]
fn by_id_returns_none_for_an_unknown_id() {
    assert!(by_id("no-such-icon").is_none());
    assert!(by_id("").is_none());
}

#[test]
fn by_id_is_exact_and_case_sensitive() {
    let id = ALL.first().map_or("", |i| i.id);
    assert!(by_id(&id.to_uppercase()).is_none());
    assert!(by_id(&format!("{id}x")).is_none());
}
