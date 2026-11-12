/// Colours as 0xRRGGBB. A single light theme for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub background: u32,
    pub text: u32,
}

pub const THEME: Theme = Theme {
    background: 0xFFFFFF,
    text: 0x1C1C1E,
};
