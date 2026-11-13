//! The presence dot over an avatar (`ds/PresenceBadge` of twake-space-chat), without a window.
//! Source: `src/ds/PresenceBadge/PresenceBadge.tsx:18-52`.

use twake_tokens::Rgba8;

use crate::theme::Theme;

/// What the server says about a person.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceState {
    /// Active.
    Online,
    /// Idle.
    Away,
    /// Gone: no dot.
    Offline,
}

/// The dot drawn in the lower right corner of an avatar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PresenceDot {
    /// Diameter of the coloured disc.
    pub diameter_px: f32,
    /// Width of the ring around it.
    pub ring_px: f32,
    /// Ring colour (the surface under the avatar).
    pub ring: Rgba8,
    /// Disc colour.
    pub color: Rgba8,
    /// Distance from the right and bottom edges of the avatar.
    pub inset_px: f32,
    /// Duration of the colour change; none under reduced motion (the view's job).
    pub transition_ms: u32,
}

/// Avatars up to this size take the small dot.
const SMALL_AVATAR_PX: f32 = 40.0;

impl PresenceDot {
    /// The dot for this presence on an avatar of this size; `None` for no dot (unknown or offline).
    pub fn resolve(state: Option<PresenceState>, avatar_px: f32, theme: &Theme) -> Option<Self> {
        let scheme = theme.colors();
        let color = match state? {
            PresenceState::Online => scheme.success.main,
            PresenceState::Away => scheme.warning.main,
            PresenceState::Offline => return None,
        };
        Some(Self {
            diameter_px: if avatar_px > SMALL_AVATAR_PX {
                12.0
            } else {
                10.0
            },
            ring_px: 2.0,
            ring: scheme.background.paper,
            color,
            inset_px: 0.0,
            transition_ms: 150,
        })
    }
}
