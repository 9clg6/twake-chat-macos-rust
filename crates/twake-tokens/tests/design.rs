//! What the generator must also export (lot 1 d): `action.*`, shadows, zIndex, transitions, the font
//! family and the avatar gradients of twake-mui 19.3.0. Values are the ones of `tokens.json` today
//! (`schemes.*.palette.action`, `zIndex`, `transitions`, `typographyBase.fontFamily`) and of
//! `Avatar/helpers.js` (`colorMapping`). Public items: see the report of the spec author.

use twake_tokens::{
    AVATAR_COLOR_NAMES, AVATAR_GRADIENTS, DARK, DURATION_COMPLEX, DURATION_ENTERING_SCREEN,
    DURATION_LEAVING_SCREEN, DURATION_SHORT, DURATION_SHORTER, DURATION_SHORTEST,
    DURATION_STANDARD, EASE_IN, EASE_IN_OUT, EASE_OUT, FONT_FAMILY, Gradient, GradientStop, LIGHT,
    Rgba8, SHADOWS_DARK, SHADOWS_LIGHT, SHARP, Z_APP_BAR, Z_DRAWER, Z_FAB, Z_MOBILE_STEPPER,
    Z_MODAL, Z_SNACKBAR, Z_SPEED_DIAL, Z_TOOLTIP,
};

// ------------------------------------------------------------------ action

#[test]
fn light_action_colours_are_the_ink_at_the_alphas_of_twake_mui() {
    let a = LIGHT.action;
    assert_eq!(a.active, Rgba8::new(66, 66, 68, 143)); // 0.56
    assert_eq!(a.hover, Rgba8::new(66, 66, 68, 10)); // 0.04
    assert_eq!(a.selected, Rgba8::new(66, 66, 68, 20)); // 0.08
    assert_eq!(a.disabled, Rgba8::new(66, 66, 68, 61)); // 0.24
    assert_eq!(a.disabled_background, Rgba8::new(66, 66, 68, 31)); // 0.12
    assert_eq!(a.focus, Rgba8::new(66, 66, 68, 31)); // 0.12
}

#[test]
fn dark_action_colours_are_white_at_the_alphas_of_twake_mui() {
    let a = DARK.action;
    assert_eq!(a.active, Rgba8::new(255, 255, 255, 255));
    assert_eq!(a.hover, Rgba8::new(255, 255, 255, 20)); // 0.08
    assert_eq!(a.selected, Rgba8::new(255, 255, 255, 41)); // 0.16
    assert_eq!(a.disabled, Rgba8::new(255, 255, 255, 82)); // 0.32
    assert_eq!(a.disabled_background, Rgba8::new(255, 255, 255, 31)); // 0.12
    assert_eq!(a.focus, Rgba8::new(255, 255, 255, 61)); // 0.24
}

// ------------------------------------------------------------------ shadows

#[test]
fn shadows_keep_the_css_text_of_every_elevation_of_each_scheme() {
    assert_eq!(SHADOWS_LIGHT.len(), 26);
    assert_eq!(SHADOWS_DARK.len(), 26);
    assert_eq!(SHADOWS_LIGHT[0], "none");
    assert!(
        SHADOWS_LIGHT[1].contains("rgba(66, 66, 68, 0.08)"),
        "{}",
        SHADOWS_LIGHT[1]
    );
    assert_ne!(SHADOWS_LIGHT[1], SHADOWS_DARK[1]);
}

// ------------------------------------------------------------------ zIndex, transitions, font

#[test]
fn z_indexes_are_those_of_the_theme() {
    assert_eq!(Z_MOBILE_STEPPER, 1000);
    assert_eq!(Z_FAB, 1050);
    assert_eq!(Z_SPEED_DIAL, 1050);
    assert_eq!(Z_APP_BAR, 1100);
    assert_eq!(Z_DRAWER, 1200);
    assert_eq!(Z_MODAL, 1300);
    assert_eq!(Z_SNACKBAR, 1400);
    assert_eq!(Z_TOOLTIP, 1500);
}

#[test]
fn durations_are_in_milliseconds() {
    assert_eq!(DURATION_SHORTEST, 150);
    assert_eq!(DURATION_SHORTER, 200);
    assert_eq!(DURATION_SHORT, 250);
    assert_eq!(DURATION_STANDARD, 300);
    assert_eq!(DURATION_COMPLEX, 375);
    assert_eq!(DURATION_ENTERING_SCREEN, 225);
    assert_eq!(DURATION_LEAVING_SCREEN, 195);
}

#[test]
fn easings_are_cubic_beziers_x1_y1_x2_y2() {
    assert_eq!(EASE_IN_OUT, [0.4, 0.0, 0.2, 1.0]);
    assert_eq!(EASE_OUT, [0.0, 0.0, 0.2, 1.0]);
    assert_eq!(EASE_IN, [0.4, 0.0, 1.0, 1.0]);
    assert_eq!(SHARP, [0.4, 0.0, 0.6, 1.0]);
}

#[test]
fn the_font_family_starts_with_inter_and_ends_with_the_emoji_fonts() {
    assert!(FONT_FAMILY.starts_with("\"Inter\""), "{FONT_FAMILY}");
    assert!(FONT_FAMILY.contains("sans-serif"));
    assert!(
        FONT_FAMILY.ends_with("\"Segoe UI Symbol\""),
        "{FONT_FAMILY}"
    );
}

// ------------------------------------------------------------------ avatar colours

const fn stop(hex: u32, at_percent: f32) -> GradientStop {
    GradientStop {
        color: Rgba8::rgb(hex),
        at_percent,
    }
}

#[test]
fn the_names_are_the_supported_colors_in_order() {
    assert_eq!(
        AVATAR_COLOR_NAMES,
        [
            "sunrise",
            "downy",
            "sugarCoral",
            "pinkBonnet",
            "blueMana",
            "nightBlue",
            "snowPea",
            "pluviophile",
            "cornflower",
            "paleGreen",
            "moonBlue",
        ]
    );
}

#[test]
fn the_eleven_gradients_are_those_of_color_mapping_in_the_order_of_supported_colors() {
    let want = |angle_deg, from, to| Gradient {
        angle_deg,
        from,
        to,
    };
    let table = [
        want(136.0, stop(0xF8D280, 14.84), stop(0xF2AC69, 96.03)),
        want(136.0, stop(0x81EAD4, 14.84), stop(0x62C6B7, 96.03)),
        want(136.0, stop(0xF19E86, 14.84), stop(0xF95967, 96.03)),
        want(136.0, stop(0xE4ABF0, 14.84), stop(0xD96EED, 96.03)),
        want(136.0, stop(0x85D9FD, 14.84), stop(0x2A9EFC, 96.03)),
        want(136.0, stop(0x80AEFF, 39.32), stop(0x883DFE, 96.03)),
        want(136.0, stop(0xBDF4A1, 14.84), stop(0x52CE64, 96.03)),
        want(136.0, stop(0xA1D6F4, 14.84), stop(0x52CEC2, 96.03)),
        want(135.0, stop(0x86D9D3, 0.0), stop(0x1CCFB4, 100.0)),
        want(135.0, stop(0xE2FA17, 0.0), stop(0x75D8CB, 100.0)),
        want(136.0, stop(0x6DCFFF, 14.84), stop(0x3D88F8, 96.03)),
    ];
    assert_eq!(AVATAR_GRADIENTS, table);
}
