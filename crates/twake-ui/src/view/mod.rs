//! GPUI drawing of the view models: the room list and the conversation, both virtualised.
//! No test draws these (a window is needed); the models they draw are tested in their own modules.

mod room_list;
mod timeline;

pub use room_list::{RoomListView, RoomSelected};
pub use timeline::TimelineView;

use twake_gpui::{
    Div, FontWeight, ParentElement, ScrollWheelEvent, Styled, Window, div, px, white,
};
use twake_ui_kit::avatar::{LABEL_WEIGHT, label_size_px};
use twake_ui_kit::view::avatar_background;

use crate::avatar::AvatarView;

/// Height of the title bar of a column, in pixels.
pub(crate) const HEADER_HEIGHT: f32 = 56.0;
/// Rows built beyond the window on each side.
pub(crate) const OVERSCAN: usize = 4;
/// Pixels of a wheel "line".
const WHEEL_LINE: f32 = 20.0;

/// A round avatar with initials.
pub(crate) fn avatar(view: &AvatarView, size: f32) -> Div {
    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .size(px(size))
        .rounded_full()
        .bg(avatar_background(view.color_index))
        .text_color(white())
        .text_size(px(label_size_px(size)))
        .font_weight(FontWeight(f32::from(LABEL_WEIGHT)))
        .child(view.initials.clone())
}

/// The height of the list area of a column in this window.
pub(crate) fn viewport(window: &Window) -> f32 {
    f32::from(window.viewport_size().height) - HEADER_HEIGHT
}

/// The scroll position after a wheel event, kept inside `0..=total - viewport`.
pub(crate) fn scrolled(top: f32, event: &ScrollWheelEvent, total: f32, viewport: f32) -> f32 {
    let delta = f32::from(event.delta.pixel_delta(px(WHEEL_LINE)).y);
    clamp(top - delta, total, viewport)
}

/// `top` inside `0..=total - viewport`.
pub(crate) fn clamp(top: f32, total: f32, viewport: f32) -> f32 {
    top.clamp(0.0, (total - viewport).max(0.0))
}
