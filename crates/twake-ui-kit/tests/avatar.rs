//! Avatar of twake-mui: which gradient a name gets and how its label is drawn.
//! Need: lot 1 a) of `2026-11-13-design-match.md`: key = sum of the UTF-16 code of the first unit of
//! each code point of the UPPER-CASE name, modulo 11 (`makeKey` of `Avatar/helpers.js`, twake-mui 19.3.0);
//! label = size / 2, weight 600 (`Avatar/index.js`). Public items: `twake_ui_kit::avatar`.

use twake_tokens::AVATAR_GRADIENTS;
use twake_ui_kit::avatar::{LABEL_WEIGHT, color_index, gradient, label_size_px};

#[test]
fn the_key_is_the_sum_of_the_codes_of_the_upper_case_name_modulo_11() {
    // A65 L76 I73 C67 E69 = 350 = 31 * 11 + 9
    assert_eq!(color_index("Alice"), 9);
    assert_eq!(color_index("Twake"), 6);
    assert_eq!(color_index("Bob"), 2);
    assert_eq!(color_index("a"), 10);
}

#[test]
fn case_does_not_change_the_colour() {
    assert_eq!(color_index("alice"), color_index("ALICE"));
    assert_eq!(color_index("alice"), color_index("Alice"));
}

#[test]
fn spaces_count_as_characters() {
    // 65 + 32 + 66 = 163 = 14 * 11 + 9
    assert_eq!(color_index("A B"), 9);
}

#[test]
fn accents_use_the_code_of_the_upper_case_letter() {
    // U+00C9 = 201 = 18 * 11 + 3
    assert_eq!(color_index("é"), 3);
    assert_eq!(color_index("Zoë"), 9);
}

#[test]
fn an_upper_case_that_makes_two_letters_counts_both() {
    // "ß".toUpperCase() is "SS": 83 + 83 = 166 = 15 * 11 + 1
    assert_eq!(color_index("ß"), 1);
}

#[test]
fn a_letter_outside_the_bmp_counts_its_first_utf16_unit_only_as_the_web_does() {
    // Array.from(name) gives code points, charCodeAt(0) of U+1F600 is 0xD83D = 55357 = 5032 * 11 + 5
    assert_eq!(color_index("😀"), 5);
    assert_eq!(color_index("A😀"), 4);
}

#[test]
fn the_key_is_always_one_of_the_eleven_gradients() {
    for name in [
        "",
        "x",
        "Design crew",
        "@john.doe:example.org",
        "日本語",
        "🙂🙂🙂",
    ] {
        assert!(color_index(name) < 11, "{name:?}");
    }
}

#[test]
fn the_gradient_of_an_index_is_the_one_of_the_table_and_wraps() {
    assert_eq!(gradient(0), &AVATAR_GRADIENTS[0]);
    assert_eq!(gradient(10), &AVATAR_GRADIENTS[10]);
    assert_eq!(gradient(11), &AVATAR_GRADIENTS[0]);
}

#[test]
fn the_label_is_half_the_avatar_and_semi_bold() {
    assert_eq!(label_size_px(48.0), 24.0);
    assert_eq!(label_size_px(32.0), 16.0);
    assert_eq!(label_size_px(40.0), 20.0);
    assert_eq!(LABEL_WEIGHT, 600);
}
