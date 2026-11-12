# Twake design system, extracted for a Rust / GPUI client

Source: `twake-space-chat` (React client). Extracted 2026-11-12.
Companion files: `tokens.json` (machine export of everything numeric below) and
`export-tokens.mjs` (the script that produced it).

Provenance, in order of authority:

1. `@linagora/twake-mui` **19.3.0** (MUI v9 theme system), `dist/lib/*.js`: `makePalette`, `makeTypography`,
   `makeShadows`, `makeTheme`, `radius`, `overrides` (1369 lines). Built from `linagora/twake-ui`, `packages/twake-mui`.
2. `@linagora/twake-css` palette (`palette.json`, the raw colour scales) and `@linagora/twake-icons` 3.0.0 (SVG icons).
3. `src/app/theme.ts` (what Twake Chat changes on top of twake-mui: `ACCESSIBLE_THEME`).
4. `src/ds/*` (the local design system: 115 component folders) and `src/ds/figma-map.json`.
5. `AGENTS.md`, `DECISION.md` (D23, D34, D158, D183, D209, D245, D261, D626...), `docs/accessibility.md`.

Figma was not queried: the code is the source of truth, and `figma-map.json` already names the Figma
component of each mapped `ds` component (Library Twake, file key `6Sf5ux54xWxOjwN3j7gL78`).

Conventions used below: px are CSS px (map 1:1 to GPUI logical pixels, `px()`); `sx` numbers on spacing props
(`gap`, `p`, `px`, `m`...) are multiples of the **8 px spacing unit**; `borderRadius: n` in `sx` is `n x 6 px`
(`shape.borderRadius`), so `1` = 6 px and `2` = 12 px, whereas a string (`'16px'`) is literal.

---

## 1. Tokens

### 1.1 Colour model

The theme has two colour schemes (`light`, `dark`) and is emitted as CSS variables
`--twake-palette-<group>-<key>` (selector `[data-theme="dark"]`; prefix `twake`). The app follows the
system or the user setting (D261); dark swaps every palette value, not just the background.
Alpha colours are `rgba()` of Grey 900 (`#424244`) in light and of white in dark.

**Raw scales** (`twake-css/palette.json`; these are the *inputs*, the semantic palette picks from them):

| Scale | 50 | 100 | 200 | 300 | 400 | 500 | 600 | 700 | 800 | 900 | Other |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Primary | `#f6fbff` | `#e3f1ff` | `#d2e9ff` | `#bfdfff` | `#7bbdff` | `#46a2ff` | `#0a84ff` | `#006bd8` | `#0157ad` | `#082139` | ContrastText `#fff` |
| Secondary | `#FCFCFD` | `#F2F2F3` | `#D8D7DA` | `#BEBDC2` | `#A4A2A9` | `#8D8A93` | `#7A7781` | `#64626A` | `#55535A` | `#3A383D` | ContrastText `#fff` |
| Error | `#fffbfe` | `#fceeee` | `#ffd2d7` | `#ff939d` | `#ff7a87` | `#ff4d5e` | `#ff3347` | `#d32c3c` | `#b52c39` | `#7a1e27` | ContrastText `#fff` |
| Warning | `#fff8e1` | `#ffecb3` | `#ffe082` | `#ffd54f` | `#ffca28` | `#ffb300` | `#ff8f00` | `#ff6f00` | `#b5672c` | `#281c07` | ContrastText `#fff` |
| Success | `#f3fff5` | `#e3ffe7` | `#b9f6ca` | `#97eca1` | `#66e475` | `#38c949` | `#00c853` | `#00ad48` | `#245538` | `#08250b` | ContrastText `#fff` |
| Info | `#edffff` | `#d4feff` | `#affdff` | `#86f5f7` | `#4ee2e5` | `#1de9b6` | `#00bfa5` | `#00796b` | `#004345` | `#002526` |  |
| Grey | `#FBFDFF` | `#F3F6F9` | `#EDF2F7` | `#E1E3E6` | `#C5C7CA` | `#AAACAE` | `#909294` | `#757779` | `#4E5052` | `#424244` | A100 `#FCFCFC`, A200 `#E5ECF3`, A400 `#2C3039`, A700 `#5D6169`, A900 `#1C1B1F` |
| Common | white `#fff` black `#000` | | | | | | | | | | |


**Semantic palette** (what components read; 70 keys, both schemes). `mainChannel`-style
variables (`"10 132 255"`) also exist for every colour and are used to build `rgba(var / alpha)`:

| Key (`--twake-palette-…`) | Light | Dark |
|---|---|---|
| `primary-light` | `#d2e9ff` | `#bfdfff` |
| `primary-main` | `#0a84ff` | `#7bbdff` |
| `primary-dark` | `#006bd8` | `#46a2ff` |
| `primary-contrastText` | `#fff` | `rgba(66, 66, 68, 0.9)` |
| `secondary-light` | `#F2F2F3` | `#E1E3E6` |
| `secondary-main` | `#7A7781` | `#C5C7CA` |
| `secondary-dark` | `#64626A` | `#AAACAE` |
| `secondary-contrastText` | `#fff` | `rgba(66, 66, 68, 0.9)` |
| `error-light` | `#ffd2d7` | `#ff939d` |
| `error-main` | `#ff3347` | `#ff7a87` |
| `error-dark` | `#d32c3c` | `#ff4d5e` |
| `error-contrastText` | `#fff` | `rgba(66, 66, 68, 0.9)` |
| `warning-light` | `#ffe082` | `#ffd54f` |
| `warning-main` | `#ff8f00` | `#ffca28` |
| `warning-dark` | `#ff6f00` | `#ffb300` |
| `warning-contrastText` | `#fff` | `rgba(66, 66, 68, 0.9)` |
| `success-light` | `#b9f6ca` | `#97eca1` |
| `success-main` | `#00c853` | `#66e475` |
| `success-dark` | `#00ad48` | `#38c949` |
| `success-contrastText` | `#fff` | `rgba(66, 66, 68, 0.9)` |
| `info-light` | `#affdff` | `#86f5f7` |
| `info-main` | `#00bfa5` | `#4ee2e5` |
| `info-dark` | `#00796b` | `#1de9b6` |
| `info-contrastText` | `#fff` | `rgba(66, 66, 68, 0.9)` |
| `grey-50` | `#FBFDFF` | `#FBFDFF` |
| `grey-100` | `#F3F6F9` | `#F3F6F9` |
| `grey-200` | `#EDF2F7` | `#EDF2F7` |
| `grey-300` | `#E1E3E6` | `#E1E3E6` |
| `grey-400` | `#C5C7CA` | `#C5C7CA` |
| `grey-500` | `#AAACAE` | `#AAACAE` |
| `grey-600` | `#909294` | `#909294` |
| `grey-700` | `#757779` | `#757779` |
| `grey-800` | `#4E5052` | `#4E5052` |
| `grey-900` | `#424244` | `#424244` |
| `grey-A100` | `#FCFCFC` | `#FCFCFC` |
| `grey-A200` | `#E5ECF3` | `#E5ECF3` |
| `grey-A400` | `#2C3039` | `#2C3039` |
| `grey-A700` | `#5D6169` | `#5D6169` |
| `grey-A900` | `#1C1B1F` | `#1C1B1F` |
| `text-primary` | `rgba(66, 66, 68, 0.9)` | `#fff` |
| `text-secondary` | `rgba(66, 66, 68, 0.64)` | `rgba(255, 255, 255, 0.64)` |
| `text-disabled` | `rgba(66, 66, 68, 0.32)` | `rgba(255, 255, 255, 0.4)` |
| `text-icon` | `rgba(66, 66, 68, 0.72)` | `#fff` |
| `background-default` | `#F3F6F9` | `#2C3039` |
| `background-paper` | `#fff` | `#4E5052` |
| `background-contrast` | `rgba(66, 66, 68, 0.048)` | `rgba(255, 255, 255, 0.072)` |
| `border-main` | `rgba(66, 66, 68, 0.16)` | `rgba(255, 255, 255, 0.24)` |
| `border-disabled` | `rgba(66, 66, 68, 0.08)` | `rgba(255, 255, 255, 0.12)` |
| `border-ghost` | `rgba(66, 66, 68, 0.48)` | `rgba(255, 255, 255, 0.48)` |
| `border-ghostDisabled` | `rgba(66, 66, 68, 0.16)` | `rgba(255, 255, 255, 0.24)` |
| `border-opacity` | `0.32` | `0.48` |
| `border-ghostOpacity` | `0.48` | `0.48` |
| `divider` | `rgba(66, 66, 68, 0.12)` | `rgba(255, 255, 255, 0.16)` |
| `action-active` | `rgba(66, 66, 68, 0.56)` | `#fff` |
| `action-hover` | `rgba(66, 66, 68, 0.04)` | `rgba(255, 255, 255, 0.08)` |
| `action-hoverOpacity` | `0.04` | `0.08` |
| `action-selected` | `rgba(66, 66, 68, 0.08)` | `rgba(255, 255, 255, 0.16)` |
| `action-selectedOpacity` | `0.08` | `0.16` |
| `action-disabled` | `rgba(66, 66, 68, 0.24)` | `rgba(255, 255, 255, 0.32)` |
| `action-disabledOpacity` | `0.32` | `0.32` |
| `action-disabledBackground` | `rgba(66, 66, 68, 0.12)` | `rgba(255, 255, 255, 0.12)` |
| `action-focus` | `rgba(66, 66, 68, 0.12)` | `rgba(255, 255, 255, 0.24)` |
| `action-focusOpacity` | `0.12` | `0.24` |
| `action-activatedOpacity` | `0.12` | `0.24` |
| `action-ghostOpacity` | `0.08` | `0.08` |
| `action-hoverGhostOpacity` | `0.16` | `0.16` |
| `common-black` | `#000` | `#000` |
| `common-white` | `#fff` | `#fff` |
| `common-background` | `#fff` | `#000` |
| `common-onBackground` | `#000` | `#fff` |

Rules read from `makePalette`:

- Light: `primary.main`=Primary 600 `#0a84ff`, `light`=200, `dark`=700. `secondary.main`=Secondary 600, `light`=100, `dark`=700.
  Same pattern for error / warning / success / info (light=200, main=600, dark=700). `info.contrastText` is `#fff`.
- Dark: `main`=scale 400, `light`=300, `dark`=500, `contrastText`=`rgba(66,66,68,0.9)` (dark text on the pastel). `secondary` in dark is the *Grey* scale 400/300/500.
- `background.default` light = Grey 100 `#F3F6F9`; dark = Grey A400 `#2C3039`. `background.paper` light = white; dark = Grey 800 `#4E5052`.
  `background.contrast` = a 4.8 % (light) / 7.2 % (dark) tint used for disabled inputs and segmented tabs.
- Text is Grey 900 at 90 / 64 / 32 / 72 % (primary / secondary / disabled / icon) in light; white at 100 / 64 / 40 / 100 % in dark.
- Interaction opacities: hover 4 % (light) / 8 % (dark), selected 8 / 16 %, focus 12 / 24 %, disabled-background 12 %; "ghost" fills use 8 % rest and 16 % hover of the colour.
- The app does **not** darken the brand colours (D23 was reversed by D245, 2026-10-20): `#0a84ff` stays, axe `color-contrast` is switched off. Take colours from the theme only.

**Component-level colour tokens** (palette extensions twake-mui adds for Alert, AppBar, Avatar, Button, Chip, inputs, Tooltip...):

| Key | Light | Dark |
|---|---|---|
| `Alert.errorColor` | `rgb(102, 84, 86)` | `rgb(255, 211, 215)` |
| `Alert.infoColor` | `rgb(70, 101, 102)` | `rgb(206, 251, 251)` |
| `Alert.successColor` | `rgb(74, 98, 80)` | `rgb(213, 247, 217)` |
| `Alert.warningColor` | `rgb(102, 89, 52)` | `rgb(255, 238, 184)` |
| `Alert.errorFilledBg` | `var(--twake-palette-error-main, #ff3347)` | `var(--twake-palette-error-dark, #ff4d5e)` |
| `Alert.infoFilledBg` | `var(--twake-palette-info-main, #00bfa5)` | `var(--twake-palette-info-dark, #1de9b6)` |
| `Alert.successFilledBg` | `var(--twake-palette-success-main, #00c853)` | `var(--twake-palette-success-dark, #38c949)` |
| `Alert.warningFilledBg` | `var(--twake-palette-warning-main, #ff8f00)` | `var(--twake-palette-warning-dark, #ffb300)` |
| `Alert.errorFilledColor` | `#fff` | `#fff` |
| `Alert.infoFilledColor` | `rgba(0, 0, 0, 0.87)` | `rgba(0, 0, 0, 0.87)` |
| `Alert.successFilledColor` | `rgba(0, 0, 0, 0.87)` | `rgba(0, 0, 0, 0.87)` |
| `Alert.warningFilledColor` | `rgba(0, 0, 0, 0.87)` | `rgba(0, 0, 0, 0.87)` |
| `Alert.errorStandardBg` | `rgb(255, 250, 251)` | `rgb(25, 14, 15)` |
| `Alert.infoStandardBg` | `rgb(247, 254, 255)` | `rgb(13, 24, 24)` |
| `Alert.successStandardBg` | `rgb(248, 254, 249)` | `rgb(15, 23, 16)` |
| `Alert.warningStandardBg` | `rgb(255, 251, 242)` | `rgb(25, 21, 7)` |
| `Alert.errorIconColor` | `var(--twake-palette-error-main, #ff3347)` | `var(--twake-palette-error-main, #ff7a87)` |
| `Alert.infoIconColor` | `var(--twake-palette-info-main, #00bfa5)` | `var(--twake-palette-info-main, #4ee2e5)` |
| `Alert.successIconColor` | `var(--twake-palette-success-main, #00c853)` | `var(--twake-palette-success-main, #66e475)` |
| `Alert.warningIconColor` | `var(--twake-palette-warning-main, #ff8f00)` | `var(--twake-palette-warning-main, #ffca28)` |
| `AppBar.defaultBg` | `var(--twake-palette-grey-100, #F3F6F9)` | `var(--twake-palette-grey-900, #424244)` |
| `Avatar.defaultBg` | `var(--twake-palette-grey-400, #C5C7CA)` | `var(--twake-palette-grey-600, #909294)` |
| `Button.inheritContainedBg` | `var(--twake-palette-grey-300, #E1E3E6)` | `var(--twake-palette-grey-800, #4E5052)` |
| `Button.inheritContainedHoverBg` | `var(--twake-palette-grey-A100, #FCFCFC)` | `var(--twake-palette-grey-700, #757779)` |
| `Chip.defaultBorder` | `var(--twake-palette-grey-400, #C5C7CA)` | `var(--twake-palette-grey-700, #757779)` |
| `Chip.defaultAvatarColor` | `var(--twake-palette-grey-700, #757779)` | `var(--twake-palette-grey-300, #E1E3E6)` |
| `Chip.defaultIconColor` | `var(--twake-palette-grey-700, #757779)` | `var(--twake-palette-grey-300, #E1E3E6)` |
| `FilledInput.bg` | `rgba(0, 0, 0, 0.06)` | `rgba(255, 255, 255, 0.09)` |
| `FilledInput.hoverBg` | `rgba(0, 0, 0, 0.09)` | `rgba(255, 255, 255, 0.13)` |
| `FilledInput.disabledBg` | `rgba(0, 0, 0, 0.12)` | `rgba(255, 255, 255, 0.12)` |
| `Input.autofillWebkitBoxShadow` | `none` | `0 0 0 100px #266798 inset` |
| `LinearProgress.primaryBg` | `rgb(161, 208, 255)` | `rgb(61, 94, 127)` |
| `LinearProgress.secondaryBg` | `rgb(204, 203, 207)` | `rgb(98, 99, 101)` |
| `LinearProgress.errorBg` | `rgb(255, 177, 185)` | `rgb(127, 61, 67)` |
| `LinearProgress.infoBg` | `rgb(158, 230, 220)` | `rgb(39, 113, 114)` |
| `LinearProgress.successBg` | `rgb(158, 234, 189)` | `rgb(51, 114, 58)` |
| `LinearProgress.warningBg` | `rgb(255, 212, 158)` | `rgb(127, 101, 20)` |
| `Skeleton.bg` | `rgba(var(--twake-palette-text-primaryChannel, 66 66 68) / 0.11)` | `rgba(var(--twake-palette-text-primaryChannel, 255 255 255) / 0.13)` |
| `Slider.primaryTrack` | `rgb(161, 208, 255)` | `rgb(61, 94, 127)` |
| `Slider.secondaryTrack` | `rgb(204, 203, 207)` | `rgb(98, 99, 101)` |
| `Slider.errorTrack` | `rgb(255, 177, 185)` | `rgb(127, 61, 67)` |
| `Slider.infoTrack` | `rgb(158, 230, 220)` | `rgb(39, 113, 114)` |
| `Slider.successTrack` | `rgb(158, 234, 189)` | `rgb(51, 114, 58)` |
| `Slider.warningTrack` | `rgb(255, 212, 158)` | `rgb(127, 101, 20)` |
| `SnackbarContent.bg` | `rgb(48, 49, 49)` | `rgb(250, 250, 251)` |
| `SnackbarContent.color` | `#fff` | `rgba(0, 0, 0, 0.87)` |
| `SpeedDialAction.fabHoverBg` | `rgb(216, 216, 216)` | `rgb(104, 106, 107)` |
| `StepConnector.border` | `var(--twake-palette-grey-400, #C5C7CA)` | `var(--twake-palette-grey-600, #909294)` |
| `StepContent.border` | `var(--twake-palette-grey-400, #C5C7CA)` | `var(--twake-palette-grey-600, #909294)` |
| `Switch.defaultColor` | `var(--twake-palette-common-white, #fff)` | `var(--twake-palette-grey-300, #E1E3E6)` |
| `Switch.defaultDisabledColor` | `var(--twake-palette-grey-100, #F3F6F9)` | `var(--twake-palette-grey-600, #909294)` |
| `Switch.primaryDisabledColor` | `rgb(161, 208, 255)` | `rgb(55, 85, 114)` |
| `Switch.secondaryDisabledColor` | `rgb(204, 203, 207)` | `rgb(88, 89, 90)` |
| `Switch.errorDisabledColor` | `rgb(255, 177, 185)` | `rgb(114, 54, 60)` |
| `Switch.infoDisabledColor` | `rgb(158, 230, 220)` | `rgb(35, 101, 103)` |
| `Switch.successDisabledColor` | `rgb(158, 234, 189)` | `rgb(45, 102, 52)` |
| `Switch.warningDisabledColor` | `rgb(255, 212, 158)` | `rgb(114, 90, 18)` |
| `TableCell.border` | `rgba(232, 232, 232, 1)` | `rgba(81, 81, 81, 1)` |
| `Tooltip.bg` | `rgba(117, 119, 121, 0.92)` | `rgba(117, 119, 121, 0.92)` |

Fixed colours outside the palette (from `src/ds`):

| Where | Value |
|---|---|
| Tooltip background | `rgba(78,80,82,0.9)` (Grey 800 at 90 %) |
| Chat wallpaper under-image colour | light `rgb(222 228 252)`, dark `rgb(44 52 72)`; images `public/images/chat-bg-light.jpg` / `chat-bg-dark.jpg`, `cover`, fixed (D192); none inside Twake Space or when the user picks "plain paper" (then `background.paper`) |
| Own message bubble | `color-mix(in srgb, primary.main 12%, background.paper)`; other bubbles `background.paper`; radius 16 px |
| Presence dot | online `success.main`, else `warning.main`, ringed by 2 px `background.paper` |
| Disabled avatar | `#d6d8da`, image `grayscale(1) brightness(2)` at 50 % |
| Scrollbars | thumb `rgba(128,128,128,0.45)`, 8 px wide, radius 4, transparent track |
| Backdrop | `rgba(0,0,0,0.12)` light, `rgba(0,0,0,0.5)` dark |

**High-contrast mode** (`html[data-contrast="more"]`, `theme.ts`, measured >= 7:1). Overrides in light: text-primary `#1A1A1C`,
text-secondary/icon/action-active `#4A4A4F`, primary-main `#0055B8`, primary-dark `#003F8A`, primary-light `#D2E9FF`, error-main `#B3001B`,
divider/text-disabled `#6B6B70`. In dark: background-default `#000000`, paper `#121212`, text-primary `#FFFFFF`, text-secondary `#D0D0D0`,
primary-main `#7CC0FF` (contrast text `#000000`), error-main `#FF8A95` (contrast `#000000`), divider `#9A9A9F`. Extra: edges of fields, chips and bubbles get `1px solid divider`;
current / pressed / selected get an inset 2 px primary ring; focus ring becomes 3 px `text.primary` with 2 px offset. Also `html[data-grayscale]` = `filter: grayscale(1)` on the root.

### 1.2 Typography

Font family stack (as emitted): `"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif, "Apple Color Emoji", "Segoe UI Emoji", "Segoe UI Symbol"`.
The font file itself is not bundled by twake-mui (check `index.html` / `public/` of the app; Inter must be shipped with the macOS client). `htmlFontSize` 16, base `fontSize` 14,
weights 300 / 400 / 500 / 700 (semi-bold 600 is used directly). No text-transform anywhere (`button`, `overline` set `inherit`).

| Variant | Size px | Weight | Line-height | Letter-spacing | Notes |
|---|---|---|---|---|---|
| h1 | 45 | 600 | 1.087 (= 48.9 px) | 0 |  |
| h2 | 32 | 600 | 1.313 (= 42.0 px) | 0 |  |
| h3 | 24 | 600 | 1.167 (= 28.0 px) | 0 |  |
| h4 | 22 | 600 | 1.167 (= 25.7 px) | 0 |  |
| h5 | 16 | 600 | 1.313 (= 21.0 px) | 0.15px |  |
| h6 | 14 | 500 | 1.313 (= 18.4 px) | 0.1px |  |
| subtitle1 | 14 | 500 | 1.358 (= 19.0 px) | 0.1px |  |
| subtitle2 | 14 | 600 | 1.313 (= 18.4 px) | 0.25px |  |
| body1 | 16 | 400 | 1.313 (= 21.0 px) | -0.15px |  |
| body2 | 14 | 500 | 1.313 (= 18.4 px) | 0.25px |  |
| body3 | 14 | 400 | 1.313 (= 18.4 px) | 0.25px |  |
| button | 14 | 500 | 1.313 (= 18.4 px) | 0.1px | no text-transform |
| caption | 12 | 500 | 1.317 (= 15.8 px) | 0.5px |  |
| overline | 11 | 500 | 13px | 0.4px | no text-transform |


Notes:

- `lineHeight` is unitless (multiply by the font size); `overline` is the only fixed px line-height (13 px). Letter-spacing is in px.
- `body3` is a twake-mui addition (14/400); `Typography` maps it to `<p>`. (An override in `MuiTypography` sets `text.secondary` under a condition the export could not resolve; see `tokens.json`.)
- `DialogTitle` = h3 (24 px, 600) on >= 769 px, h4 (22 px) below. `Tab` = 14 / 600 / 0.25 px; `ListSubheader` = 14 / 600.
- Components in `ds` often set sizes through `rem(px)` (= `theme.typography.pxToRem`), so they follow the **text-size setting** (D261): small 0.9, normal 1, large 1.15, multiplied into every typography size and every `pxToRem`. The Rust crate needs a global `text_scale`.
- Names start with a capital letter in lists, message authors and activity (`::first-letter { text-transform: uppercase }`, class `capitalised-name`, D209); the stored name is untouched, copy yields the typed text.
- Common local sizes: row title 14 / 500 (`rem(14)`), chip 12 / 500 / 0.5 px (`rem(12)`, D626), bottom-nav label 11 px / 16 px line (`rem(11)`), unread badge 10 px.

### 1.3 Spacing, shape, breakpoints, z-index

| Token | Value |
|---|---|
| Spacing unit | **8 px** (`var(--twake-spacing, 8px)`; `theme.spacing(n)` = n x 8) |
| `shape.borderRadius` | **6 px** (default for cards, accordion, dialog on small screens) |
| radius.none / sm / md / lg / pill | 0 / **4** / **8** / **12** / **100 px** |
| Breakpoints | xs 0, **sm 544**, **md 769**, **lg 1024**, **xl 1201** (note: md is 769, not 768; media queries use `max-width: 768.95px`) |
| z-index | mobileStepper 1000, fab 1050, speedDial 1050, appBar 1100, drawer 1200, modal 1300, snackbar 1400, tooltip 1500 |

Radii in practice: outlined input 4, tooltip 4, menu / accordion 6, chip pill 100 (reaction chips 16, "square" chip 8), button 100 (pill), dialog 6 (small screens) else MUI default,
bubble 16, FAB 28 / 16 / 12 (large / medium / small), segmented tabs 99, avatar circle 50 %.

**Screen layout constants** (`src/ds/ChatShell/layout.ts`):

| Constant | px |
|---|---|
| `PANE_WIDTH` (room list column and side pane, equal) | 360 |
| `MIN_CONVERSATION_WIDTH` (below this the pane floats over it, D220) | 360 |
| `PANE_GAPS` (large screens: two between three panes, one at the edge) | 24 |
| `THREAD_WINDOW_WIDTH` / `_FOLDED_WIDTH` | 420 / 320 |
| `THREAD_WINDOW_BOTTOM_OFFSET` (keeps the send button clear) | 88 |
| `CALL_CONVERSATION_WIDTH` | 380 |

The side pane is a column beside the conversation from 769 px up, as wide as the list; the frame separates it with a 1 px `divider` line (D158). Scrolling content keeps 16 px from frame edges; the composer touches the bottom edge. Below 769 px one pane shows at a time. The page is `background.paper` (no card) inside Twake Space (D158).

### 1.4 Shadows / elevation

`makeShadows`: 26 entries (`none`, 24 elevations, one flat extra). Each elevation = two offset layers (x always 0) + a constant 0.5 px hairline `0 0 0 0.5px rgba(shadowColor, 0.12)`.
`shadowColor` = Grey 900 `#424244` in light, `#000` in dark. Tuple table `[y1, blur1, spread1, a1, y2, blur2, spread2, a2]`:

```
 1 [2,4,0,.08,   4,16,0,.06]    9 [5,8,-3,.14,  7,27,3,.12]    17 [9,12,-6,.19, 10,38,6,.19]
 2 [3,5,0,.09,   4,17,0,.07]   10 [6,9,-3,.14,  7,29,3,.13]    18 [9,13,-6,.20, 10,40,6,.19]
 3 [3,5,-1,.09,  5,19,1,.08]   11 [6,9,-3,.15,  7,30,3,.14]    19 [10,13,-6,.21,10,41,6,.20]
 4 [3,6,-1,.10,  5,20,1,.08]   12 [7,10,-4,.16, 8,31,4,.15]    20 [10,14,-7,.21,11,42,7,.21]
 5 [4,6,-1,.11,  5,22,1,.09]   13 [7,10,-4,.16, 8,33,4,.15]    21 [11,14,-7,.22,11,44,7,.22]
 6 [4,7,-2,.11,  6,23,2,.10]   14 [8,11,-5,.17, 9,34,5,.16]    22 [11,15,-7,.23,11,45,7,.22]
 7 [5,7,-2,.12,  6,24,2,.11]   15 [8,11,-5,.18, 9,35,5,.17]    23 [12,15,-8,.23,12,47,8,.23]
 8 [5,8,-2,.13,  6,26,2,.11]   16 [9,12,-5,.18, 9,37,5,.18]    24 [12,16,-8,.24,12,48,8,.24]
25 (extra, flat) = 0 1px 4px 0 rgba(shadowColor, .04)
```

Resolved samples (light):

| Elevation | Light shadow (Grey900 #424244 at alpha) |
|---|---|
| 0 | `none` |
| 1 | `0px 2px 4px 0px rgba(66, 66, 68, 0.08), 0px 4px 16px 0px rgba(66, 66, 68, 0.06), 0px 0px 0px 0.5px rgba(66, 66, 68, 0.12)` |
| 2 | `0px 3px 5px 0px rgba(66, 66, 68, 0.09), 0px 4px 17px 0px rgba(66, 66, 68, 0.07), 0px 0px 0px 0.5px rgba(66, 66, 68, 0.12)` |
| 4 | `0px 3px 6px -1px rgba(66, 66, 68, 0.1), 0px 5px 20px 1px rgba(66, 66, 68, 0.08), 0px 0px 0px 0.5px rgba(66, 66, 68, 0.12)` |
| 8 | `0px 5px 8px -2px rgba(66, 66, 68, 0.13), 0px 6px 26px 2px rgba(66, 66, 68, 0.11), 0px 0px 0px 0.5px rgba(66, 66, 68, 0.12)` |
| 12 | `0px 7px 10px -4px rgba(66, 66, 68, 0.16), 0px 8px 31px 4px rgba(66, 66, 68, 0.15), 0px 0px 0px 0.5px rgba(66, 66, 68, 0.12)` |
| 16 | `0px 9px 12px -5px rgba(66, 66, 68, 0.18), 0px 9px 37px 5px rgba(66, 66, 68, 0.18), 0px 0px 0px 0.5px rgba(66, 66, 68, 0.12)` |
| 24 | `0px 12px 16px -8px rgba(66, 66, 68, 0.24), 0px 12px 48px 8px rgba(66, 66, 68, 0.24), 0px 0px 0px 0.5px rgba(66, 66, 68, 0.12)` |
| 25 | `0px 1px 4px rgba(66, 66, 68, 0.04)` |

`shadows[1]` is the everyday shadow (switch thumb, segmented-tab indicator, accordion). GPUI supports multiple `BoxShadow`s per element, so the 3-layer string maps directly. Dark mode additionally sets `overlays` to `none` (no elevation tint).

### 1.5 Motion

MUI durations (ms): shortest 150, shorter 200, short 250, standard 300, complex 375, enteringScreen 225, leavingScreen 195.
Easings: easeInOut `cubic-bezier(0.4,0,0.2,1)`, easeOut `(0,0,0.2,1)`, easeIn `(0.4,0,1,1)`, sharp `(0.4,0,0.6,1)`.

App motion (`src/ds/motion.ts`, DECISION D34 "little motion, where it helps"): `EASE_OUT = cubic-bezier(0.23,1,0.32,1)` (what answers the user), `EASE_DRAWER = cubic-bezier(0.32,0.72,0,1)` (slides from an edge).
`Arriving` plays `opacity 0 -> 1` plus a `transform` offset once on mount, `animation-fill-mode: backwards` (never `both`), under 300 ms: side pane 200, consent bar 300, drop-to-send frame 150,
reply/edit banner 125, copy tick 150, thread window 200 (from its bottom edge), poll bars `scaleX` 250 on vote, rare "beats" 300. Jump-to-message highlight starts 2x as strong and settles in 300 ms.
Only `transform` and `opacity` are animated. What is used all day (message list, room switch, hover) is **instant**. Tooltips and snackbars have **no fade** (timeout 0).
`prefers-reduced-motion: reduce` (and the "reduce motion" setting) forces every animation/transition to 0.01 ms, iteration count 1, no smooth scroll (RGAA 13.8).
Ripple: the theme disables nothing explicitly; under reduced motion it is cut by the global rule above.

### 1.6 Icons

- Library: **`@linagora/twake-icons` 3.0.0**: 424 SVG icons / illustrations as React (SVGr) components (427 exports, one helper `getFileTypeIcon`). Single-colour glyphs use `fill="currentColor"` (no fill attribute in the file, colour set by the host); illustrations keep their own colours. Each has its own `viewBox` (e.g. `Check` = `0 0 17 13`), so scale by *size* with preserved aspect.
- `<Icon icon={Check} size={16}/>` renders `<svg viewBox class="twake-icon" width=16 height=16>`; `.twake-icon { overflow: visible }`.
- **How to get SVGs**: `tokens.json` contains `icons` = `{ "<Name>": "<svg ...>" }` for all 424, generated by rendering each component with `react-dom/server` (see `export-tokens.mjs`). Embed with `include_str!` / generate an `Icon` enum + `asset_path`, register through GPUI's `svg()` element with `text_color`.
- Sizes in use (count of `size={n}` in `src/`): 16 (57x), 20 (13), 14 (12), 32 (9), 24 (9), 18 (9), 48, 112, 96, 72. Rule of thumb: inline status icons 14-16, toolbar/row actions 20-24, empty-state illustrations 72-112. Avatar glyph sizes 10 / 16 / 20 / 28 / 36 for avatar sizes xs..xl.
- Local Material icons not in twake-icons: `src/ds/icons.tsx` (`icon(path, viewBox='0 0 24 24')` factory; Videocam, AddCircle, Done, DoneAll, ChevronLeft/Right, ExpandMore, Mic, MicOff, CallEnd, CallHandset, ScreenShare, HandRaised, Poll, MoreVert, ... ~40, each a single `d` path, Material 24 px grid). Copy their `d` strings from that file (not exported in `tokens.json`).
- Icon-only buttons draw the glyph in `text.secondary` (`color: default`); `action.active` for pinned/muted markers.

### 1.7 Focus, density, scale

- Focus: `focusVisible: true` in the theme: a **2 px ring in `primary.main`** on every keyboard-reached control (WCAG 2.4.7 / 1.4.11). Message bubble: `outline 2px primary.main` on `.Mui-focusVisible`. Selected list items use `action.selected`.
- Density (D261, `ds/Density`): `compact` shrinks inner paddings from tablet width up (a phone always keeps finger sizes). Room row: min height 48 -> 36, avatar 48 -> 32 (child rooms 32 -> 24), filter chip 32 -> 28 px high, status icons 14 -> 16/20 depending.
- Text size: 0.9 / 1 / 1.15 multiplier (see 1.2). Theme: light / dark / system. Layout: compact or airy. Bubble alignment, 12/24 h clock, wallpaper, shades of grey are personalisation switches.

---

## 2. Accessibility rules the Rust client must keep

From `AGENTS.md` ("Accessibility"), `docs/accessibility.md`, the axe suite:

- **Targets >= 24 x 24 px** (WCAG 2.2 AA, enforced by axe). Real controls are larger: `IconButton` default size `medium` = 24 px icon + 12 px padding = 48 px; `small` = 8 px padding, `xsmall` = 5 px, `large` = 16 px. Reaction chips 28 px high, chips 28 / 32, list rows >= 48 (40 small, 56 large).
- **Every icon-only button has an accessible name AND a tooltip** that shows on hover and on keyboard focus, stays while the pointer is over it, and closes on Escape (WCAG 1.4.13): that is `ds/IconAction`; never a native `title`. Tooltip: top placement, **1000 ms enter delay**, 13 px (0.8125 rem) text, padding 4 x 8, radius 4, background `rgba(78,80,82,.9)`, arrow, **no fade**.
- Colours from the theme only; in `ds` read `theme.vars` so dark scheme follows. Contrast of colour is NOT gated (D245), but text-field edges are at 3:1 (`border.main`).
- Lists contain list items only; a new view focuses its title (`PaneHeader focusKey`); the conversation is one tab stop with arrows / Home / End between messages; the actions of a message are one stop, arrows inside. F6 / Shift+F6 cycle landmarks (sections rail, conversations list, conversation, details). First tab stop on the framed screen is a "skip to messages" link.
- A button that waits keeps focus (`aria-disabled`, not `disabled`). A button without an action is shown disabled, never hidden. A save button is never disabled for a missing field: it stays enabled, shows `helperText` ("Give it a name.") and focuses the field.
- Toggles expose state (`aria-pressed`; menu choices `menuitemradio`); `aria-current="page"` marks current room/tab. Errors use `role="alert"`, linked to their field; "is typing", "Link copied", counts of search results are polite status regions (`ds/LiveStatus`); the timeline is `role="log"` and silent, new messages from others are announced in a hidden status.
- Initials in avatars are hidden from screen readers. A message is read with author, text, time, read state, replied-to.
- Reduce motion respected; text scale respected; high-contrast mode (1.1).
- Message states: read ticks (`ds/ReadTicks`): clock (sending), one tick (server), double tick `primary.main` when read; 16 px icons.

---

## 3. What twake-mui restyles (MUI v9)

twake-mui 19.3.0 = MUI `createTheme` with `cssVariables: { colorSchemeSelector: 'data-theme', cssVarPrefix: 'twake' }`, palette / shadows / typography / shape / breakpoints / zIndex from section 1, and a `components` override map for **50 MUI components**:

`MuiAccordion`, `MuiAccordionDetails`, `MuiAccordionSummary`, `MuiAlert`, `MuiAlertTitle`, `MuiAppBar`, `MuiAutocomplete`, `MuiAvatar`, `MuiAvatarGroup`, `MuiBackdrop`, `MuiBadge`, `MuiButton`, `MuiCheckbox`, `MuiChip`, `MuiCssBaseline`, `MuiDialog`, `MuiDialogActions`, `MuiDialogContent`, `MuiDialogTitle`, `MuiDivider`, `MuiFab`, `MuiFormHelperText`, `MuiFormLabel`, `MuiIconButton`, `MuiInputLabel`, `MuiLink`, `MuiListItem`, `MuiListItemButton`, `MuiListItemIcon`, `MuiListItemSecondaryAction`, `MuiListItemText`, `MuiListSubheader`, `MuiMenu`, `MuiMenuItem`, `MuiOutlinedInput`, `MuiRadio`, `MuiSelect`, `MuiSnackbar`, `MuiSnackbarContent`, `MuiSwitch`, `MuiTab`, `MuiTableCell`, `MuiTableHead`, `MuiTableRow`, `MuiTableSortLabel`, `MuiTabs`, `MuiToggleButton`, `MuiToggleButtonGroup`, `MuiTooltip`, `MuiTypography`

Plus 70+ own components exported from the package (ActionsMenu, BottomSheet, Dialogs/*, NavigationList, Sidebar, SearchBar, SpaceOverlay, NestedSelect, VirtualizedTable, ...). The notable restyles (`tokens.json -> components` has all of them with callbacks resolved against the light theme):

| MUI component | Restyle |
|---|---|
| **Button** | default `variant: contained`, `disableElevation`. Root: radius **100px** (pill), min-height **40**, padding `10px 24px`; outlined `5px 15px`, border `currentColor`; text `10px 8px`, min-width auto. Extra `ghost` variant: bg 25 % of main (hover 32 %), text colour `text.primary`; per colour `secondary/success/...` ghost uses `action.ghostOpacity` 0.08 / hover 0.16. Disabled ghost: transparent, 1 px `action.disabledBackground` border. (The app sets `defaultProps.variant: 'text'`: a bare button is a text button; primary actions say `contained`.) |
| **IconButton** | twake default `large`; the app forces `medium`. Padding: xsmall 5 / small 8 / medium 12 / large 16. Colour `default` = `text.secondary` (scheme-aware). `.dialogIconButton` is absolutely positioned (top 1.15rem, right/left 1.15rem; 0.25rem under 769 px), paper bg. |
| **OutlinedInput** | radius **4**; padding `12.5px 16px` (multiline 16.5); border `border.main` (16 % ink), hover `text.disabled`, focused width 1 (ring from focusVisible), error border `error.main` (32 % at rest, full on hover/focus), disabled bg `background.contrast`; border-colour transition 200 ms standard. |
| **Switch** | track 56 x 40 (padding 6 x 1), thumb 20 px circle white with `shadows[1]`, travel 15 px, checked track = `primary.main` (`secondary` colour = `success.main`), disabled track `action.disabledBackground`, disabled thumb `background.default`. |
| **Chip** | MUI default shape; `.square` radius 8. The app draws filter chips 28/32 high, 12 px / 500, label padding 12, pill (100). |
| **Tooltip** | arrow, bg `rgba(78,80,82,.9)`, radius 4; twake: 16 px text, `8px 12px`; app: 13 px, `4px 8px`, top, 1 s delay, no fade. |
| **Dialog** | paper widths: small 480, medium 544, large 800, full 100 %; on < 1024 px small dialogs get 16 margin, 6 radius, 8 padding; backdrop 12 % / 50 %. Title 24/600 (22 under 769), padding `1.5rem 2rem`; actions margin `16px 32px` (`8px 16px` on phones, buttons flex-grow); `columnLayout` stacks buttons reversed, full width, 8 gap. |
| **Menu / MenuItem** | menu paper max-width 320; item gap 16, vertical padding 4, min-height 40 from 544 px up; icon colour `text.icon`. |
| **Avatar** | sizes xs 16 / s 24 / m 32 / l 48 / xl 64 with font 8 / 11 / 16 / 24 / 32 px and glyph 10 / 16 / 20 / 28 / 36; weight 600; `colorDefault` bg `background.paper`, fg `text.secondary`; `.border` 2 px paper ring; `.innerBorder` inset 1 px `border.main`. Colours come from `nameToColor` / `supportedColors` (hash of the name; also used by `ds/NameAvatar`). |
| **Tabs / Tab** | Tab 14/600/0.25, min-width 72 (160 from 544 px), padding `6px 12px`. `segmented` Tabs: pill (99) on `background.contrast`, min-height 40, indicator = paper pill with `shadows[1]`, selected text `text.primary`. |
| **ListItemButton** | gap 16; min-height 48 (small 40, large 56), padding 8/4/12 vertical; a second variant set (selected by a prop the export shows as `true`, probably secondary text) is 56 (48 / 64); selected / focus-visible bg `action.selected`; disabled opacity 0.5; `doubleGutters` 32 px sides. |
| **ListSubheader** | 14/600, bg `background.contrast` when sticky else `background.default`, padding 8 vertical. |
| **Badge** | height 18, min-width 18, radius 100, 2 px paper border, 10 px tabular digits; dot 12; offsets translate +-37 %. |
| **Fab** | 56 (medium, radius 16) / 40 (small, 12) / 96 (large, 28); paper bg, hover `color-mix(paper, #000 5%)`; `primary` = `primary.light` bg + `primary.dark` text (white in dark). Extended: radius 16, height 56 / 48 / 42. |
| **AppBar** | bg Grey A400 `#2C3039` in light, `primary.main` in dark, white text (`--AppBar-background`). |
| **Alert** | 6 severities (primary, secondary, success, error, warning, info) x standard / outlined / filled: standard bg = `severity.main` at 12 % (24 % dark), text `text.primary`; outlined 1 px border; filled `severity.main` + contrastText. Icons from twake-icons (CheckCircle, Info, Warning, WarningCircle). |
| **Checkbox / Radio** | padding 8 (small 6.4); icon box 18 + 3 padding; twake-icons CheckSquare / RadioChecked / RadioUnchecked. |
| **Accordion** | `shadows[1]`, 1 px `border.main`, radius 6, bottom margin 16. |
| **Divider** | text variant with 8 px wrapper padding, 14/500; inset 64 (80 in dialogs). |
| **Snackbar** | bottom-centre, auto-hide 2000 ms, app removes fade. |
| **Typography** | maps body3 -> p; `body3` default colour `text.secondary`. |
| **ToggleButton** | selected text `text.primary`, bg transparent, icon `primary.main`; padding `9px 24px`. |
| **CssBaseline (app)** | thin scrollbars, `overscroll-behavior: none`, reduced-motion rule, `.capitalised-name`, high-contrast block. |

Other overrides present: `MuiBackdrop`, `MuiDialogTitle/Content/Actions`, `MuiInputLabel`, `MuiFormLabel`, `MuiFormHelperText`, `MuiSelect`, `MuiAutocomplete`, `MuiLink`,
`MuiAlertTitle`, `MuiAccordionSummary/Details`, `MuiListItem`, `MuiListItemIcon`, `MuiListItemText`, `MuiListItemSecondaryAction`, `MuiTable*` (head, row, cell, sort label), `MuiSnackbarContent`, `MuiAvatarGroup`, `MuiToggleButtonGroup`: see `tokens.json`.

The app also layers (`src/app/theme.ts`): `MuiOutlinedInput` select shows the option icon beside the label (D155); `MuiIconButton` forced `medium` and `default` colour = `var(--twake-palette-text-secondary)`; `MuiButton` default variant `text`; `MuiSnackbar`, `MuiTooltip` as above; the `TextScale` wrapper; `SpaceOverlay` tweaks inside Twake Space (dialogs render on the overlay).

---

## 4. Proposed Rust model: a `twake-tokens` crate

Goal: generate Rust from `tokens.json` (build script or `xtask`), so the design system is data, not hand-copied constants. Suggested layout:

```
crates/twake-tokens/
  build.rs            # reads ../../docs/spec/tokens.json, emits $OUT_DIR/generated.rs
  src/lib.rs          # include!(concat!(env!("OUT_DIR"), "/generated.rs")) + hand-written types below
  assets/icons/*.svg  # written by build.rs from tokens.json["icons"] (or xtask)
```

```rust
// ---- colour ---------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rgba { pub r: u8, pub g: u8, pub b: u8, pub a: f32 }   // convert to gpui::Rgba / Hsla at the edge
impl Rgba { pub const fn hex(h: u32) -> Self; pub const fn with_alpha(self, a: f32) -> Self;
            pub fn over(self, bg: Rgba) -> Rgba; /* for color-mix 12% bubbles */ }

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Scheme { Light, Dark }                    // + HighContrast flag in Settings

pub struct Tone { pub light: Rgba, pub main: Rgba, pub dark: Rgba, pub contrast_text: Rgba }

pub struct Palette {
    pub primary: Tone, pub secondary: Tone, pub error: Tone,
    pub warning: Tone, pub success: Tone, pub info: Tone,
    pub grey: [Rgba; 10], pub grey_a: GreyAccent,   // 50..900, A100 A200 A400 A700 A900
    pub text: Text,          // primary, secondary, disabled, icon
    pub background: Background,  // default, paper, contrast
    pub border: Border,      // main, disabled, ghost, ghost_disabled + opacities
    pub divider: Rgba,
    pub action: Action,      // active, hover, selected, disabled, disabled_background, focus + ghost opacities
    pub common: Common,      // white, black
    pub ext: ComponentColours, // alert/avatar/button/chip/input/tooltip extras
}
pub static LIGHT: Palette = /* generated */;
pub static DARK: Palette  = /* generated */;
pub fn palette(s: Scheme) -> &'static Palette;

// ---- type -----------------------------------------------------------------
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextStyle { H1, H2, H3, H4, H5, H6, Subtitle1, Subtitle2, Body1, Body2, Body3, Button, Caption, Overline }
pub struct TypeSpec { pub size: f32, pub weight: u16, pub line_height: LineHeight /* Ratio(f32)|Px(f32) */, pub letter_spacing: f32 }
impl TextStyle { pub const fn spec(self) -> TypeSpec; }
pub const FONT_FAMILY: &[&str];                 // ["Inter", ".SystemUIFont" (macOS stand-in for -apple-system), ...]
pub struct TextScale(pub f32);                  // 0.9 | 1.0 | 1.15, multiplies size (and px-derived sizes)

// ---- geometry -------------------------------------------------------------
pub const SPACING: f32 = 8.0;  pub const fn space(n: f32) -> f32 { n * SPACING }
pub enum Radius { None, Sm, Md, Lg, Pill }  // 0, 4, 8, 12, 100
pub const SHAPE_RADIUS: f32 = 6.0;
pub enum Breakpoint { Xs = 0, Sm = 544, Md = 769, Lg = 1024, Xl = 1201 }
pub mod layout { pub const PANE_WIDTH: f32 = 360.0; pub const MIN_CONVERSATION_WIDTH: f32 = 360.0;
                 pub const PANE_GAPS: f32 = 24.0; pub const THREAD_WINDOW_WIDTH: f32 = 420.0; /* ... */ }
pub enum Layer { Fab = 1050, AppBar = 1100, Drawer = 1200, Modal = 1300, Snackbar = 1400, Tooltip = 1500 }

pub struct ShadowLayer { pub x: f32, pub y: f32, pub blur: f32, pub spread: f32, pub colour: Rgba }
pub fn elevation(scheme: Scheme, n: u8 /*0..=25*/) -> [ShadowLayer; 3]; // 0 => empty

// ---- motion ---------------------------------------------------------------
pub struct Easing(pub f32, pub f32, pub f32, pub f32);       // cubic-bezier
pub const EASE_OUT: Easing; pub const EASE_DRAWER: Easing; pub const EASE_IN_OUT: Easing;
pub mod duration { pub const SHORTEST: u32 = 150; /* ... */ pub const ARRIVE_PANE: u32 = 200; }
pub struct Motion { pub reduced: bool }  // reduced => all durations ~0, no transform animation

// ---- icons ----------------------------------------------------------------
pub enum IconName { Check, Folder, /* 424 generated */ }
impl IconName { pub const fn svg(self) -> &'static str; pub const fn view_box(self) -> (f32, f32, f32, f32); }
pub enum IconSize { Inline = 14, Small = 16, Medium = 20, Large = 24, Xl = 32, Hero = 112 }

// ---- component specs (hand-written, driven by the tokens) ------------------
pub struct AvatarSize; // Xs..Xl => (box, font, glyph) = (16,8,10) (24,11,16) (32,16,20) (48,24,28) (64,32,36)
pub struct ButtonSpec { pub min_height: f32, pub padding: (f32, f32), pub radius: Radius, pub variant: ButtonVariant /* Contained|Outlined|Text|Ghost */ }
pub struct Density { pub compact: bool }  // resolve row heights / avatar sizes (section 5 catalogue)
pub struct Settings { pub scheme: Scheme, pub high_contrast: bool, pub grayscale: bool, pub text_scale: TextScale, pub density: Density, pub reduced_motion: bool }
```

Design notes:

- Keep **semantic tokens** (`text.secondary`, `action.selected`, `background.paper`) as the only thing views read; never raw greys. A `Theme` object resolves `Settings` -> `&Palette` (+ high-contrast overrides from 1.1 as a third table) once, and is a GPUI `Global`.
- Colours with alpha such as `rgba(66,66,68,0.64)` are *not* opaque greys: keep `Rgba { a }` and composite over the surface in the renderer (GPUI `Hsla` has alpha). `color-mix(in srgb, primary 12%, paper)` = linear blend in sRGB space: implement `Rgba::mix`.
- CSS `calc(var(--x) / ...)` ghost fills = `Rgba::with_alpha(main, 0.25/0.32)`; `action.*Opacity` values are fractions to feed `with_alpha`.
- Line-height ratios (`1.313`) stay ratios; `overline` is the single px line height. Letter-spacing is px, not em.
- Fonts: ship Inter (variable, 400/500/600/700) with the app; fall back to the system UI font where the web falls back to `-apple-system`. Emoji: Apple Color Emoji.
- Generated file must be deterministic and checked in CI (`cargo xtask tokens --check`); re-run `node export-tokens.mjs > tokens.json` when twake-mui is bumped. A golden test per token group (a few hex / px values) catches drift.
- Gaps the generator cannot fill (needs hand work): per-component sizes inside `ds/*` (they live in JSX, not in the theme; see the catalogue), CSS state selectors, and anything responsive (`max-width: 768.95px` becomes a `Breakpoint` check on window width).

### 4.1 Exporting tokens: script

`docs/spec/export-tokens.mjs` (run, output saved in `tokens.json`):

```
node docs/spec/export-tokens.mjs [path/to/twake-space-chat] > docs/spec/tokens.json
```

It imports `makeTheme` / `radius` from the **installed** `@linagora/twake-mui` dist (registering a tiny Node loader because the dist uses extensionless relative imports and a JSON import without attributes), builds the light and dark themes, and writes: `paletteRaw`, `schemes.{light,dark}.{palette,shadows}`, `typography` (14 variants) + `typographyBase`, `shape`, `radius`, `spacing`, `breakpoints`, `zIndex`, `transitions`, `components` (50 override entries, style callbacks evaluated against the light theme; React elements become `"<ReactElement>"`, unresolvable callbacks `"<fn>"`; colours inside are `var(--twake-palette-…, fallback)`, i.e. token names with their light value), and `icons` (424 static SVG strings). Not exported by the script because they live in app code, not in the theme: `ACCESSIBLE_THEME`, `motion.ts`, `layout.ts` (all reproduced by hand above).

---

## 5. Component catalogue (`src/ds`, 115 folders)

Generated from the source (`src/ds/<Name>/*.tsx`): the leading comment of each file, exported symbols, the `*Props` type, and the values found literally in the code. **Reading notes:** "Dimensions" lists numbers written next to size-like props in `sx`, **as written**: spacing props (`gap`, `p`, `px`, `py`, `m`...) are x 8 px, `borderRadius` numbers are x 6 px, `width/height/size/min*/max*` and string values are px. Figma column: `mapped` (25) = drawn in Library Twake, `todo` (16) = drawn but no own Figma component, `no-figma` (74) = Chat-only. "Candidate for twake-ui" means the component is meant to move into twake-mui. "State hooks" shows which state selectors / ARIA states the component styles (hover, `focus-visible`, `Mui-disabled`, `aria-*`).

Key components with their exact numbers (hand-verified against the source):

| Component | Anatomy and dimensions |
|---|---|
| **MessageBubble** | avatar 32 (at the foot of a group's last bubble, gap 4), hover toolbar buttons 32 each, bubble radius **16**, inner pad `px 1.5 (12), py 0.5 (4)`; max width `min(75%, 520px)` of the row; own = right-aligned, bg 12 % primary over paper; others paper; time + ticks inside bubble bottom-right, reactions bottom-left same line; pinned shows `Pin` icon 14 before time; selected message: 20 px round check (14 px `Check`), dark bar on top (D183); warning tint `warning.main` 12 % for flagged; hover bg `text.primary` 6 %; "jumbo" emoji messages have no bubble. Hover: quick reactions over the top right, Reply, "..." (D183, D489). |
| **RoomRow** | min height 48 (36 dense / child), avatar **48** top-level / 32 child (dense 32 / 24), gap 8, title `body2` 14 baseline-aligned with time `caption`; unread badge height 16, min-width 16, radius 8, 10 px (primary = unread, grey `action.disabledBackground` = muted; mention variant); dot 8 px; status icons 14/16/20 (key = encrypted, pin); selected `action.selected`, selected+hover `action.focus`; corner kind badge 14-20 circle `background.paper` at (-2,-2). |
| **BottomNav** | pill indicator **44 x 28**, radius 14, icon 20, label 11/16 `caption`, item radius 12; selected = tinted pill + `primary`, others `text.secondary`, disabled `text.disabled`; count badge at (top -6, left 32). |
| **FilterChips** | chip height 32 (28 dense), gap 6, row padding 16, pill 100, 12 px / 500, label px 12, max width 240, selected = filled `primary.dark` text on primary tint (dark: `text.primary`); dot 8 px; overflow arrows 28 px round buttons with 16 px chevrons. |
| **ReactionChips** | height 28, radius 16, 14 px, outlined on paper (own = filled primary), gap 4; tooltip "emoji Alice, Bob +2". |
| **PresenceBadge** | dot (size derived from avatar), 2 px paper ring, bottom-right. |
| **ReadTicks** | 16 px; read = `primary.main`, else `text.secondary`. |
| **IconAction** | `IconButton` + Tooltip (rules in section 2); optional `badge` count at (top -2, left 18). |
| **AppBar** | min height **56**, logo height 28, gap 8; right: help, apps, account avatar. |
| **PillButton** | primary button at the foot of a pane: contained, min-width 160 (flex-1 on phones when paired with a Cancel outlined), padding 16, gap 8, right-aligned. |
| **PaneHeader / DialogHeader** | title h4 (22 / 600) for Chats, Contacts, Activity, Settings (D205); focus on the title on view change. |
| **Composer** | attachment, text, emoji (with GIF tab), send; Scribe chip 28 high, 12 px / 500, icon 16, spinner 14 (D626); toolbar row 32; touches the bottom edge. |

#### ActivityItem

Local design system: the body of a line of the Activity (D179), as the « activity » mockup of Twake Matrix: avatar, who and what they did, time, room, and the message quoted under them behind a thin line. Candidate for twake-ui.

- Exports: ActivityDay, ActivityItem  |  Figma: todo
- `ActivityItemProps`: avatar: ReactNode; actor?: string | null; action: ReactNode; time: string; room: string; quote?: string | null
- Typography: body2, caption
- Dimensions (px, as written): gap=1; minWidth=0; px=0.5

#### AddRowContent

Local design system: the inside of an « Add … » row, as the « Contact list » of Library Twake (18880:11800): a 48 px light-blue avatar with the person-add icon, a 14 px medium label. Candidate for twake-ui.

- Exports: AddRowContent  |  Figma: todo
- Dimensions (px, as written): size=18

#### AlbumGrid

Local design system: images sent together, as a grid of square tiles in one bubble (D89). Two columns; the last of an odd count takes both. Each tile holds its image, a menu shown on hover and keyboard focus, and its reactions over the foot of the image, so that every row keeps its height. Candidate for twake-ui.

- Exports: AlbumGrid, AlbumTile  |  Figma: no-figma
- Dimensions (px, as written): borderRadius=2; bottom=4; left=4; m=0.5; px=0.5; right=4
- State hooks: &:hover, focus-visible

#### AppBar

Local design system: the app bar of Twake Chat, full width above the panes: the logo on the left; help, the Twake apps and the account on the right. A button without an action is shown disabled, never hidden. The banner landmark of the page, as the bar of the platform (`@linagora/twake-bar`, D64) is when it takes its place. twake-mui has no app bar of its own, nor an apps grid or a logo (docs/twake-mui-gaps.md): the logo is `public/twake-chat-logo.svg`.

- Exports: AppBar  |  Figma: mapped → 🔷 app bar
- `AppBarProps`: labels: { help: string; apps: string; account: string }; onHelp?: (event: MouseEvent<HTMLElement>) => void; onApps?: (event: MouseEvent<HTMLElement>) => void; avatar: ReactNode; onAccount?: (event: MouseEvent<HTMLElement>) => void; isAccountOpen?: boolean
- Dimensions (px, as written): gap=1; height=28; minHeight=56; minWidth=0; padding=0.5
- State hooks: disabled

#### Arriving

Local design system: what it holds arrives instead of popping in (see `motion.ts`): once, when it is drawn. Give it a new `key` to play it again for a new content. Candidate for twake-ui.

- Exports: Arriving  |  Figma: no-figma

#### AssistantAvatar

Local design system: the avatar of the assistant of the user, its outlined icon in the primary colour on a light gradient of it. Candidate for twake-ui.

- Exports: AssistantAvatar  |  Figma: no-figma

#### AssistantMark

Local design system: the mark of an assistant brought into a conversation, beside its name. Candidate for twake-ui.

- Exports: AssistantMark  |  Figma: no-figma
- Dimensions (px, as written): size=16

#### AssistantQuestionCard

Local design system: the card that floats over the app with a yes/no question of the assistant (a shortcut to its room): avatar, a line of context, a bold title, the text, and the buttons. A region, not a status: it holds actions. Candidate for twake-ui.

- Exports: AssistantQuestionCard, AssistantQuestionStack  |  Figma: no-figma
- Typography: body2, caption, subtitle2
- Palette (theme.vars): background.paper, text.primary, text.secondary
- Dimensions (px, as written): borderRadius=1,2; gap=1,1.5; minWidth=0; p=2; px=1; right=16; size=32; top=16
- State hooks: disabled

#### AudioPlayer

Local design system: an audio file of the timeline (not a voice message): play or pause, a slider to seek, the time, the speed and the download. Candidate for twake-ui.

- Exports: AudioPlayer  |  Figma: no-figma
- `AudioPlayerProps`: src: string | null; duration: number; playLabel: string; pauseLabel: string; positionLabel: string; speedLabel: string; downloadLabel: string; formatTime: (milliseconds: number) => string; onPlay?: () => void; onDownload: () => void
- Typography: caption
- Dimensions (px, as written): gap=1; minWidth=120
- State hooks: disabled

#### AvatarStack

Local design system: avatars side by side, each overlapping the last.

- Exports: AvatarStack  |  Figma: no-figma

#### BannerStack

Local design system: the notices under the header of a conversation, in one bar (D356): the most urgent shows, « 1/3 » and ‹ › page through the others, each keeps its own actions and close « x ». The neutral grey of the « banner » of Library Twake (node 435:274), one height for all. Candidate for twake-ui.

- Exports: BannerStack  |  Figma: mapped → 🔷 banner
- Dimensions (px, as written): gap=1; minWidth=0; size=16
- State hooks: disabled

#### BottomNav

Local design system: the bar at the bottom of the left column of the Twake Chat frame, under the list, as the « bottom nav » of Library Twake: the icon above its label, the current item on a grey pill with its filled icon, in the grey of the others (D615), a blue badge for what waits. An item without an action is shown disabled, never hidden. The landmark around it belongs to ChatShell. Candidate for twake-ui.

- Exports: BottomNav  |  Figma: mapped → 🔷 bottom navigation
- Typography: caption
- Dimensions (px, as written): borderRadius=2; gap=0.5; height=20,28; left=32; minWidth=0; padding=0; top=-6; width=20,44
- State hooks: &:hover, Mui-disabled, aria-current, disabled, selected

#### CallInvite

Local design system: an incoming call, centred over the app as a phone shows it: who calls, a pulsing ring around them, decline in red and answer in green. Candidate for twake-ui.

- Exports: CallInvite  |  Figma: no-figma
- Typography: body2, h4
- Dimensions (px, as written): gap=1,1.5,6; height=28,30,64; px=4; py=5; width=28,30,64
- State hooks: &:hover, disabled

#### CallLayout

Local design system: a conversation during a call. On a large screen the call takes the width and the conversation a column on its right; on a phone, the call above the conversation, or the call alone in a small frame. Without a call, nothing changes. Candidate for twake-ui.

- Exports: CallCover, CallLayout  |  Figma: no-figma
- Dimensions (px, as written): minHeight=0; minWidth=0

#### CallStage

Local design system: what the call server says of a call, above its grid: the recording badge, the live captions, an error of the call service.

- Exports: CallBanner, CallStage  |  Figma: todo
- Typography: body2, caption
- Dimensions (px, as written): borderRadius=1,2; bottom=6; gap=0.25,0.5,1,1.5; height=0,24,48; left=8; m=0; maxWidth=480; minHeight=0; minWidth=0; p=0,0.5,1.5; px=0.75,1; py=0.25; right=8; size=16; top=6; width=0,24,48
- State hooks: &:hover, focus-visible, Mui-disabled, aria-pressed, disabled

#### CallWindow

Local design system: the window a call floats in (D571). Dragged by its bar and resized by its edges, freely, up to the edges of the screen; its one button opens a menu that does the same without dragging (WCAG 2.5.7). Candidate for twake-ui.

- Exports: CallBand, CallWindow  |  Figma: no-figma
- Typography: subtitle2
- Dimensions (px, as written): borderRadius=0.5,3; bottom=0; gap=1; height=18; left=0; minHeight=0; minWidth=0; py=0.5; right=0; top=0; width=24

#### CameraScanner

Local design system: the camera, read for a QR code a few times a second (`readQrCode`). Candidate for twake-ui.

- Exports: CameraScanner  |  Figma: no-figma
- Palette (theme.vars): action.hover
- Dimensions (px, as written): borderRadius=1

#### CappedList

Local design system: a list that shows its first rows, with a button that shows more (the categories of the room list). Candidate for twake-ui.

- Exports: CappedList  |  Figma: no-figma
- `CappedListProps`: rows: readonly ReactNode[]; keepRows?: number; labels: { more: string; less: string }
- Dimensions (px, as written): mx=1

#### CenteredState

Local design system: a message centred in its area (sign-in, loading, nothing selected). Candidate for twake-ui (an `Empty` with actions).

- Exports: CenteredState  |  Figma: mapped → 🔹 empty
- `CenteredStateProps`: title?: string; text?: string; visual?: ReactNode; image?: string; children?: ReactNode; isPage?: boolean; 'data-testid'?: string
- Typography: body2, h3
- Dimensions (px, as written): gap=1.5; height=116; maxWidth=420; p=3; width=116
- State hooks: selected

#### ChatShell

Local design system: the frame of the chat screen: the app bar above, then the list with the bottom bar under it, the conversation and the side pane, each a card of its own on the grey of the frame, as Discord sets its windows apart. On a phone the panes stack: one shows at a time. F6 moves between them; a skip link, first on the page, leads to the conversation past the list. Inside Twake Space (`data-embedded` on the root, set once by the page that starts the app) the page is the paper of the theme alone, no card: Space draws the frame (ADR 010 of Twake Space). Candidate for twake-ui.

- Exports: CALL_CONVERSATION_WIDTH, ChatShell, MIN_CONVERSATION_WIDTH, PANE_GAPS, PANE_WIDTH, THREAD_WINDOW_BOTTOM_OFFSET, THREAD_WINDOW_FOLDED_WIDTH, THREAD_WINDOW_WIDTH  |  Figma: no-figma
- `ChatShellProps`: appBar?: ReactNode; banner?: ReactNode; bottomBar?: ReactNode; start?: ReactNode; main: ReactNode; end?: ReactNode; narrowPane?: ShellPane; isStartFolded?: boolean; onEndClose?: () => void; labels: {; start: string; main: string; end: string; bottomBar?: string; skip?: string; }
- Dimensions (px, as written): bottom=0; gap=0,0.5; left=8; minHeight=0; minWidth=0; px=2; py=1; right=0; top=0,8
- State hooks: focus-visible

#### CheckCircle

Local design system: the tick of a row that is picked in a list, drawn only (the row carries the checkbox role): round, or a square box before the row, empty until picked (Flutter « Add people »). Candidate for twake-ui.

- Exports: CheckCircle  |  Figma: mapped → 🔷 checkbox
- Dimensions (px, as written): height=20; size=14; width=20

#### ColorDots

Local design system: one colour picked among the Drive palette (`COLORS` of twake-mui), as Telegram picks the colour of a folder. The `ColorList` of twake-mui draws the same dots but gives them no name (axe: a button without a name) and no checked state: this one is a radio group. Candidate for twake-ui.

- Exports: ColorDots  |  Figma: todo
- Palette (theme.vars): divider
- Dimensions (px, as written): gap=1.5; p=0.5

#### Composer

The « + » of the composer: alone it picks a file of the computer; with Drive or a poll beside it, it opens a menu of them.

- Exports: AttachMenu, AttachmentStrip, Composer, FORMATS, FORMAT_ICONS, FormatToolbar, IS_MAC, LinkDialog, MarkdownField, RecordingBar, RoundButton, ScribeBar, ScribeIcon, ScribeOptions, SuggestionList  |  Figma: no-figma
- `ComposerProps`: plain?: boolean; allowEmpty?: boolean; autoFocus?: boolean; placeholder: string; attachLabel: string; computerLabel?: string; sendLabel: string; disabled?: boolean; context?: ComposerContext | null; initialText?: string; initialPicked?: readonly ComposerSuggestion[]; onPickedChange?: (picked: readonly ComposerSuggestion[]) => void; onSend: (text: string, picked: readonly ComposerSuggestion[]) => void; triggers?: readonly ComposerTrigger[]; insertion?: { id: number; suggestion: ComposerSuggestion } | null; focusKey?: string; focusRef?: RefObject<(() => void) | null>; onTextChange?: (text: string) => void; onEditLast?: () => boolean; onAttach?: (file: File) => void; attachments?: ComposerAttac
- `ScribeProps`: code, shift, alt
- `ScribeBarProps`: options: readonly ScribeOption[]; label: string; optionsLabel: string; labelFor: (name: string) => string; iconFor: (name: string) => ReactNode; isOpen: boolean; onToggle: () => void; disabled: boolean; isBusy: boolean; onPick: (prompt: string) => void; notice?: string
- Typography: caption
- Palette (theme.vars): divider, primary.main, text.primary, warning.main
- Dimensions (px, as written): borderRadius=0.5,1.5,3; bottom=0; fontSize=0; gap=0.25,0.5,1,1.5; height=20,28,32; left=0; m=1; minHeight=24,40; minWidth=0,24,280; mx=0.5; p=0,1; padding=8; px=0.5,1,2; py=0.5,0.75,1.5; right=0,2,4; size=14,16,20,24; top=0,2,4; width=8,20,32
- State hooks: &:hover, focus-visible, Mui-disabled, aria-selected, disabled, selected

#### ConsentBar

Local design system: a question that waits for an answer at the bottom of the screen without hiding it (consent to analytics). Candidate for twake-ui.

- Exports: ConsentBar  |  Figma: mapped → 🔷 banner
- Typography: body2, subtitle1
- Dimensions (px, as written): borderRadius=0; bottom=0; gap=1,2; maxWidth=960; minWidth=0; px=2

#### CornerBadge

Local design system: a tinted round button over the bottom right corner of a picture (the pen of an avatar), the small primary Fab of twake-mui: a bare icon button there is a blue pen on the photo, which does not show. Candidate for twake-ui.

- Exports: CornerBadge  |  Figma: no-figma
- Dimensions (px, as written): bottom=-8; right=-8

#### Density

Local design system: the display density (D261). One context read by the ds components: with `compact` every inner padding shrinks, from tablet width up (a phone keeps the size of a finger). Dialogs keep their spacing: none holds a component that reads it; one that does wraps its content in `<DensityProvider compact={false}>`.

- Exports: DensityProvider  |  Figma: no-figma

#### DetailPage

Local design system: a page that takes the place of the conversation (the info of a room, as in the Flutter Twake Chat): its header stays, its body scrolls. Candidate for twake-ui.

- Exports: DetailHero, DetailPage, DetailSurface, KeptPage  |  Figma: no-figma
- Dimensions (px, as written): gap=2; minHeight=0; top=0

#### DialogHeader

Local design system: the header of a dialog of the Figma mockups (« New poll »): a round avatar holding its icon, a title with a line under it, the close button at the end. Candidate for twake-ui.

- Exports: DialogHeader  |  Figma: todo
- Typography: caption, subtitle2
- Dimensions (px, as written): gap=1; height=32; minWidth=0; px=3; py=1; width=32

#### DialogTitleBar

Local design system: the row of a dialog title and its action. twake-mui leaves 24 px under the title and 24 px above the content; the bar keeps one 16 px gap between them.

- Exports: DialogTitleBar  |  Figma: no-figma
- Dimensions (px, as written): px=1

#### Divider

Local design system: the lines between the messages. « Today », « Monday 5 October » between two days; the « New messages » line above the first message that was unread when the room opened, as the Flutter Twake Chat draws it: a rule, then the words in the middle. Candidate for twake-ui.

- Exports: DayDivider, NewMessagesDivider  |  Figma: no-figma
- Dimensions (px, as written): px=2; py=1

#### DropZone

Local design system: a pane that takes files dropped on it, with a veil saying so while they are dragged over. Candidate for twake-ui.

- Exports: DropZone  |  Figma: no-figma
- Typography: h6
- Dimensions (px, as written): borderRadius=4; minHeight=0; minWidth=0

#### EmojiComparison

Local design system: the emojis two devices show to be compared during a verification, large, each with its name. Candidate for twake-ui.

- Exports: EmojiComparison  |  Figma: no-figma
- Typography: caption
- Dimensions (px, as written): fontSize=40; gap=1.5; m=0; p=0

#### EmojiPicker

Local design system: a grid of emojis by category, for reactions and the composer, with a search field. One tab stop for the grid; the arrows move inside it (↓ from the search field enters it), Enter picks. Candidate for twake-ui.

- Exports: EmojiPicker, PICKER_SIZE, PickerPlaceholder  |  Figma: todo
- Typography: caption
- Dimensions (px, as written): borderRadius=2; fontSize=22; height=36; minHeight=72; p=1,2; px=0.5; width=36,352
- State hooks: &:hover

#### EmptyConversation

Local design system: a conversation without any message yet, as the Twake Matrix Figma draws it (« Congratulations on your new chat! », node 43856:44718): a translucent card, the mascot, a title, a hint, what the conversation offers ticked. The card itself is no button (D397). Candidate for twake-ui.

- Exports: EmptyConversation  |  Figma: no-figma
- Typography: body2, caption, subtitle1
- Palette (theme.vars): background.paper
- Dimensions (px, as written): gap=1,1.5; height=126; m=0; maxWidth=320; p=0,3; width=126,268
- State hooks: selected

#### EmptyState

Local design system: an empty area as Twake Space draws it (Figma « empty states »): an illustration, a title, an optional action, centred over the whole area in the shared empty card. Candidate for twake-ui.

- Exports: EmptyState  |  Figma: mapped → 🔹 empty
- Typography: subtitle1
- Dimensions (px, as written): height=126; p=3; width=126

#### FabDock

Local design system: holds a floating button at the bottom right of a scrolling panel. Candidate for twake-ui.

- Exports: FabDock  |  Figma: no-figma
- Dimensions (px, as written): bottom=16

#### FileChip

Local design system: a file of a message as a chip, spaced from the words above it, if any. Candidate for twake-ui.

- Exports: FileChip  |  Figma: todo
- `FileChipProps`: icon: ReactElement; label: string; action: string; isBelowText: boolean; onClick: () => void

#### FilterChips

Local design system: a row of quick filters, one chosen at a time (All, Unread, Favourites...), as WhatsApp shows them above the conversations. Candidate for twake-ui.

- Exports: FilterChips  |  Figma: mapped → 🔷 Chip filters
- Palette (theme.vars): background.paper
- Dimensions (px, as written): height=8,28; maxWidth=240; px=1.5,2; size=16; width=8,28
- State hooks: &:hover, aria-pressed

#### FloatingWindow

Local design system: windows over the page, bottom right, apart from the panes (threads), side by side as the drafts of Gmail: each folds down to its header, or closes. On a phone, the whole screen. Not modal: the conversation behind stays usable. Candidate for twake-ui.

- Exports: FloatingWindow, FloatingWindowStack  |  Figma: no-figma
- Palette (theme.vars): background.paper
- Dimensions (px, as written): borderRadius=0,3; gap=0,1; left=0; minHeight=0; minWidth=0; right=0,16

#### FormColumn

Local design system: the fields of a form in a page of the main view, at most 680 px wide and centred, its button at the right edge of that column (Swann, D361). The page around it keeps the whole width (D340): headers and lists are not in it.

- Exports: FormColumn  |  Figma: no-figma
- Dimensions (px, as written): maxWidth=680

#### GifPicker

Local design system: the GIFs tab of the picker of the composer (D63, D276): a search field and a grid of previews, each one a button, in the 352 × 552 panel of the emoji picker. Candidate for twake-ui.

- Exports: GifPicker  |  Figma: no-figma
- `GifPickerProps`: label: string; searchPlaceholder: string; query: string; onQueryChange: (query: string) => void; gifs: readonly GifPickerItem[]; isLoading: boolean; message: string | null; loadMoreLabel: string; onLoadMore: (() => void) | null; onPick: (id: string) => void; footer?: ReactNode
- Typography: body2
- Dimensions (px, as written): gap=1; minHeight=72; p=1
- State hooks: disabled

#### GroupLabel

Local design system: the small grey title of a group of rows in a panel (« Administration », « Danger zone »). Candidate for twake-ui.

- Exports: GroupLabel  |  Figma: no-figma
- Typography: caption
- Dimensions (px, as written): px=2

#### HeroPhoto

Local design system: the picture of a person or a room across the top of its info, its name on it (Telegram mockup), and the round button that shows it so. Candidate for twake-ui.

- Exports: HeroPhoto, PhotoButton  |  Figma: no-figma
- Dimensions (px, as written): bottom=0; px=2

#### Highlight

Local design system: a text with the letters typed in a search marked in bold, as in Slack (« Quen » in « Quentin »). Candidate for twake-ui.

- Exports: Highlight  |  Figma: no-figma
- Palette (theme.vars): primary.main, warning.main
- Dimensions (px, as written): borderRadius=0.5

#### HoverActionItem

Local design system: a list item whose action (an icon at its right) shows on its hover or keyboard focus only, as the menu of a room row: one per row would line the whole list with icons. Candidate for twake-ui.

- Exports: HoverActionItem  |  Figma: mapped → 🔹 ListItem
- `HoverActionItemProps`: action: ReactNode; children: ReactNode; 'data-testid'?: string
- Dimensions (px, as written): right=8; top=2
- State hooks: &:hover, focus-visible

#### HoverRow

Local design system: a row whose `Reveal` children show on hover and on focus inside it, always when there is no pointer to hover with. Candidate for twake-ui.

- Exports: HoverRow, Reveal, RowButton  |  Figma: no-figma
- Dimensions (px, as written): gap=2; minWidth=0
- State hooks: &:hover

#### IconAction

Local design system: a button drawn as an icon, named by a tooltip that shows on hover and on keyboard focus, stays while the pointer is over it and closes on Escape (WCAG 1.4.13), where the native `title` shows on hover only. Candidate for twake-ui.

- Exports: IconAction  |  Figma: mapped → 🔷 icon button
- `IconActionProps`: label: string; tooltip?: string; href?: string; target?: string; rel?: string; badge?: { count: number; label: string }
- Dimensions (px, as written): left=18; top=-2
- State hooks: disabled

#### ImagePreview

Local design system: an image inside a message, at its size shrunk to fit, with its place kept while it loads so that the timeline does not jump. Candidate for twake-ui.

- Exports: ImagePreview  |  Figma: no-figma
- `ImagePreviewProps`: src: string | null; alt: string; width: number; height: number; label: string; onClick: () => void; isFlush?: boolean; isTipBelow?: boolean
- Dimensions (px, as written): borderRadius=2

#### ImageViewer

Local design system: the image of a message shown large, over the screen, with the way to download it; a PDF too, in the viewer of the browser, and a video, played here (D587). Candidate for twake-ui.

- Exports: ImageViewer, ViewerMedia  |  Figma: no-figma
- `ImageViewerProps`: src: string | null | undefined; alt: string; isDocument?: boolean; video?: {; unplayableLabel: string | null; poster: string | null; compactLabel: string; onError: (video: HTMLVideoElement) => void; onCompact: (video: HTMLVideoElement) => void; }; title: string; downloadLabel: string; closeLabel: string; loadingLabel: string; onDownload: () => void; onClose: () => void; steps?: {; previousLabel: string; nextLabel: string; positionLabel: string; onPrevious: Step | null; onNext: Step | null; }
- Typography: body2, subtitle1
- Dimensions (px, as written): gap=1; minHeight=0; minWidth=0; p=1
- State hooks: disabled

#### InertPreview

Local design system: a framed picture of a part of the app, drawn with its real components but inert: nothing in it takes the focus, is read aloud or reacts to the pointer. Candidate for twake-ui.

- Exports: InertPreview  |  Figma: no-figma
- Dimensions (px, as written): borderRadius=2

#### InfoSkeleton

Local design system: the shimmer of the info of a room that loads (avatar, name, one line). Candidate for twake-ui.

- Exports: InfoSkeleton  |  Figma: no-figma
- Dimensions (px, as written): gap=1.5; height=28,96; p=3; width=96

#### InlineMarkdown

Local design system: the Markdown of a message on one line, drawn with its marks (bold, italic, struck, underlined, code) and without its signs: the preview of a room in the list. Candidate for twake-ui.

- Exports: InlineMarkdown  |  Figma: no-figma

#### IntentDialog

Local design system: the dialog that holds the picker of Twake Drive, an iframe another script puts in it (cozy-interapp, D36), or another intent (the composer of Twake Mail, D65). The service draws its own title, and its close button once ready. Candidate for twake-ui.

- Exports: IntentDialog  |  Figma: no-figma
- `IntentDialogProps`: open: boolean; title: string; error: string | null; onClose: () => void; onContainer: (element: HTMLDivElement | null) => void; name?: string; isClosedByService?: boolean; closeButton?: { label: string; isShown: boolean }
- Dimensions (px, as written): height=800; m=2; minHeight=0; right=8; top=8; width=900

#### InviteRow

Local design system: an invitation in the room list, at the measures of a room row (40 px avatar, 56 px high, same margins and type): its name, who invites, and « Join » / « Decline » under them. Candidate for twake-ui.

- Exports: InviteRow  |  Figma: mapped → 🔹 ListItem
- `InviteRowProps`: name: string; from: string | null; acceptLabel: string; declineLabel: string; onAccept: () => void; onDecline: () => void; 'data-testid'?: string
- Typography: caption, subtitle2
- Dimensions (px, as written): gap=1; minHeight=32,56; minWidth=0; mx=1; px=1,1.5; py=0.25,1

#### LinkPreviewCard

Local design system: what a web page is, shown under the message that links it: its picture, title, description and site. Candidate for twake-ui.

- Exports: LinkPreviewCard, THUMBNAIL  |  Figma: no-figma
- `LinkPreviewCardProps`: href: string; title: string | null; description: string | null; siteName: string | null; imageSrc: string | null; hasImage?: boolean; imageWidth?: number | null; imageHeight?: number | null
- Typography: caption, subtitle2
- Palette (theme.vars): background.paper
- Dimensions (px, as written): gap=0.25; minWidth=0

#### LinkRow

Local design system: a link shared in a room, as the Flutter Twake Chat lists them: its picture (or the first letter of its site), its title, its description, the link itself. Candidate for twake-ui.

- Exports: LinkList, LinkRow  |  Figma: mapped → File type list
- Typography: body1, body2
- Dimensions (px, as written): gap=2; m=0; minWidth=0; p=0; px=2

#### ListFooter

Local design system: the line under a list that is still filling, said politely to a screen reader: a spinner and its text. Candidate for twake-ui.

- Exports: ListFooter  |  Figma: no-figma
- Typography: body2
- Dimensions (px, as written): gap=1; p=2; size=16

#### ListRow

Local design system: the rows of the other tabs of the left column (Contacts, Activity) drawn as those of the room list (D191): 8 px around and between, 48 px high, a line between two rows. Candidate for twake-ui.

- Exports: ListRowButton, ListRows  |  Figma: mapped → 🔹 ListItem
- Dimensions (px, as written): gap=1; left=16; minWidth=0; mx=1; my=0; px=1; right=16; top=0
- State hooks: Mui-selected, selected

#### LiveAnnouncement

Local design system: a polite live region, visually hidden, that stays in the page so that a change of its text is announced.

- Exports: LiveAnnouncement  |  Figma: no-figma
- Dimensions (px, as written): height=1; width=1

#### LiveStatus

Local design system: what a screen reader says when it changes (« Link copied », « 3 results »), never shown: the eye has it elsewhere. Always in the page, so that a change is heard (a region added with its text is not, in most screen readers). Candidate for twake-ui.

- Exports: LiveStatus  |  Figma: no-figma

#### Loading

Local design system: a progress indicator centred in the space it takes (a list or a panel that loads). Candidate for twake-ui.

- Exports: Loading  |  Figma: mapped → 🔷 progress
- Dimensions (px, as written): padding=3

#### MainColumn

Local design system: a page in the main view, the whole width of it, the room list beside it (Create, folders; D326, full width since D340).

- Exports: MainColumn  |  Figma: no-figma
- Dimensions (px, as written): minHeight=0

#### MediaGrid

Local design system: the media of a room, square tiles edge to edge, three a row (Contact info mockup, D188). A press opens the message of the media. Candidate for twake-ui.

- Exports: MediaGrid, MediaTile  |  Figma: mapped → File type list
- Palette (theme.vars): common.white
- Dimensions (px, as written): m=0; p=0; width=32

#### MenuAction

Local design system: an entry of an actions menu: icon, label, and in red when it destroys something (Delete chat, Leave). Candidate for twake-ui.

- Exports: MenuAction  |  Figma: mapped → 🔹 menuItem
- State hooks: disabled

#### MenuButton

Local design system: the small text button that opens a menu of choices, its current value and a chevron (the role of a member). Candidate for twake-ui.

- Exports: MenuButton  |  Figma: no-figma
- Dimensions (px, as written): gap=0.5; size=16

#### MenuRadio

Local design system: a choice among others in a menu, the current one checked, the names of the list aligned. Candidate for twake-ui.

- Exports: MenuRadio  |  Figma: no-figma
- State hooks: selected

#### MessageBubble

Local design system: a chat bubble, as in the Twake Chat mockups: white on the grey timeline for others, with their name on the first message of a group and their avatar and the tail on the last, light blue for the user. Candidate for twake-ui.

- Exports: BotBadge, MessageBubble  |  Figma: no-figma
- `MessageBubbleProps`: isOwn: boolean; isAllLeft?: boolean; jumboEmoji?: boolean; author: string; label?: string; avatar?: ReactNode; content: ReactNode; time: string; status?: ReactNode; anchorId?: string; isHighlighted?: boolean; onAuthorClick?: () => void; authorLabel?: string; botLabel?: string | null; isFirstOfGroup?: boolean; isLastOfGroup?: boolean; isHideAuthor?: boolean; quote?: { author: string; text: string; onClick?: () => void } | null; editedLabel?: string | null; toolbar?: ReactNode; reactions?: ReactNode; isPinned?: boolean; pinnedLabel?: string; footer?: ReactNode; children?: ReactNode; isDense?: boolean; selection?: { isChecked: boolean; onToggle: (isRange: boolean) => void }; 'data-testid'?: str
- Typography: caption
- Dimensions (px, as written): borderRadius=1; fontSize=48; gap=0.5,1; height=18.345,20; m=0; minHeight=24; minWidth=0,24; my=0.5; p=0; px=0.5,0.75,1.5,2; py=0.5,1; right=120; size=14; width=14.676,20
- State hooks: &:hover, focus-visible, selected

#### MessageToolbar

Local design system: the actions of a message (quick reactions, reply, edit, delete), in the menu of its « … ». The quick ones are a toolbar at the head of the menu: one tab stop, the arrows move between its buttons (WAI-ARIA toolbar pattern). Candidate for twake-ui.

- Exports: KEY_STEPS, MessageHoverReactions, MessageToolbar, MessageToolbarMenu  |  Figma: mapped → 🔷 actionMenu
- Palette (theme.vars): background.paper
- Dimensions (px, as written): borderRadius=1.5,2; fontSize=18; gap=0.25,0.5; height=16,32; minHeight=32; minWidth=200; my=0.5; p=0,0.25,0.5; width=16,32

#### MessagesSkeleton

Local design system: the shimmer of a conversation that loads, drawn as its bubbles: the others on the left with their avatar, the user's on the right.

- Exports: MessagesSkeleton  |  Figma: no-figma
- Dimensions (px, as written): gap=1; height=40,64; maxWidth=520; px=2; py=2

#### NameAvatar

Local design system: the avatar of a name, a picture or its initials.

- Exports: NameAvatar  |  Figma: mapped → 👤 avatar

#### Notice

Local design system: a secondary text in italics (a message that cannot be read, deleted, or of a kind not drawn). Candidate for twake-ui.

- Exports: Notice  |  Figma: no-figma
- Typography: body2, caption

#### OptionCards

Local design system: a radio group drawn as cards side by side, each with its radio and label on top and its own preview below. The whole card is the label of its radio, so a click anywhere on it chooses it.

- Exports: OptionCards  |  Figma: no-figma
- Dimensions (px, as written): gap=1,2; minWidth=0

#### OutgoingAttachment

Local design system: a file the user is sending, shown at once at the end of the timeline from the file itself (an image, the first frame of a video, or the name and size of any other file), under a ring of its upload with its percentage and a button to stop it, until the message arrives. One that failed offers to try again or to drop it. Candidate for twake-ui.

- Exports: OutgoingAttachment  |  Figma: no-figma
- `OutgoingAttachmentProps`: kind: 'image' | 'video' | 'file'; src: string | null; width: number; height: number; name: string; sizeLabel: string; progress: number | null; isFailed: boolean; labels: {; sending: string; cancel: string; failed: string; retry: string; discard: string; }; onCancel: () => void; onRetry: () => void; onDiscard: () => void
- Typography: body2, caption
- Palette (theme.vars): common.white
- Dimensions (px, as written): borderRadius=2; bottom=8; gap=0.5,1; minWidth=0; p=1; px=2; py=0.5; top=4; width=28
- State hooks: &:hover

#### PaneHeader

Local design system: the header of a pane: avatar, title, subtitle, actions. Candidate for twake-ui.

- Exports: PANE_HEADER_HEIGHT, PaneHeader  |  Figma: todo
- `PaneHeaderProps`: title: string; subtitle?: ReactNode; titleMark?: ReactNode; titleBadge?: ReactNode; avatar?: ReactNode; actions?: ReactNode; level?: 1 | 2; navigation?: ReactNode; focusKey?: string; isTitleHidden?: boolean; isFloating?: boolean; onTitleClick?: () => void; titleActionLabel?: string
- Typography: caption, h5
- Palette (theme.vars): text.secondary
- Dimensions (px, as written): gap=0.75,1; height=0; minHeight=24; minWidth=0; px=1,2; py=0.5; right=0; top=0
- State hooks: &:hover, focus-visible

#### PanelCard

Local design system: a white card of the info panel holding sections (title above content), separated by lines, as in the mockups. Candidate for twake-ui.

- Exports: ClampedText, PanelAction, PanelCard, PanelSurface, PanelTitle  |  Figma: mapped → 🔷 card
- Typography: body2, caption
- Dimensions (px, as written): borderRadius=3; gap=1,2; minHeight=48; minWidth=0; mx=2; px=2; py=1.5,1.75
- State hooks: disabled

#### PanelHero

Local design system: the head of a side panel, a centred avatar and its texts (profile). Its close button is in the header of the pane. Candidate for twake-ui.

- Exports: HeroName, PanelHero  |  Figma: todo
- Typography: h4
- Dimensions (px, as written): gap=1; px=2

#### PanelTile

Local design system: a square action of the info panel, icon above its label, as in the mockups (Mute, Search). The one row of actions of the info of a room and of the profile of a person (D355). Candidate for twake-ui.

- Exports: PanelTile, PanelTiles  |  Figma: todo
- `PanelTileProps`: icon: ReactNode; label: string; isActive?: boolean; disabled?: boolean; onClick: (anchor: HTMLElement) => void; hasMenu?: boolean; 'data-testid'?: string
- Typography: caption
- Dimensions (px, as written): borderRadius=3; gap=0.5,1; px=2
- State hooks: &:hover, aria-pressed, disabled, selected

#### PickRowButton

Local design system: the button of a row of people, light blue under the pointer and once picked, as a conversation of the room list (RoomRow): its grey last activity stays readable in the dark theme, where it is not on the grey hover of MUI. Candidate for twake-ui.

- Exports: PickRowButton  |  Figma: mapped → Contact list
- Palette (theme.vars): primary.main
- State hooks: &:hover, Mui-selected, selected

#### PickedStrip

Local design system: the people picked in a list, as one line of avatars with a first name under each and a small cross to take one out. It scrolls sideways, never wraps; the newest is at the end and scrolled into view. Not a twake-mui Chip: a chip is a pill, with no avatar over a caption. Candidate for twake-ui.

- Exports: PickedStrip  |  Figma: todo
- Typography: caption
- Palette (theme.vars): action.hover, background.paper, divider
- Dimensions (px, as written): gap=1.5; height=24; m=0; p=0; px=2; right=0; size=14; top=-4; width=24,56
- State hooks: &:hover

#### PicturePicker

Local design system: the picture of something being created, a grey circle with a camera until one is picked (« Create a group » of the Twake Matrix Figma). A press asks for the file. Candidate for twake-ui.

- Exports: PicturePicker  |  Figma: mapped → Placeholder
- Dimensions (px, as written): p=0; size=32
- State hooks: selected

#### PillButton

Local design system: the primary button at the foot of a pane (« Next », « Create » of the Twake Matrix Figma, node 43881:156148), at its own width on the right, never across the whole pane (Bob, 2026-10-23). With `cancel`, the secondary button sits right before it, same size (Swann, 2026-10-27). Candidate for twake-ui.

- Exports: PillButton  |  Figma: todo
- Dimensions (px, as written): gap=1; minWidth=160; padding=2

#### PinnedBanner

Local design system: the pinned message shown under the header of a conversation, as the Flutter Twake Chat: a blue bar, « Pinned message », its text; a click shows it in the timeline, the list icon on the right shows them all. Candidate for twake-ui.

- Exports: PinnedBanner  |  Figma: no-figma
- Dimensions (px, as written): gap=1,1.5; minWidth=0; px=3; py=0.75; width=3
- State hooks: &:hover

#### PollCard

Local design system: a poll inside a message (D54): its question, one button per answer (pressed: the answer of the user), filled by its share of the votes when the results show. Candidate for twake-ui.

- Exports: PollCard  |  Figma: no-figma
- `PollCardProps`: question: string; answers: readonly PollCardAnswer[]; summary: string; endedLabel: string | null; isVotingClosed: boolean; note: string | null; onVote: (answerId: string) => void; end: { label: string; onClick: () => void } | null
- Typography: caption, subtitle1
- Dimensions (px, as written): gap=0.75,1; minWidth=0; width=400
- State hooks: &:hover, Mui-disabled, aria-pressed, disabled

#### PresenceBadge

Local design system: the presence of a person as a dot over the bottom right of their avatar (Clément, 2026-10-28). Green online, amber away, no dot offline. The dot is decoration: the presence text says it.

- Exports: PresenceBadge  |  Figma: no-figma
- Palette (theme.vars): background.paper, success.main, warning.main
- Dimensions (px, as written): bottom=0; right=0

#### ProfileCard

Local design system: the cards of the profile page (Figma Profile): a thin grey outline, soft corners, no shadow, rows with a line between them.

- Exports: ProfileCard, ProfileChoice, ProfileChoices, ProfileRow  |  Figma: no-figma
- Dimensions (px, as written): fontSize=14; gap=1.5,2; height=10,24; m=0; px=2; width=10,24
- State hooks: &:hover, disabled

#### PromptButtons

Local design system: first requests to send, one button each, side by side under an empty conversation, with a note under them. Candidate for twake-ui.

- Exports: PromptButtons  |  Figma: no-figma
- Typography: body2
- Dimensions (px, as written): gap=1,1.5
- State hooks: disabled

#### QrCode

Local design system: a QR code drawn from bytes, for another device to scan. Always dark on white, whatever the theme: a scanner reads it, not the user. Candidate for twake-ui.

- Exports: QrCode  |  Figma: no-figma
- Dimensions (px, as written): borderRadius=1

#### QuestionButtons

Local design system: « Yes » and « No » under a message that asks a yes/no question, and a line to send the answer again when it did not go out. Candidate for twake-ui.

- Exports: QuestionButtons  |  Figma: no-figma
- Typography: caption
- Dimensions (px, as written): gap=1
- State hooks: disabled

#### ReactionChips

Local design system: the reactions under a message, as the chips of the Feed of Twake Space: one chip per emoji, with the face of who gave it when they are alone, the count beyond; the user's own are highlighted and a click toggles them. Hovering a chip shows, right above it, its emoji and up to three names (« +N » beyond); its label for the screen readers keeps every name. With several emojis, a « … » opens who gave what. Candidate for twake-ui.

- Exports: ReactionChips  |  Figma: no-figma
- Dimensions (px, as written): fontSize=20; gap=0.5,1.5; height=28; maxWidth=320
- State hooks: aria-pressed

#### ReadTicks

Local design system: the ticks of a message of the user, as Telegram draws them: a clock while it goes to the server, one tick once the server has it, two in blue once someone read it (the shape tells it, not the colour alone). Candidate for twake-ui.

- Exports: ReadTicks  |  Figma: no-figma
- `ReadTicksProps`: isRead: boolean; isSending?: boolean; label: string; 'data-testid'?: string
- Dimensions (px, as written): height=16; width=16

#### RequestCard

Local design system: a request shown in the conversation, below its messages (an assistant asked into the room), with its explanation and the answers when the reader has one to give. Candidate for twake-ui.

- Exports: RequestCard  |  Figma: no-figma
- Typography: body2, caption
- Palette (theme.vars): background.paper
- Dimensions (px, as written): borderRadius=2; gap=0.5,1,1.5; maxWidth=480; my=1; p=2

#### ResolvedMark

Local design system: the check of a resolved thread, named for screen readers. Candidate for twake-ui.

- Exports: ResolvedMark  |  Figma: no-figma
- Dimensions (px, as written): size=16

#### RichText

Local design system: a code block of a message opened in a window that fills the screen but stays a window, from a button that shows only while the pointer is over the block (as in Slack). Candidate for twake-ui.

- Exports: CodeExpandButton, CodeExpandDialog, RichText  |  Figma: no-figma
- `RichTextProps`: html: string | null; text: string; highlight?: string; onUserLinkClick?: (userId: string) => void; personName?: (userId: string) => string | undefined; everyone?: boolean; onRoomLinkClick?: (; roomId: string,; eventId: string | null,; via: readonly string[]; ) => void; parseLink?: (href: string) =>; | { kind: 'user'; userId: string }; | {; kind: 'room'; roomId: string; eventId: string | null; via: readonly string[]; }; | null; onWebLinkClick?: (href: string) => boolean; phone?: {; callLabel: (number: string) => string; copyLabel: string; onAction?: (action: 'call' | 'copy') => void; }; expandCode?: {; buttonLabel: string; title: string; closeLabel: string; }
- Typography: subtitle1
- Palette (theme.vars): background.paper, text.primary, warning.main
- Dimensions (px, as written): borderRadius=0.5; gap=1; m=0; p=1,2; size=16,20

#### RoomLine

Local design system: a small grey centred line among the messages (who created, added, renamed or left). Candidate for twake-ui.

- Exports: RoomLine  |  Figma: no-figma
- Dimensions (px, as written): px=2; py=0.5

#### RoomRow

Local design system: a conversation in a list, as the Flutter Twake Chat draws it: avatar, name and time, then who wrote last and what, with the ticks of the last message of the user under the time. The unread count sits at the right end of the row, centred on its height; the time is hidden while it shows (D608). The kind of the room stays on the corner of the avatar. Candidate for twake-ui.

- Exports: RoomRow, UnreadBadge  |  Figma: mapped → Message list
- `RoomRowProps`: name: string; query?: string; previewQuery?: string; avatar?: ReactNode; isDense?: boolean; time?: string; sender?: string; preview?: string; media?: 'image' | 'video'; tick?: { isRead: boolean; label: string }; mutedLabel?: string; pinnedLabel?: string; encryptedLabel?: string; assistantLabel?: string; unreadCount: number; hasMention: boolean; isMarkedUnread?: boolean; isSelected: boolean; onOpen: () => void; unreadLabel: string; menu?: ReactNode; action?: ReactNode; kind?: RoomKindBadge; depth?: number; expander?: { isOpen: boolean; label: string; onToggle: () => void }; detail?: string; optionId?: string; participants?: ReactNode; isPressed?: boolean; selection?: { isChecked: boolean }; o
- Typography: body2, caption
- Dimensions (px, as written): borderRadius=8; bottom=-2; fontSize=10; gap=0.25,1; height=8,16,20; left=8; minWidth=0,16; p=0; px=0.5; py=0.5; right=-2,8; size=14; top=0,32,48; width=8,20
- State hooks: &:hover, focus-visible, aria-pressed, aria-selected, aria-current, Mui-selected, disabled, selected

#### SearchField

Local design system: the SearchBar of twake-mui, readable in the dark theme. Its root reads `theme.palette`, which keeps the light values once the theme switches through CSS variables: a light field on the dark sidebar. The background is set again through `sx`, which follows the colour scheme. To fix in twake-mui (use `theme.vars`), then remove.

- Exports: SearchField  |  Figma: mapped → 🔷 searchBar
- Palette (theme.vars): primary.main

#### SearchMatchBar

Local design system: the bar of a search in a conversation, under its header: what was searched, « 2 of 5 », ↑ to an older match, ↓ to a newer one, × to close, as in Slack. Candidate for twake-ui.

- Exports: SearchMatchBar  |  Figma: no-figma
- `SearchMatchBarProps`: query: string; position: string; olderLabel: string; newerLabel: string; closeLabel: string; onOlder: (() => void) | null; onNewer: (() => void) | null; onClose: () => void
- Dimensions (px, as written): gap=1; minWidth=0; py=0.25; size=16
- State hooks: disabled

#### SectionTitle

Local design system: the title of a group in a list, light, without the band of `ListSubheader`. Candidate for twake-ui.

- Exports: SectionTitle  |  Figma: mapped → 🔹 ListSubHeader
- `SectionTitleProps`: children: string; expander?: { isOpen: boolean; onToggle: () => void }; unread?: { count: number; hasMention: boolean; label: string }
- Typography: caption
- Dimensions (px, as written): borderRadius=1; gap=1; height=18; minWidth=0; p=0; px=2; width=18

#### SeenBy

Local design system: who has read, under the newest message each one read (D470), as small avatars or as a line of names. Nothing to press: the « Seen » submenu of the message lists them all, with the time (D327). Candidate for twake-ui.

- Exports: SeenBy  |  Figma: no-figma
- `SeenByProps`: variant: 'avatars' | 'names'; label: string; avatars: readonly ReactNode[]; isEnd?: boolean
- Typography: caption
- Dimensions (px, as written): px=0.25

#### SelectionBar

Local design system: the dark bar at the top of a conversation while messages are ticked: a check, how many, then the actions and a cross (Figma mockup 13). Dark in both schemes, white on it (AA). Candidate for twake-ui.

- Exports: SelectionBar  |  Figma: todo
- Dimensions (px, as written): gap=1; px=2; py=0.5; size=16,18
- State hooks: Mui-disabled, disabled, selected

#### SettingsList

Local design system: the list of the settings, as the Flutter Twake Chat draws it: the title, the user (picture, name, id), a row per section with its icon and a chevron, the chosen one on the select state of the Twake UI (action/select) with the primary blue for icon and label, and the sign-out in red at the end. Candidate for twake-ui.

- Exports: SettingsList  |  Figma: todo
- `SettingsListProps`: title: string; profile: { avatar: ReactNode; name: string; id: string }; sections: readonly SettingsSection[]; selectedId: string | null; onSelect: (id: string) => void; signOutLabel: string; signOutIcon: ReactNode; onSignOut: () => void
- Typography: caption, subtitle1
- Dimensions (px, as written): gap=2; minHeight=0; minWidth=0; mx=1; px=2; py=0.5
- State hooks: &:hover, aria-current, Mui-selected, selected

#### SettingsSection

Local design system: a section of a settings page, after the « Notifications » page of the Figma (sections 16 px apart, by the screen): a bold heading, its controls 8 px under it, a small muted helper under what it explains. Candidate for twake-ui.

- Exports: SettingHelper, SettingsSection  |  Figma: no-figma
- Typography: caption, subtitle1

#### SidebarHeader

Local design system: the top of a tab of the left column (Chats, Contacts, Activity, Settings, a folder): title, actions, search. One component so that switching tabs moves nothing: the title, the buttons and the search sit at the same pixels in every tab, in both densities. Candidate for twake-ui.

- Exports: SidebarHeader  |  Figma: todo
- `SidebarHeaderProps`: title: string; titleBadge?: ReactNode; navigation?: ReactNode; actions?: ReactNode; level?: 1 | 2; focusKey?: string; search?: Omit<ComponentProps<typeof SearchField>, 'size'>; searchAction?: ReactNode
- Typography: h4
- Dimensions (px, as written): bottom=0; gap=0.75,1; minWidth=0; px=2; py=0.5; right=4; top=0

#### SpaceCard

Local design system: what a space shows in place of a conversation, as the mockup of Twake Chat draws it: its picture, its name, who may come in, what it is for, and its actions. Candidate for twake-ui.

- Exports: SpaceCard  |  Figma: no-figma
- Typography: body2, h4
- Dimensions (px, as written): gap=1,1.5; maxWidth=420; p=3,4; size=16

#### SplashLoader

Local design system: the loading screen of the Flutter client (white page, the Twake Chat animation 176 px in the middle played once, the Twake Workplace logo 210 px wide 40 px from the bottom). The player loads lazily; the box stays empty until it is there.

- Exports: SplashLoader  |  Figma: no-figma
- `SplashLoaderProps`: label: string; 'data-testid'?: string
- Dimensions (px, as written): bottom=40; height=176; width=176,210

#### Spotlight

Local design system: a large dialog with a big search field on top, the results below in sections and the keys at the bottom, as Spotlight on a Mac. Candidate for twake-ui.

- Exports: Spotlight, SpotlightSection  |  Figma: no-figma
- Typography: caption, h6
- Dimensions (px, as written): fontSize=20; gap=0.75,1,1.5,2; height=24; m=0; p=0; px=0.75,2.5; py=1,2; width=24

#### Stack

Local design system: a flex box with spacing in theme units, so screens lay out without `sx` (twake-react-app section 5). Candidate for twake-ui.

- Exports: Stack  |  Figma: no-figma
- `StackProps`: direction?: 'row' | 'column'; gap?: number; padding?: number; paddingX?: number; paddingY?: number; maxWidth?: number; align?: 'start' | 'center' | 'end' | 'stretch' | 'baseline'; justify?: 'start' | 'center' | 'end' | 'space-between'; grow?: boolean; scroll?: boolean; wrap?: boolean; sticky?: boolean; textAlign?: 'left' | 'center' | 'right'; component?: ElementType; onNearEnd?: () => void; onSubmit?: ComponentProps<'form'>['onSubmit']; noValidate?: boolean; children?: ReactNode; 'data-testid'?: string
- Dimensions (px, as written): gap=0; height=24; left=0; m=0; minHeight=0; minWidth=0; right=0; top=0

#### StatusLine

Local design system: the line under a person's name, `status · presence` (Clément, 2026-10-28). The status cuts with an ellipsis first (its whole text in a tip), the presence never; what someone is typing replaces both.

- Exports: StatusLine  |  Figma: no-figma
- Palette (theme.vars): text.disabled
- Dimensions (px, as written): minWidth=0; px=0.5
- State hooks: disabled

#### ThreadCard

Local design system: the card of a thread under its root message, as in Discord (DECISION.md D3). No title: the root message sits just above. Candidate for twake-ui.

- Exports: ThreadCard  |  Figma: no-figma
- `ThreadCardProps`: countLabel: string; lastAuthor: string | null; lastText: string | null; lastWhen: string | null; isUnread: boolean; unreadLabel?: string; resolvedLabel?: string; onClick: () => void; 'data-testid'?: string
- Typography: caption
- Dimensions (px, as written): borderRadius=2; gap=0.75; height=8; px=1.5; py=1; width=8
- State hooks: &:hover, disabled

#### Timeline

Local design system: the scrolling list of messages, which tells whether the reader is at its bottom, keeps a reader who is there and, for one who went up, shows how many messages came below. Candidate for twake-ui.

- Exports: Timeline  |  Figma: no-figma
- `TimelineProps`: label: string; scroller: RefObject<HTMLDivElement | null>; atBottom: RefObject<boolean>; onScroll?: (isAtBottom: boolean) => void; onResize?: () => void; isHidden?: boolean; onPress?: () => void; newBelow?: {; label: string; count: number; countLabel: string; onClick: () => void; }; announcement?: string; isCentered?: boolean; children: ReactNode
- Palette (theme.vars): background.paper, primary.contrastText, primary.dark, primary.main
- Dimensions (px, as written): bottom=16; height=24; minHeight=0; mx=2; px=0; py=2; right=16; width=24
- State hooks: &:hover

#### TintedIcon

Local design system: a round icon, primary on primary light, as the entries of the Create page. Candidate for twake-ui.

- Exports: TintedIcon  |  Figma: todo
- Palette (theme.vars): primary.light, primary.main
- Dimensions (px, as written): height=48; width=48

#### ToggleRow

Local design system: a switch with its label and a line under it saying what it does, the switch at the end, an icon before the label if given. Disabled = locked: the switch is faded with a lock, the text is not. Candidate for twake-ui.

- Exports: ToggleRow  |  Figma: mapped → 🔷 switch
- Typography: body1, caption
- Dimensions (px, as written): gap=2
- State hooks: disabled

#### TwoColumns

Local design system: two blocks side by side, stacked on a narrow screen.

- Exports: TwoColumns  |  Figma: no-figma
- Dimensions (px, as written): gap=3

#### VideoPreview

The corner player of a video (D300, D394): mounted once above the screens, so it outlives the message that handed it the video and the room it was in.

- Exports: MiniPlayer, VideoPreview  |  Figma: no-figma
- `VideoPreviewProps`: poster?: string | null; width: number; height: number; durationLabel: string | null; playLabel: string; onOpen: () => void
- Typography: caption
- Dimensions (px, as written): borderRadius=2; minHeight=120; minWidth=160; px=1; py=0.5; top=4
- State hooks: &:hover

#### VoicePlayer

Local design system: a voice message in the timeline (D54): play or pause, and its waveform as the progress bar, which seeks with a click or the arrows. Candidate for twake-ui.

- Exports: VoicePlayer  |  Figma: no-figma
- `VoicePlayerProps`: src: string | null; duration: number; bars: readonly number[]; playLabel: string; pauseLabel: string; positionLabel: string; formatTime: (milliseconds: number) => string; onPlay?: () => void
- Typography: caption
- Dimensions (px, as written): borderRadius=1; gap=1; height=32; minWidth=120; width=3
- State hooks: focus-visible, disabled


---

## 6. Gaps and cautions

- **Fonts**: Inter is declared but not shipped by twake-mui; locate the web app's font files before bundling.
- **Per-component dimensions** of `ds/*` are in JSX `sx`, not in a machine-readable theme: section 5 lists them as written but a few components compute sizes in code (`PresenceBadge.dot`, `AvatarStack`, `FloatingWindow`, density branches). For pixel parity, read the individual source file listed in the catalogue.
- `tokens.json -> components` callbacks were evaluated with the **light** theme only; dark variants are expressed there as `*:where([data-theme="dark"]) &` selectors where twake-mui wrote them, otherwise the dark values come from the palette variables.
- Figma: not queried. `figma-map.json` gives component names in Library Twake (`6Sf5ux54xWxOjwN3j7gL78`). 74 ds components have no Figma counterpart (Chat only, drawn in the Twake Matrix mockups).
- Local icons of `src/ds/icons.tsx` (~40 Material paths) are not in `tokens.json`.
- The wallpaper JPGs (`public/images/chat-bg-*.jpg`) and the logo (`public/twake-chat-logo.svg`) are assets to copy.
- Web-only constructs without a GPUI equivalent: `color-mix`, CSS vars with fallbacks, `:where([data-theme])` selectors, ripple, `scrollbar-width`, `content-visibility`. Handle with the `Rgba::mix` helper and explicit state structs.
- AGENTS.md vs code mismatches noticed: AGENTS.md says "the page is the `background.paper` of the theme alone" while `wallpaper.ts` paints gradient images behind conversations outside Twake Space (DECISION D192 vs D158 scope: consistent, only in-frame); `docs/accessibility.md` still lists "Calls are not built yet (phase 7)" and an "emoji picker of 64 emojis, without search" gap, stale against AGENTS.md (calls exist; the emoji picker has a GIF tab). DECISION.md D23 (darker brand colours) is superseded by D245 and no longer matches `theme.ts`, as D245 says.
