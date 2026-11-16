//! The only crate naming the GPUI source; re-exports it and holds the runtime glue.

pub use gpui::*;
pub use gpui_platform;

/// Adds font files to the text system (the facade stays free of other workspace crates).
///
/// # Errors
/// The platform text system's error when a blob is not a font it can load.
pub fn register_fonts(
    text: &TextSystem,
    fonts: &[&'static [u8]],
) -> Result<(), impl std::fmt::Debug + use<>> {
    text.add_fonts(
        fonts
            .iter()
            .map(|f| std::borrow::Cow::Borrowed(*f))
            .collect(),
    )
}
