//! GPUI drawing of the small components: an icon button, a chip, a tooltip, a menu, a dialog and the
//! presence dot. Each draws the style resolved for ONE state (the gallery shows every state at once);
//! no test draws them, the styles are tested in their own modules.

use twake_gpui::{
    AnyElement, BoxShadow, Div, FontWeight, IntoElement, ParentElement, Styled, div, point, px,
};
use twake_icons::{CROSS_SMALL, Icon};
use twake_tokens::{LineHeight, Rgba8, TypeSpec};

use super::{hsla, icon};
use crate::button::FocusRing;
use crate::chip::{ChipProps, ChipState, ChipStyle};
use crate::dialog::{DialogSize, DialogStyle};
use crate::icon::{IconColor, IconSpec};
use crate::icon_button::{IconButtonColor, IconButtonSize, IconButtonState, IconButtonStyle};
use crate::menu::{MenuItemState, MenuItemStyle, MenuStyle, SeparatorStyle};
use crate::presence::PresenceDot;
use crate::theme::Theme;
use crate::tooltip::TooltipStyle;

/// The ring of keyboard focus, drawn outside the box.
fn ring_shadow(ring: FocusRing) -> BoxShadow {
    BoxShadow {
        color: hsla(ring.color),
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: px(ring.width_px),
    }
}

/// The ring as a border: a spread shadow would show through a translucent fill and tint the whole box.
pub(super) fn ringed<E: Styled>(base: E, ring: Option<FocusRing>) -> E {
    match ring {
        Some(ring) => base
            .border(px(ring.width_px))
            .border_color(hsla(ring.color)),
        None => base,
    }
}

fn glyph(icon_ref: &'static Icon, size_px: f32, color: Rgba8) -> impl IntoElement {
    icon(&IconSpec {
        icon: icon_ref,
        size_px,
        color: IconColor::Fixed(color),
    })
}

fn typed(base: Div, font: TypeSpec) -> Div {
    let line = match font.line_height {
        LineHeight::Ratio(ratio) => font.font_size * ratio,
        LineHeight::Px(px) => px,
    };
    base.text_size(px(font.font_size))
        .font_weight(FontWeight(f32::from(font.font_weight)))
        .line_height(px(line))
}

/// A round icon button in one state.
pub fn icon_button(
    glyph_ref: &'static Icon,
    icon_px: f32,
    (size, color, state): (IconButtonSize, IconButtonColor, IconButtonState),
    theme: &Theme,
) -> Div {
    let s = IconButtonStyle::resolve(size, color, state, theme);
    let side = px(s.diameter_px(icon_px));
    let base = div()
        .flex()
        .items_center()
        .justify_center()
        .size(side)
        .rounded(side * s.radius_fraction)
        .bg(hsla(s.background))
        .child(glyph(glyph_ref, icon_px, s.foreground));
    ringed(base, s.focus_ring)
}

/// A chip in one state.
pub fn chip(label: &str, props: ChipProps, state: ChipState, theme: &Theme) -> Div {
    let s = ChipStyle::resolve(props, state, theme);
    let mut base = div()
        .flex()
        .items_center()
        .h(px(s.height_px))
        .rounded(px(s.radius_px))
        .bg(hsla(s.background))
        .text_color(hsla(s.foreground))
        .text_size(px(s.font_size_px))
        .opacity(s.opacity)
        .child(div().px(px(s.label_padding_x_px)).child(label.to_owned()));
    if let Some(border) = s.border {
        base = base
            .border(px(border.width_px))
            .border_color(hsla(border.color));
    }
    if let Some(cross) = s.delete_icon {
        base = base.child(
            div()
                .ml(px(cross.margin_left_px))
                .mr(px(cross.margin_right_px))
                .child(glyph(&CROSS_SMALL, cross.size_px, cross.color)),
        );
    }
    ringed(base, s.focus_ring)
}

/// The box of a tooltip (the arrow is not drawn yet).
pub fn tooltip(text: &str, theme: &Theme) -> Div {
    let s = TooltipStyle::resolve(theme);
    div()
        .px(px(s.padding_x_px))
        .py(px(s.padding_y_px))
        .max_w(px(s.max_width_px))
        .rounded(px(s.radius_px))
        .bg(hsla(s.background))
        .text_color(hsla(s.text))
        .text_size(px(s.font_size_px))
        .font_weight(FontWeight(f32::from(s.font_weight)))
        .line_height(px(s.font_size_px * s.line_height_ratio))
        .child(text.to_owned())
}

/// The panel of a menu; add `menu_item`s and `menu_separator`s as children.
pub fn menu_panel(theme: &Theme) -> Div {
    let s = MenuStyle::resolve(theme);
    // The CSS shadow of the token is not parsed: GPUI's medium shadow stands in for it (A029).
    div()
        .flex()
        .flex_col()
        .py(px(s.padding_y_px))
        .max_w(px(s.max_width_px))
        .rounded(px(s.radius_px))
        .bg(hsla(s.background))
        .shadow_md()
}

/// One row of a menu in one state; the icon is optional.
pub fn menu_item(
    label: &str,
    icon_ref: Option<&'static Icon>,
    state: MenuItemState,
    theme: &Theme,
) -> Div {
    let s = MenuItemStyle::resolve(state, theme);
    let mut row = typed(div(), s.font)
        .flex()
        .items_center()
        .gap(px(s.gap_px))
        .min_h(px(s.min_height_px))
        .px(px(s.padding_x_px))
        .py(px(s.padding_y_px))
        .bg(hsla(s.background))
        .text_color(hsla(s.foreground))
        .opacity(s.opacity);
    // GPUI has no inset shadow: the ring is a border of the row.
    row = ringed(row, s.focus_ring);
    if let Some(icon_ref) = icon_ref {
        row = row.child(glyph(icon_ref, 20.0, s.icon_color));
    }
    row.child(label.to_owned())
}

/// The line between two groups of items.
pub fn menu_separator(theme: &Theme) -> Div {
    let s = SeparatorStyle::resolve(theme);
    div()
        .h(px(s.height_px))
        .my(px(s.margin_y_px))
        .bg(hsla(s.color))
}

/// A dialog in a window of this width: the backdrop colour is the caller's (`DialogStyle::backdrop`).
pub fn dialog(
    title: &str,
    body: &str,
    actions: Vec<AnyElement>,
    (size, window_px): (DialogSize, f32),
    theme: &Theme,
) -> Div {
    let s = DialogStyle::resolve(size, window_px, theme);
    let c = s.content_padding;
    let mut row = div().flex().justify_end().gap(px(s.actions_gap_px));
    for action in actions {
        row = row.child(action);
    }
    div()
        .flex()
        .flex_col()
        .w(px(s.max_width_px.unwrap_or(window_px - 2.0 * s.margin_px)))
        .rounded(px(s.radius_px))
        .bg(hsla(s.background))
        .shadow_xl()
        .child(
            typed(div(), s.title.font)
                .px(px(s.title.padding_x_px))
                .py(px(s.title.padding_y_px))
                .child(title.to_owned()),
        )
        .child(
            div()
                .pl(px(c.left))
                .pt(px(c.top))
                .pr(px(c.right))
                .pb(px(c.bottom))
                .child(body.to_owned()),
        )
        .child(
            row.mx(px(s.actions_margin_x_px))
                .my(px(s.actions_margin_y_px)),
        )
}

/// The dot over an avatar: absolute, in the lower right corner of its parent.
pub fn presence_dot(dot: &PresenceDot) -> Div {
    div()
        .absolute()
        .right(px(dot.inset_px))
        .bottom(px(dot.inset_px))
        .size(px(dot.diameter_px))
        .rounded_full()
        .bg(hsla(dot.color))
        .shadow(vec![ring_shadow(FocusRing {
            width_px: dot.ring_px,
            color: dot.ring,
        })])
}
