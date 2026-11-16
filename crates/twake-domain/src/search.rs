//! Folding of text for searches: the room list (D392) and the mention lists (A015).

use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

/// A015: NFD, combining marks removed, lower case. "Élodie" becomes "elodie".
pub fn fold(s: &str) -> String {
    s.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(char::to_lowercase)
        .collect()
}
