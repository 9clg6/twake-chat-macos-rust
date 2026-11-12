//! Design tokens (colours, type, radii, shadows, motion, breakpoints); GPUI-free and dependency-free.

/// Colours as 0xRRGGBB for one scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// Window background.
    pub background: u32,
    /// Default text colour.
    pub text: u32,
}

/// Light scheme. Hand-written placeholder: this module is replaced by the output of
/// `cargo xtask tokens` (generated from `docs/spec/tokens.json`) and must not be edited then.
pub const THEME: Theme = Theme {
    background: 0xFFFFFF,
    text: 0x1C1C1E,
};
