//! Screens and entities; calls use cases only.
//! The pure half (view models, virtual list window) has no window; `view` draws it with GPUI.

pub mod avatar;
mod clock;
pub mod room_row;
pub mod timeline_rows;
pub mod virtual_list;
