# GPUI notes (first facts of spike S1, seen on 0.2.2; "1.28" lines added on `gpui-unofficial` 1.28.1, A028)

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
  `BoxShadow` (a border would collide with the outlined one). GPUI 1.28 has inset shadows (`BoxShadow { inset: true }`), so "no inset shadow" no longer holds. The keyboard-only rule of design-system.md 1.7
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

## `TextInput` (composer field on `TextBuffer`, A021), seen 2026-11-12, macOS, gpui 0.2.2, US International PC layout

Checked on screen in `--preview` (window captured with `screencapture -l <id>`, keys and clicks injected with `osascript` / `CGEvent`):

- **Works**: focus by click and the 2 px ring; typing; wrapping at the width of the field; Shift+Enter makes a line and the field grows up to 8 rows, then scrolls to keep the caret in view (both ways); Enter sends the text as a local echo in the thread and empties the field; double click selects a word, triple click a line, drag selects across lines; Shift+Down, Alt+Shift+Left, Cmd+Right, Cmd+Up, Cmd+A, Cmd+C, Cmd+V, Cmd+Z, Backspace on a selection.
- **Dead keys work through `EntityInputHandler`**: backtick then `a` gives `à`, acute then `e` gives `é` (D602), the pending accent is drawn underlined (marked text), acute then space leaves the bare accent.
- **Key events**: GPUI sends a printable key to the keymap first and, when nothing handles it, to the system input method (`handle_key_event`, `platform/mac/window.rs`); while a composition is open the IME gets the key first. So the field uses `on_key_down` for Enter, Backspace, arrows and Cmd shortcuts and leaves every letter to `replace_text_in_range` / `replace_and_mark_text_in_range`. `cx.stop_propagation()` is what tells macOS the key was handled.
- **`actions!` and `#[derive(Action)]` name the `gpui` crate** (`gpui::Action`), like `IntoElement`: a crate that only depends on `twake-gpui` cannot use them (A010). `twake-gpui` already re-exports `gpui::*`, so `EntityInputHandler`, `ElementInputHandler`, `FocusHandle`, `Focusable`, `ClipboardItem`, `KeyDownEvent`, `WrappedLine`... come through it; the field needs no action.
- **Text injected by System Events `keystroke "é"` arrives as `a`** (the synthetic event carries the key, not the character): not a field bug; use key codes for dead keys. `keystroke` of an emoji gave `aa`.
- **Multi-line text**: `TextSystem::shape_text` splits on `\n` and gives one `WrappedLine` per logical line; the wrap points are `wrap_boundaries()` (glyph index of the first glyph of the next row). `WrappedLayout::position_for_index` puts an offset at a wrap point on the upper row, so the field finds the row itself (the last row that starts at or before the offset) and measures with `unwrapped_layout.x_for_index`. `request_measured_layout` gives the height from the width the parent grants; the text is shaped again in `prepaint` at the real width (the line cache makes it cheap).
- **Up and Down** go by visual rows from the geometry of the last paint (the model moves by logical lines); the x is the one of the caret at each press (no goal column across wrapped rows).
- **Caret blink**: a task of the entity (`spawn_in`, 530 ms timer) toggles the caret only while the field is focused and has had no input for 530 ms; each key resets it. The timer still wakes every 530 ms when the field is not focused (no redraw then).
- **Clipboard**: `cx.read_from_clipboard` / `write_to_clipboard` with `ClipboardItem::new_string`; paste goes through `TextBuffer::paste` (plain text, CRLF to LF, control characters dropped, D332). Copy and paste checked on screen.
- **Not checked**: a real IME with a candidate window (Japanese, Chinese): `bounds_for_range` is written for it (the candidate window should sit under the marked text) but no IME was driven; the caret inside a clause is not followed (A021). Emoji picker (Ctrl+Cmd+Space, `show_character_palette`) is not wired. No wheel scroll inside the field (the caret scrolls it). Drag after a double or triple click selects by characters, not by words or lines. VoiceOver reads nothing from the field yet (no accessibility node).
- **Perf** (`TWAKE_PREVIEW_BENCH=1 --preview`, 600 frames scrolling the 10 043 rows with the composer under the thread, window on screen): `release-fast` frame ms p50 4.2, p95 4.5 and 4.7, p99 6.3 and 8.2, max 126.3 and 101.1 (first paint), 2 frames over 25 ms, row build p50 0.02 (2 runs); debug p50 4.2, p95 4.6, p99 5.3, max 463.7, 2 over 25 ms. Same as without the composer. The field itself allocates per frame: the shaped rows (`Vec<Row>`, `Vec<WrappedLine>`) and the text clone of the measure closure (a `SharedString`, so a reference count); only the composer is concerned, not the list. The text is copied into a `SharedString` once per edit, not per frame.
- Side effect of the test: Cmd+C overwrote the clipboard of the machine with the test text.

## Moving to `gpui-unofficial` 1.28.1 (A028, 2026-11-13, macOS 26)

- **`twake_gpui::gpui_platform::application()` replaces `Application::new()`**: the platform is a separate crate.
- **`gpui_platform` needs its `font-kit` feature on macOS.** Without it the build and the window work, layout is right,
  but no glyph is drawn anywhere (`WARN gpui_macos: compiled without the font-kit feature, so no text will be rendered`,
  visible only with a `log` logger: the app installs none). SVG icons and boxes still draw. `twake-gpui` enables it.
- Renamed: `Component` to `ViewElement`, `ScrollWheelEvent` to `ScrollEvent`, `on_scroll_wheel` to `on_scroll` (the old
  names are deprecated aliases for now). `BoxShadow` has a new field `inset`. `Window::focus(&handle, cx)` takes the `App`.
- `gpui-unofficial` 1.28.1 pulls 142 more crates and no new licence; AccessKit, `wgpu` and Linux/Windows crates are in the
  lockfile but not compiled on macOS (`cargo tree --target aarch64-apple-darwin`).
- Accessibility: `docs/dev/gpui-a11y.md`.
