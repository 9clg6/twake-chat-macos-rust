# API sketch required by the ui-kit foundation tests

Written by the test author from the need, before the production code exists. The need:
`ARCHITECTURE.md` (`twake-ui-kit` mirrors `ds/` and twake-mui, `twake-gpui` is the only crate naming
GPUI, tokens and icons stay GPUI-free), `docs/spec/design-system.md` (section 3 Button / IconButton,
1.2 typography, 1.7 focus, 1.6 icons), `docs/spec/tokens.json` (`components.MuiButton`), decisions
A010 (UI on gpui 0.2.2 behind `twake-gpui`) and A013 (icons with hard-coded colours keep them).
Only what a Button, an IconAction and the theme need (YAGNI): no ghost variant, no Chip, no Avatar yet.
The pure half only: no window, no GPUI type in `twake-ui-kit` tests.

Tests (all red: the modules do not exist yet, so they fail to compile):
`crates/twake-ui-kit/tests/theme.rs`, `crates/twake-ui-kit/tests/button.rs`,
`crates/twake-ui-kit/tests/icon.rs`, `crates/twake-ui-kit/tests/color.rs`.
No dev-dependency added to `twake-gpui` (the colour glue lives in `twake-ui-kit`, section 4).

## 1. `twake_ui_kit::theme`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme { Light, Dark }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme { pub scheme: ColorScheme }

impl Theme {
    pub const fn new(scheme: ColorScheme) -> Self;
    pub const fn system(dark: bool) -> Self;        // the OS appearance: true is Dark
    pub fn colors(&self) -> &'static twake_tokens::Scheme;  // LIGHT or DARK
    pub fn toggle(&mut self);                       // Light <-> Dark
}
```

## 2. `twake_ui_kit::button` (twake-mui `Button`, same names)

```rust
pub enum ButtonVariant { Contained, Outlined, Text }          // ghost: later
pub enum ButtonColor   { Primary, Secondary, Error }
pub enum ButtonSize    { XSmall, Small, Medium, Large }       // MUI "xsmall|small|medium|large"
pub enum ButtonState   { Rest, Hover, Pressed, FocusVisible, Disabled }

pub struct Border    { pub width_px: f32, pub color: Rgba8 }
pub struct FocusRing { pub width_px: f32, pub color: Rgba8 }  // added: see open question 3

pub struct ResolvedButton {
    pub background: Rgba8,            // alpha 0 when there is none
    pub foreground: Rgba8,
    pub border: Option<Border>,       // outlined only
    pub focus_ring: Option<FocusRing>,// FocusVisible only
    pub radius_px: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub height_px: f32,               // the MUI min-height
    pub font: TypeSpec,
    pub opacity: f32,
}

pub struct ButtonStyle;
impl ButtonStyle {
    pub fn resolve(variant: ButtonVariant, color: ButtonColor, size: ButtonSize,
                   state: ButtonState, theme: &Theme) -> ResolvedButton;
}
```

Values (sources in the comments of `button.rs`; "MUI" = default Button of MUI v9, which twake-mui
only restyles for geometry):

- Radius 100 for every variant, size and state (pill).
- Height (min-height) xsmall 24 / small 36 / medium 40 / large 48.
- Padding (x, y), `tokens.json components.MuiButton`: contained medium 24 x 10, outlined 15 x 5,
  text 8 x 10; small 16 x 9 (text 16 x 8); large 32 x 13 (text 10 x 14); xsmall 8 x 4 (outlined 7 x 3).
- Font: `TYPE_BUTTON` (14 / 500 / 1.313 / 0.1); xsmall 11 / 500 / 16 px / 0.5.
- Opacity 1 in every state (a disabled button changes its colours, MUI does not fade it).
- Contained: bg `tone.main`, fg `tone.contrast_text`, no border; Hover and Pressed bg `tone.dark`.
- Outlined: bg transparent, fg `tone.main`, border 1 px in `tone.main` (twake: `borderColor: currentColor`);
  Hover and Pressed bg = `tone.main` at the scheme's hover opacity (alpha 10 light, 20 dark).
- Text: as outlined without a border.
- Disabled: fg = `action.disabled` (light `rgba(66,66,68,.24)` = alpha 61, dark white alpha 82); contained bg =
  `action.disabledBackground` (alpha 31 in both); outlined and text bg transparent.
- FocusVisible: colours of Rest plus a 2 px ring in `primary.main` of the scheme, whatever the button colour
  (design-system.md 1.7). Every other state has no ring.
- Alpha bytes are the rounded `0.xx * 255`, the rule `twake-tokens` already follows (`0.9` is 230).

## 3. `twake_ui_kit::icon`

```rust
pub enum IconColor { Current, Fixed(Rgba8) }
pub struct IconSpec { pub icon: &'static Icon, pub size_px: f32, pub color: IconColor }
pub fn svg_for(spec: &IconSpec) -> String;
```

`svg_for` is `Icon::to_svg` with `width="{size}" height="{size}"` on the root (Rust `Display`
spelling: `20`, `14.5`), whatever the viewBox (the box scales with its aspect kept, as `<Icon size>`).
`Current` keeps `fill="currentColor"`. `Fixed(c)` replaces it by `fill="#rrggbb"` (lowercase) or
`fill="#rrggbbaa"` when alpha is not 255. The body is never touched. An icon whose body holds a hard-coded
colour (`fill="#..."`, A013) is never recoloured: `svg_for` gives the same document for `Current` and `Fixed`.

## 4. `twake_ui_kit::color` colour glue

The module lives in `twake-ui-kit`, not in `twake-gpui`: the edge `twake-gpui -> twake-tokens` is forbidden by
ARCHITECTURE.md. `Rgba` is `gpui::Rgba`, reached through the `twake_gpui::Rgba` re-export.

```rust
pub fn to_gpui_rgba(color: Rgba8) -> Rgba;     // Rgba = gpui::Rgba, channels n / 255.0 in 0..=1
pub fn from_gpui_rgba(color: Rgba) -> Rgba8;   // channel * 255 rounded, clamped to 0..=255
```

## Open questions (the need is silent; the tests pin the proposal above)

1. Pressed: decided, pressed == hover. MUI Button has no pressed colour (only a ripple), tested equal to Hover.
2. Outlined small and large padding: not written in `tokens.json`; derived from the cascade order of
   `MuiButton` (size rules come after the variant ones): 16 x 9 and 32 x 13.
3. The requested `ResolvedButton` had no field for the focus ring, which design-system.md 1.7 requires on
   every keyboard-reached control; `focus_ring` was added. The `color` argument was added to `resolve`
   (the request listed colours in the need but not in the signature).
4. `twake-tokens` has no `action` colours (hover opacity, disabled, disabled background). The tests expect
   the values of design-system.md lines 107-114 whatever their home. Decided (A014): the `action` colours
   (hover, disabled, disabledBackground) are constants of `twake-ui-kit` for now.
5. Disabled outlined border: twake sets `borderColor: currentColor`, MUI sets the disabled border to
   `action.disabledBackground`; the cascade is not decidable from the export. Not tested.
6. Settled: the functions live in `twake_ui_kit::color`, so `twake-gpui` needs no edge to `twake-tokens`.
