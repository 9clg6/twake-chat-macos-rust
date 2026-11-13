//! Screens and entities; calls use cases only.
//! The pure half (view models, virtual list window) has no window; `view` draws it with GPUI.

pub mod avatar;
pub mod bubble_style;
mod clock;
pub mod room_row;
pub mod room_row_style;
pub mod timeline_rows;
pub mod view;
pub mod virtual_list;
