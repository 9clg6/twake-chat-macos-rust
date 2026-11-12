# GPUI 0.2.2 notes (first facts of spike S1)

Seen while building `twake-ui-kit::view` and `twake-desktop --gallery` (2026-11-12, macOS, gpui 0.2.2).

- **SVG goes by asset path only.** `svg().path("icons/send.svg")` asks the `AssetSource` of the `Application`
  (`Application::new().with_assets(..)`); there is no way to give a string or bytes. `twake_ui_kit::view::IconAssets`
  serves `icons/<id>.svg` from `twake-icons`. An unknown path is only logged (`log_err`), nothing is drawn.
- **SVG is an alpha mask.** The colour comes from `text_color`; a multicolour icon (A013, e.g. `assistant-color`)
  comes out in one colour. `svg_for` (sized, tinted text) is not what GPUI draws: GPUI takes `to_svg()` and the
  size from `.size(px(..))`.
- **`svg()` does not inherit the text colour of its parent.** With no `text_color` on the `Svg` itself nothing is
  painted (the paint needs `style.text.color`). First gallery run: icons missing; fixed by setting it on the element.
- **`#[derive(IntoElement)]` names the `gpui` crate** (`gpui::IntoElement`...), so a crate that only depends on
  `twake-gpui` (A010) cannot use it: implement `IntoElement` by hand with `Component::new(self)`.
- **`Window::open_window`'s build closure must return `Entity<V>`**, not `AnyView`: choose the view before the call
  (two `open_window` calls), not inside the closure.
- **Hover and active** are styles of a stateful element: `div().id(..)` first, then `.hover(|s| ..)` / `.active(|s| ..)`.
  They are applied by GPUI, no state in our code. A disabled button simply does not set them.
- **Focus has no `focus-visible`.** `.focus(|s| ..)` applies on any focus, mouse included; it needs a `FocusHandle`
  (`cx.focus_handle()` in a view) given by `.track_focus(&handle)`. `Button::focus_handle` draws the 2 px ring as a
  `BoxShadow` spread (a border would collide with the outlined one). The keyboard-only rule of design-system.md 1.7
  is therefore not met yet.
- **No letter spacing** in `TextStyle` of 0.2.2: `font.letter_spacing` of `ResolvedButton` is not applied.
- `cx.window_appearance()` gives the OS theme (`Dark | VibrantDark` is dark); used for `Theme::system`.
- Not checked: click handler, the light/dark button, hover and ring on screen (no pointer driven in this run).

## `--preview` (room list and conversation on fixtures), seen 2026-11-12, macOS, gpui 0.2.2

- **`cx.new` needs `AppContext` in scope** (`use twake_gpui::AppContext`), even on `Context<V>`.
- **`flex_1()` in a column is a height rule** (`flex: 1 1 0%`): a title `div` with `flex_1().min_w_0()` inside a `flex_col`
  got height 0 and drew only the ellipsis ("..."). A `text_ellipsis` child of a column needs neither.
- **No window id from GPUI**: capture with `screencapture -l <CGWindowID>` (id read through `CGWindowListCopyWindowInfo`),
  not a full-screen capture, which also grabs whatever is on the desktop.
- **Selected row**: `primary.light` is a pale blue; with the light text of the dark scheme it is unreadable, so the dark
  scheme uses `primary.dark`.
- Seen on screen and still wrong: the footer of a bubble of 5 lines is clipped by one line (the slot height is an estimate,
  `overflow_hidden` cuts the rest); the `JT` avatar (grey 600) has no visible disc on the dark background.
- **Bench** (`TWAKE_PREVIEW_BENCH=1 twake-desktop --preview`, 600 frames scrolling 10 043 rows, window on screen):
  - release: frame ms p50 4.2, p95 4.9, p99 6.8 to 8.4, max 114.7 to 122.6, 2 frames over 25 ms; row build ms p50 0.02, max 0.07 (2 runs).
  - debug: frame ms p50 4.2, p95 5.1, p99 13.8, max 452.0, 2 frames over 25 ms; row build p50 0.05, max 0.13.
  - The 2 slow frames are at the start (first paint, glyph cache); the p50 is the display cadence, not a CPU cost.
- The preview datasets are written by hand in `preview_data.rs`; `twake-testing` stays a dev-dependency (A011 Q4).
