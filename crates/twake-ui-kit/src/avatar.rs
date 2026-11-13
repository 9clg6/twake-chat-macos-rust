//! Avatar of twake-mui: which gradient a name gets and how its label is drawn
//! (`Avatar/helpers.js` and `Avatar/index.js` of twake-mui 19.3.0).

use twake_tokens::{AVATAR_GRADIENTS, Gradient};

/// Weight of the label.
pub const LABEL_WEIGHT: u16 = 600;

/// The key of `makeKey`: the sum, over the code points of the upper-case name, of the first UTF-16
/// unit of each, modulo the number of gradients. An empty name gives 0 (React gives no colour).
pub fn color_index(name: &str) -> usize {
    let sum: usize = name
        .to_uppercase()
        .chars()
        .map(|c| usize::from(c.encode_utf16(&mut [0; 2])[0]))
        .sum();
    sum % AVATAR_GRADIENTS.len()
}

/// The gradient of an index (it wraps).
pub fn gradient(index: usize) -> &'static Gradient {
    &AVATAR_GRADIENTS[index % AVATAR_GRADIENTS.len()]
}

/// The label is half the avatar.
pub fn label_size_px(size_px: f32) -> f32 {
    size_px / 2.0
}
