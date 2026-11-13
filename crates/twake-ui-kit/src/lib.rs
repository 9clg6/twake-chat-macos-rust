//! Design-system components mirroring twake-mui and ds/, with no Matrix and no use cases.
//! The pure half (theme, style resolution, SVG text, colour glue) has no window; `view` draws it with GPUI.

pub mod avatar;
pub mod button;
pub mod chip;
pub mod color;
pub mod dialog;
pub mod icon;
pub mod icon_button;
pub mod menu;
pub mod presence;
pub mod shadow;
pub mod text_buffer;
pub mod theme;
pub mod tooltip;
pub mod view;

/// The token types the pure half of `twake-ui` names (it has no dependency on `twake-tokens`, A010).
pub use twake_tokens::{Rgba8, TYPE_BODY2, TypeSpec};
