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
