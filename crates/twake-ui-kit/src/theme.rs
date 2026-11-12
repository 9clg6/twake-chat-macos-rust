//! The theme: which colour scheme of `twake-tokens` is active.

use twake_tokens::{DARK, LIGHT, Scheme};

/// Light or dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    /// The light scheme.
    Light,
    /// The dark scheme.
    Dark,
}

/// The active theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// The active scheme.
    pub scheme: ColorScheme,
}

impl Theme {
    /// A theme of this scheme.
    pub const fn new(scheme: ColorScheme) -> Self {
        Self { scheme }
    }

    /// The theme that follows the OS appearance (`true` is dark).
    pub const fn system(dark: bool) -> Self {
        Self::new(if dark {
            ColorScheme::Dark
        } else {
            ColorScheme::Light
        })
    }

    /// The colours of the active scheme.
    pub fn colors(&self) -> &'static Scheme {
        match self.scheme {
            ColorScheme::Light => &LIGHT,
            ColorScheme::Dark => &DARK,
        }
    }

    /// Light becomes dark and dark becomes light.
    pub fn toggle(&mut self) {
        self.scheme = match self.scheme {
            ColorScheme::Light => ColorScheme::Dark,
            ColorScheme::Dark => ColorScheme::Light,
        };
    }
}
