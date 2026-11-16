//! `Rgba8`: colours as plain bytes, parsed from CSS hex notation.
//! Need: `docs/spec/design-system.md` section 4 (colour), tokens come as `#rgb`, `#rrggbb`,
//! `#rrggbbaa`. Public items used are listed in `docs/dev/tokens-api-sketch.md`.

use twake_tokens::{ParseColorError, Rgba8};

#[test]
fn six_digit_hex_is_opaque() {
    assert_eq!(
        Rgba8::from_hex("#0a84ff"),
        Ok(Rgba8::new(10, 132, 255, 255))
    );
}

#[test]
fn three_digit_hex_doubles_each_digit() {
    assert_eq!(Rgba8::from_hex("#fff"), Ok(Rgba8::new(255, 255, 255, 255)));
    assert_eq!(Rgba8::from_hex("#0af"), Ok(Rgba8::new(0, 170, 255, 255)));
}

#[test]
fn eight_digit_hex_carries_alpha() {
    assert_eq!(
        Rgba8::from_hex("#0a84ff80"),
        Ok(Rgba8::new(10, 132, 255, 128))
    );
    assert_eq!(Rgba8::from_hex("#00000000"), Ok(Rgba8::new(0, 0, 0, 0)));
}

#[test]
fn hex_digits_are_case_insensitive() {
    assert_eq!(Rgba8::from_hex("#FCFCFD"), Rgba8::from_hex("#fcfcfd"));
    assert_eq!(Rgba8::from_hex("#2C3039"), Ok(Rgba8::new(44, 48, 57, 255)));
}

#[test]
fn rgb_builds_an_opaque_colour_from_a_packed_integer() {
    assert_eq!(Rgba8::rgb(0x0a84ff), Rgba8::new(10, 132, 255, 255));
}

#[test]
fn parsing_works_in_a_const_context() {
    const BLUE: Result<Rgba8, ParseColorError> = Rgba8::from_hex("#0a84ff");
    assert_eq!(BLUE, Ok(Rgba8::rgb(0x0a84ff)));
}

#[test]
fn text_without_the_hash_is_refused() {
    assert!(Rgba8::from_hex("0a84ff").is_err());
}

#[test]
fn lengths_other_than_3_6_or_8_digits_are_refused() {
    for text in [
        "",
        "#",
        "#f",
        "#ff",
        "#ffff",
        "#fffff",
        "#fffffff",
        "#fffffffff",
    ] {
        assert!(Rgba8::from_hex(text).is_err(), "{text:?} must be refused");
    }
}

#[test]
fn non_hex_digits_are_refused() {
    for text in ["#12g456", "#xyz", "#0a84fg", "#-12345", "#+12345"] {
        assert!(Rgba8::from_hex(text).is_err(), "{text:?} must be refused");
    }
}

#[test]
fn surrounding_whitespace_is_refused() {
    for text in [" #fff", "#fff ", "#ff f", "#fff\n"] {
        assert!(Rgba8::from_hex(text).is_err(), "{text:?} must be refused");
    }
}

#[test]
fn multibyte_text_of_the_right_byte_length_is_refused_without_panicking() {
    // 6 bytes after '#', but only 3 characters.
    assert!(Rgba8::from_hex("#ééé").is_err());
    assert!(Rgba8::from_hex("#日本").is_err());
}

#[test]
fn css_functions_are_not_hex() {
    assert!(Rgba8::from_hex("rgba(0, 0, 0, 1)").is_err());
    assert!(Rgba8::from_hex("red").is_err());
}
