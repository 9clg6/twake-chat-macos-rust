//! The embedded Inter 4.1 (SIL OFL 1.1, A032): the same text on macOS, Windows and Linux.

/// The family name, the first of `twake_tokens::FONT_FAMILY`.
pub const FAMILY: &str = "Inter";
/// The licence text that ships with the fonts.
pub const LICENSE: &str = include_str!("../assets/fonts/LICENSE.txt");

/// One static face.
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedFont {
    /// File name in `assets/fonts`.
    pub file_name: &'static str,
    /// CSS weight.
    pub weight: u16,
    /// The TrueType file.
    pub data: &'static [u8],
}

const FONTS: [EmbeddedFont; 4] = [
    EmbeddedFont {
        file_name: "Inter-Regular.ttf",
        weight: 400,
        data: include_bytes!("../assets/fonts/Inter-Regular.ttf"),
    },
    EmbeddedFont {
        file_name: "Inter-Medium.ttf",
        weight: 500,
        data: include_bytes!("../assets/fonts/Inter-Medium.ttf"),
    },
    EmbeddedFont {
        file_name: "Inter-SemiBold.ttf",
        weight: 600,
        data: include_bytes!("../assets/fonts/Inter-SemiBold.ttf"),
    },
    EmbeddedFont {
        file_name: "Inter-Bold.ttf",
        weight: 700,
        data: include_bytes!("../assets/fonts/Inter-Bold.ttf"),
    },
];

/// The four faces, by weight 400, 500, 600, 700.
pub fn embedded() -> &'static [EmbeddedFont] {
    &FONTS
}

/// Registers the faces with the text system; call before the first window.
///
/// # Errors
/// The text system's error when a face does not load.
pub fn register(text: &twake_gpui::TextSystem) -> Result<(), impl std::fmt::Debug + use<>> {
    let blobs: Vec<&'static [u8]> = FONTS.iter().map(|f| f.data).collect();
    twake_gpui::register_fonts(text, &blobs)
}
