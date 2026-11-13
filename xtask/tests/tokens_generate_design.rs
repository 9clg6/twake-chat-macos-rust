//! `xtask::tokens::generate` exports more than the colours (lot 1 d of `2026-11-13-design-match.md`,
//! scope of A012 that A0xx lifts): `action.*` of each scheme, the shadows, `zIndex`, `transitions`,
//! `typographyBase.fontFamily` and the avatar gradients (`avatarColors`: `supportedColors` and
//! `colorMapping`, a new key of `export-tokens.mjs`). Whitespace-squashed comparison as in
//! `tokens_generate.rs`. Names of the generated items: see `crates/twake-tokens/tests/design.rs`.

use xtask::tokens::generate;

const TONE: &str =
    "\"light\":\"#d2e9ff\",\"main\":\"#0a84ff\",\"dark\":\"#006bd8\",\"contrastText\":\"#fff\"";
const SHADES: &str = "\"50\":\"#f6fbff\",\"100\":\"#e3f1ff\",\"200\":\"#d2e9ff\",\"300\":\"#bfdfff\",\"400\":\"#7bbdff\",\"500\":\"#46a2ff\",\"600\":\"#0a84ff\",\"700\":\"#006bd8\",\"800\":\"#0157ad\",\"900\":\"#082139\"";

fn squash(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

fn scheme(hover: &str, shadow: &str) -> String {
    format!(
        "{{\"palette\":{{\"mode\":\"x\",\"primary\":{{{TONE}}},\"secondary\":{{{TONE}}},\"error\":{{{TONE}}},\"warning\":{{{TONE}}},\"success\":{{{TONE}}},\"info\":{{{TONE}}},\
         \"text\":{{\"primary\":\"#fff\",\"secondary\":\"#fff\",\"disabled\":\"#fff\",\"icon\":\"#fff\"}},\
         \"background\":{{\"default\":\"#fff\",\"paper\":\"#fff\",\"contrast\":\"#fff\"}},\
         \"border\":{{\"main\":\"#111\",\"disabled\":\"#222\",\"ghost\":\"#333\",\"ghostDisabled\":\"#444\"}},\
         \"action\":{{\"active\":\"rgba(66, 66, 68, 0.56)\",\"hover\":\"{hover}\",\"selected\":\"rgba(1, 2, 3, 0.08)\",\"disabled\":\"rgba(1, 2, 3, 0.24)\",\"disabledBackground\":\"rgba(1, 2, 3, 0.12)\",\"focus\":\"rgba(1, 2, 3, 0.12)\"}},\
         \"divider\":\"#555\"}},\"shadows\":[\"none\",\"{shadow}\"]}}"
    )
}

fn json(extra: &str) -> String {
    format!(
        "{{\"twakeMuiVersion\":\"9.9.9\",\"paletteRaw\":{{\"Primary\":{{{SHADES},\"ContrastText\":\"#fff\"}}}},\
         \"schemes\":{{\"light\":{},\"dark\":{}}}{extra}}}",
        scheme(
            "rgba(66, 66, 68, 0.04)",
            "0px 1px 2px rgba(66, 66, 68, 0.08)"
        ),
        scheme(
            "rgba(255, 255, 255, 0.08)",
            "0px 1px 2px rgba(0, 0, 0, 0.08)"
        ),
    )
}

fn out(extra: &str) -> String {
    squash(&generate(&json(extra)).unwrap_or_default())
}

fn message(extra: &str) -> String {
    generate(&json(extra))
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default()
}

fn has(text: &str, wanted: &str) -> bool {
    text.contains(&squash(wanted))
}

// ------------------------------------------------------------------ action

#[test]
fn each_scheme_carries_its_action_colours_with_the_alpha_rounded() {
    let text = out("");
    assert!(
        has(&text, "Rgba8::new(66,66,68,10)"),
        "light hover 0.04 -> 10"
    );
    assert!(
        has(&text, "Rgba8::new(66,66,68,143)"),
        "light active 0.56 -> 143"
    );
    assert!(
        has(&text, "Rgba8::new(255,255,255,20)"),
        "dark hover 0.08 -> 20"
    );
    assert!(has(&text, "Rgba8::new(1,2,3,20)"), "selected 0.08 -> 20");
    assert!(has(&text, "Rgba8::new(1,2,3,61)"), "disabled 0.24 -> 61");
    assert!(has(&text, "disabled_background"));
    assert!(has(&text, "focus"));
}

#[test]
fn a_scheme_without_action_names_the_missing_group() {
    let broken = json("").replace("\"action\"", "\"actionX\"");
    let message = generate(&broken)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(message.contains("action"), "{message}");
}

#[test]
fn a_scheme_without_a_focus_colour_names_it() {
    let broken = json("").replace("\"focus\"", "\"focusX\"");
    let message = generate(&broken)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(message.contains("focus"), "{message}");
}

// ------------------------------------------------------------------ shadows

#[test]
fn shadows_keep_the_css_text_of_each_scheme() {
    let text = out("");
    assert!(has(&text, "SHADOWS_LIGHT"));
    assert!(has(&text, "SHADOWS_DARK"));
    assert!(has(&text, "\"0px 1px 2px rgba(66, 66, 68, 0.08)\""));
    assert!(has(&text, "\"0px 1px 2px rgba(0, 0, 0, 0.08)\""));
}

// ------------------------------------------------------------------ zIndex, transitions, font

#[test]
fn z_indexes_become_u32_constants_named_after_the_key() {
    let text = out(
        ",\"zIndex\":{\"mobileStepper\":1000,\"speedDial\":1050,\"appBar\":1100,\"modal\":1300}",
    );
    assert!(has(&text, "pub const Z_MOBILE_STEPPER: u32 = 1000;"));
    assert!(has(&text, "pub const Z_SPEED_DIAL: u32 = 1050;"));
    assert!(has(&text, "pub const Z_APP_BAR: u32 = 1100;"));
    assert!(has(&text, "pub const Z_MODAL: u32 = 1300;"));
}

#[test]
fn durations_become_u32_milliseconds_and_easings_four_floats() {
    let text = out(
        ",\"transitions\":{\"duration\":{\"shortest\":150,\"enteringScreen\":225},\
         \"easing\":{\"easeInOut\":\"cubic-bezier(0.4, 0, 0.2, 1)\",\"easeOut\":\"cubic-bezier(0.0, 0, 0.2, 1)\",\"sharp\":\"cubic-bezier(0.4, 0, 0.6, 1)\"}}",
    );
    assert!(has(&text, "pub const DURATION_SHORTEST: u32 = 150;"));
    assert!(has(&text, "pub const DURATION_ENTERING_SCREEN: u32 = 225;"));
    assert!(has(
        &text,
        "pub const EASE_IN_OUT: [f32; 4] = [0.4, 0.0, 0.2, 1.0];"
    ));
    assert!(has(
        &text,
        "pub const EASE_OUT: [f32; 4] = [0.0, 0.0, 0.2, 1.0];"
    ));
    assert!(has(
        &text,
        "pub const SHARP: [f32; 4] = [0.4, 0.0, 0.6, 1.0];"
    ));
}

#[test]
fn an_easing_that_is_not_a_cubic_bezier_is_refused_and_quoted() {
    let message = message(",\"transitions\":{\"duration\":{},\"easing\":{\"easeOut\":\"linear\"}}");
    assert!(message.contains("linear"), "{message}");
}

#[test]
fn the_font_family_keeps_the_whole_stack_with_its_quotes_escaped() {
    let text =
        out(",\"typographyBase\":{\"fontFamily\":\"\\\"Inter\\\",-apple-system,sans-serif\"}");
    assert!(has(
        &text,
        "pub const FONT_FAMILY: &str = \"\\\"Inter\\\",-apple-system,sans-serif\";"
    ));
}

// ------------------------------------------------------------------ avatar colours

const AVATARS: &str = ",\"avatarColors\":{\"supportedColors\":[\"sunrise\",\"cornflower\"],\"colorMapping\":{\
     \"cornflower\":\"linear-gradient(135deg, #86D9D3 0%, #1CCFB4 100%)\",\
     \"sunrise\":\"linear-gradient(136deg, #F8D280 14.84%, #F2AC69 96.03%)\"}}";

#[test]
fn the_gradients_follow_supported_colors_not_the_key_order_of_the_mapping() {
    let text = out(AVATARS);
    assert!(has(
        &text,
        "pub const AVATAR_COLOR_NAMES: [&str; 2] = [\"sunrise\", \"cornflower\"];"
    ));
    assert!(has(&text, "pub const AVATAR_GRADIENTS: [Gradient; 2]"));
    let sunrise = text.find(&squash("angle_deg: 136.0")).unwrap_or(usize::MAX);
    let cornflower = text.find(&squash("angle_deg: 135.0")).unwrap_or(0);
    assert!(sunrise < cornflower, "sunrise first");
}

#[test]
fn a_gradient_keeps_its_angle_its_two_colours_and_their_positions() {
    let text = out(AVATARS);
    assert!(has(&text, "Rgba8::new(248,210,128,255)"), "#F8D280");
    assert!(has(&text, "at_percent: 14.84"));
    assert!(has(&text, "at_percent: 96.03"));
    assert!(has(&text, "Rgba8::new(28,207,180,255)"), "#1CCFB4");
    assert!(has(&text, "at_percent: 100.0"));
}

#[test]
fn a_supported_color_without_a_gradient_names_it() {
    let broken = AVATARS.replace("\"cornflower\":\"linear", "\"other\":\"linear");
    let message = message(&broken);
    assert!(message.contains("cornflower"), "{message}");
}

#[test]
fn a_gradient_that_cannot_be_read_is_refused_and_quoted() {
    let broken = AVATARS.replace(
        "linear-gradient(135deg, #86D9D3 0%, #1CCFB4 100%)",
        "radial-gradient(red, blue)",
    );
    let message = message(&broken);
    assert!(message.contains("radial-gradient(red, blue)"), "{message}");
}

#[test]
fn the_output_is_deterministic_with_every_new_section() {
    let extra = format!(
        "{AVATARS},\"zIndex\":{{\"modal\":1300}},\"typographyBase\":{{\"fontFamily\":\"Inter\"}}"
    );
    assert_eq!(generate(&json(&extra)), generate(&json(&extra)));
}
