# Accessibility in `gpui-unofficial` 1.28.1: what exists, what is missing for VoiceOver

Read from the crate sources in `~/.cargo/registry/src/*/gpui-unofficial-1.28.1`, `gpui-macos-gpui-unofficial-1.28.1`
and run on macOS 26 (Darwin 25.4), 2026-11-13. Nothing here was tried with VoiceOver itself: **VoiceOver: not tested**.

## What exists (read in the sources)

- **AccessKit is a hard dependency of `gpui`** (`accesskit 0.24`, `[dependencies.accesskit]` of
  `gpui-unofficial/Cargo.toml`; no cargo feature switches it off). macOS adapter: `gpui-macos` depends on
  `accesskit 0.24` and `accesskit_macos 0.26` (`SubclassingAdapter`). Linux: `accesskit_unix` (D-Bus, via `zbus`), Windows:
  `accesskit_windows`. Crate 0.2.2 has no accessibility code at all.
- **Files**: `gpui/src/_accessibility.rs` (the guide), `gpui/src/window/a11y.rs` (905 lines: the tree builder, activation
  state, action dispatch) and `a11y/debug.rs`, `gpui/examples/a11y.rs`, `gpui/src/platform.rs` (`A11yCallbacks`,
  `a11y_init`, `a11y_tree_update`, `a11y_update_window_bounds`), `gpui-macos/src/window.rs` (the adapter, lines 704, 2406-2460,
  3247), `gpui/src/elements/div.rs` and `text.rs` (roles and node info).
- **Model**: an element is reported only when it has an `id` (a `GlobalElementId`) **and** a role. API on elements:
  `.role(accesskit::Role)`, `.accessibility_id()`, `.a11y_label()`, a description, a focused-node marker, `.on_a11y_action(..)`,
  `.a11y_synthetic_children(..)`. `text!("...")` gives a `Text` with a source-location id and role `Label` (its value is the
  string); a plain `&str` / `String` child has **no id and no role**, so it is not in the tree.
- **Activation**: the tree is built only while the platform adapter says assistive technology is active
  (`A11y::is_active`, `window.rs` 3909, 4008-4030); `accesskit_macos` activates when a client queries the window.
- **Known upstream bugs** (from spike S1): #66691 focus, #66690 cached views, #65623 duplicate ids.

## What was observed (macOS 26, our current code, `--preview`)

`swift ax.swift <pid>` (AXUIElement, `AXIsProcessTrusted()` true for the terminal, so a command-line dump works) gives:

```
AXApplication | twake-desktop
  AXWindow | Twake Chat
    AXGroup            (AccessKit root)
    AXButton, AXButton (+ AXGroup), AXButton   (the three native traffic-light buttons)
    AXStaticText | Twake Chat                  (native title)
```

So the AccessKit adapter is live in the window (an `AXGroup` root exists), but **none of our content is in the tree**: no room
row, no message, no composer.

## What is missing for VoiceOver on Twake Chat

1. **Our views set no role, no id, no label.** `twake-ui-kit` and `twake-ui` use `div()` and plain string children. To be
   reported they need `.id(..)` + `.role(..)` + `.a11y_label(..)` (Button: `Role::Button`, room row: `Role::ListItem` in a
   `Role::List`, message: `Role::ListItem`/`Role::Article` with author, time and status in the label, composer:
   `Role::MultilineTextInput`, ids stable per room id / event id because VoiceOver announces on id change).
2. **Text input**: `TextInput` is a custom `Element` with its own input handler (A021). Whether the 1.28 adapter exposes a text
   field value, caret and selection for such an element (AccessKit text-input nodes, `set_text_selection`) and handles
   `Action::SetValue` is **not checked**; a grep for text input or selection in `window/a11y.rs`, `elements/` and the example found nothing.
3. **Virtual list**: `twake_ui::virtual_list` draws only the visible rows; the accessibility tree contains only those, so
   VoiceOver cannot reach the others without scroll actions (`Action::ScrollIntoView` handlers) and set-size / position
   information. Not designed.
4. **Focus order and focus ring** announced by VoiceOver (upstream #66691), Dynamic Type, reduce motion: not checked.
5. **Actions**: `.on_a11y_action(Action::Click, ..)` must be wired on each button and row so VoiceOver's "press" works.
6. **A test**: no automated check exists. The AX dump above is the only tool used; a regression test per screen needs a
   scripted `AXUIElement` walk (needs the terminal to be trusted).

Each point is work on `twake-ui-kit` / `twake-ui`, behind `twake_gpui`; none needs a change of dependency. This is
the 1.28 groundwork for the NFR on VoiceOver (ARCHITECTURE section 10), not its delivery.
