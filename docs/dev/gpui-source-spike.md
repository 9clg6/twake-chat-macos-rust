# Spike S1: which GPUI source behind `twake-gpui` (2026-11-12)

Branch `spike/gpui-source`. No production code changed: the experiment is the patch
`docs/dev/gpui-source-spike-unofficial.patch` (crates only, `Cargo.lock` left out). Facts are marked
**sourced** (URL and words), **measured** (this machine) or **not verified**.

## 1. State of the sources

### 1.1 Upstream Zed (`zed-industries/zed`)
- Latest commit seen on `main`: `d3fc053d5b62386247b15f069871eabe817032ab`, 2026-11-12 (GitHub API `repos/zed-industries/zed/commits`). Latest stable tag: `v1.28.1`, commit `e8bc16acee5ae1937600d571d441c4b5d7b0d288`, 2026-11-11 (releases API; `v1.29.1-pre` the same day is a pre-release).
- Crates in `crates/`: `gpui`, `gpui_platform`, `gpui_macos`, `gpui_windows`, `gpui_linux`, `gpui_wgpu`, `gpui_apple`, `gpui_ios`, `gpui_web`, `gpui_macros`, `gpui_util`, `gpui_shared_string`, `gpui_tokio`. All `license = "Apache-2.0"` (their `Cargo.toml` at `v1.28.1`). The repo as a whole is `NOASSERTION` on GitHub (Zed itself is mixed), so only the `gpui*` crates matter.
- **crates.io `gpui` has not moved**: newest `0.2.2`, 2025-10-22, licence Apache-2.0 (crates.io API). On `main`, `crates/gpui/Cargo.toml` still says `version = "0.2.2"`: the version number is not bumped, so the git `rev` is the only identity.
- **Pre-1.0, breaking**: `crates/gpui/README.md`: "GPUI is still in active development as we work on the Zed code editor, and is still pre-1.0. There will often be breaking changes between versions."
- **Outside users**: https://github.com/longbridge/gpui-component/discussions/1856, dlight, 2025-12-25: "upstream development of GPUI is de-emphasizing the needs of GPUI users outside of Zed." and "Code that isn't directly usable in Zed isn't being merged in upstream GPUI right now." (second-hand account of a rejected PR; cosoc, 2026-08-20, gives an opinion only). Not an official Zed statement.
- Discord or Zed statements about GPUI outside Zed: **not verified** (one search, nothing found).

### 1.2 Accessibility (the decisive difference)
- **0.2.2 has none**: no `accesskit`, no `NSAccessibility` anywhere in `gpui-0.2.2/src` (grep, measured).
- **Upstream has AccessKit since 2026-05-27**: PR "gpui: Accesskit support (#56065)" (`git log` of `crates/gpui/src/window/a11y.rs`), then "gpui: `Application::inaccessible()` (#57954)", "a11y: Settings UI (#59429)" 2026-06-17, "a11y: Landmarks and menu improvements (#60397)" 2026-07-13.
- Guide (`crates/gpui/src/_accessibility.rs`): "GPUI integrates with [AccessKit] to provide programmatic accessibility features". A node needs an element id and a role: "In order for nodes to be reported, they must also have a non-`None` [`role`]". `div().id(..).role(Role::TextInput)` exists; a custom `Element` can build its own children, text runs and `set_text_selection` (the guide's text-field example), so a hand-written field like our `TextInput` can be exposed.
- Backends: `gpui_macos` uses `accesskit_macos::SubclassingAdapter::for_window` (`window.rs`); `gpui_windows` depends on `accesskit_windows`; `gpui_linux` on `accesskit_unix` (their `Cargo.toml` at `v1.28.1`).
- **Open bugs, upstream** (GitHub issues, open on 2026-11-12): #66691 "gpui_macos: the app reports the window as the focused accessibility element, because AccessKit subclasses the content view instead of the first responder" (VoiceOver asks that question when the user switches to the app); #66690 "gpui: views drawn from the cache drop their accessibility nodes, and focus falls back to the window"; #65623 "Window::transact does not roll back a11y nodes, so List autoscroll retry hits 'Duplicate a11y node id'" (our timeline is a list); #41138 "Windows: Screen reader accessibility missing completely" (old title, the adapter is in the tree now: state **not verified**). Zed itself sets few roles: the `open-gpui` fork comment says Zed "doesn't set any a11y APIs on any UI elements" (via search snippet, **not verified** upstream).
- **Not verified here**: no VoiceOver pass was run (needs a person at the machine). That the tree is empty or full for our screens is therefore unknown; we set no role or id on anything yet.

### 1.3 Forks and repackaging
| Source | Evidence | Maintenance | Licence |
|---|---|---|---|
| `gpui-unofficial` | crates.io: `1.28.1`, published 2026-11-12, 68 versions; repo `gpui-archipelago/gpui-unofficial`, last commit 2026-11-05 (GitHub API). README: "Fully automated standalones release of gpui on each new Zed release tag." and "Versions mirror Zed's release tags". Known gap, README: "there is currently no way to publish a fix for an already-released version without a version suffix". Pulls `accesskit ^0.24`. | Automated, one org, 13,818 downloads. Bus factor and the trust in a publisher who is not Zed: **not verified**. | crates.io Apache-2.0; no `LICENSE` detected by GitHub API (404): **not verified** |
| `gpui-pre` | crates.io `0.3.12`, 2026-11-09, repository field `zed-industries/zed`, description "gpui-pre snapshot of zed@d2a4fb6", 499,036 downloads | Snapshots of a Zed commit, publisher not Zed: **not verified** | Apache-2.0 |
| `gpui-ce` (community edition) | crates.io `0.3.0` 2026-10-20 (docs.rs shows 0.3.3 on 2025-12-27: dates conflict, **not verified**); repo `gpui-ce/gpui-ce` pushed 2026-11-12, 1,604 stars | Active; "drop-in replacement" claim; adds gradients, blur, smooth scroll; needs `gpui_platform` from git per its docs | Apache-2.0 |
| `rgpui`, `open-gpui` | crates.io `1.7.0` 2026-11-05 / `0.2.0` 2026-07-07 | Small, single-author: not evaluated | Apache-2.0 |

`gpui-kit` was not found by name; the packages found under that idea are `gpui-pre` and `gpuikit` (a component kit, not a GPUI source).

### 1.4 Platforms and IME
- Windows and Linux backends are active: last commits `crates/gpui_windows` 2026-11-12, `crates/gpui_linux` 2026-11-11 (GitHub API). Their maturity for a chat app (IME, HiDPI, Wayland) is **not verified**: we have only run macOS.
- IME: a closed issue "Chinese Input Method Editor (IME) Interferes with Editor" #56327, 2026-11-06, shows IME work is current. Our own IME (candidate window) was never driven (`gpui-notes.md`): **not verified**.

### 1.5 `cargo deny` and licences
- Baseline 0.2.2: `advisories ok, bans ok, licenses ok, sources ok` (measured, with the 11 A018 ignores).
- `gpui-unofficial` 1.28.1: same four "ok" (measured, no new ignore needed); `cargo deny` reports 6 of the 11 ignores as "advisory was not encountered" (they can be removed at a bump, A018 review). New duplicates only (warnings): `objc2`, `objc2-app-kit`, `objc2-foundation`, `windows-*`, `thiserror`, `syn`, `toml*`, others.
- Git source: `deny.toml` `[sources]` would need the Zed git URL allowed (**not tested**, the build never resolved).

## 2. Measurements (macOS arm64, rustc 1.99.1, `--release`, one `cargo` at a time under `lockf`)
Worktrees `../twake-s1-base` (main, gpui 0.2.2) and `../twake-s1-zed` (this branch), removed afterwards. The machine had other sessions running (load average 5.9 to 16.6 seen), so build times carry noise.

| | gpui 0.2.2 (baseline) | Zed git `e8bc16a` | `gpui-unofficial` 1.28.1 (= the same tag) |
|---|---|---|---|
| Resolves | yes | **no** | yes |
| Cold build, release `twake-desktop` | 1 min 50 s (first build in a new target) | n/a | 1 min 23 s (after `cargo clean`; 18.7 s for the rebuild after fixing the 4 errors) |
| Binary `twake-desktop` | 6,441,952 B | n/a | 9,478,448 B (+47 %) |
| `cargo deny check` | ok | not run | ok, 6 ignores unused |
| Code changes to compile | none | n/a | 4 errors, 6 lines: `Component` is now `ViewElement` (`IntoElement` by hand), `BoxShadow` has `inset`, `Window::focus` takes `cx`, entry point `gpui_platform::application()` instead of `Application::new()`; 8 deprecation warnings (`ScrollWheelEvent` to `ScrollEvent`, `on_scroll_wheel` to `on_scroll`) |
| `--preview` starts | yes | n/a | yes |
| Bench `TWAKE_PREVIEW_BENCH=1` frame ms (p50 / p95 / p99 / max) | 4.2 / 5.2 / 8.3 / 107.2; 4.2 / 5.4 / 8.4 / 99.8; 4.2 / 4.8 / 8.1 / 100.3 | n/a | 4.2 / 4.3 / 5.8 / 95.0 (2 over 25 ms) |

- **Zed git failed**: `cargo build` with `gpui = { git = zed, rev = e8bc16a }` plus `gpui_platform` gives "failed to select a version for `gpui` ... versions that meet the requirements `*` are: 0.46.0 ... `gpui_platform` depends on `gpui` with feature `windows-manifest` but `gpui` does not have that feature". Cargo picks another package named `gpui` in that repository (a lint fixture under `tooling/lints/test_fixture/gpui`; its version and features do not match what the error prints, so the exact cause is **not verified**). I did not find a workaround in the time box; `gpui-unofficial` repackages the same tag and builds, which is the working form of "pinned Zed".
- **Bench caveat**: the other `gpui-unofficial` runs were disturbed: one with p50 33.3 ms (30 Hz, 379 frames over 25 ms), one with a 75 s stall (max 74,941.9 ms), a later one hung. Baseline hung once too. Window occlusion, App Nap or machine load: cause **not verified**. Only the clean runs are listed; the verdict "no difference in frame time" rests on one clean run per source against three. To redo with the window in front and the machine idle.
- Disk: 36 GiB free before; the two `target/` dirs are about 0.9 GiB each; far above the 15 GiB bar.

## 3. Accessibility options
1. **Stay on 0.2.2, own layer.** Native `NSAccessibility` through `objc2` in `twake-platform`: an accessibility tree for a custom-drawn window means a hand-built `NSAccessibilityElement` per row, bubble and field, kept in sync with GPUI's layout (bounds, focus, text ranges, actions) from outside GPUI, which gives no per-frame hook or element ids (0.2.2 has no id on every node). Estimate (judgement, not measured): several weeks for roles, labels, focus, list and text field on macOS, then again on Windows (UIA) and Linux (AT-SPI). Risks: focus and key handling collide with GPUI's own `NSView` handling; the `unsafe_code = "forbid"` workspace lint means an `unsafe`-bearing crate; every GPUI bump can break it.
2. **AccessKit by hand on 0.2.2 (`accesskit_macos`/`accesskit_winit` adapters).** `accesskit_macos::SubclassingAdapter::for_window` is what upstream GPUI uses, so it is feasible, but needs the `NSWindow`, which GPUI 0.2.2 gives only through the raw window handle (`HasWindowHandle`), plus our own tree builder walking our own models (we know the rows; GPUI does not tell us the pixels). Cheaper than 1 (no per-platform code) but still a parallel tree to maintain, and it hits the issue #66691 class of problem (focus) by itself.
3. **Upstream a11y (via `gpui-unofficial` or git).** Roles and ids are declared on elements (`.id(..).role(..)`), AccessKit does the platform work on three OSes. Cost: migration of the GPUI API (small for us today: 6 lines; grows with the UI) and the open bugs above (focus reported as the window; cached views losing nodes; duplicate ids in lists). A VoiceOver pass is still needed to know how usable it is.

## 4. Recommendation (the owner decides)
**Migrate to `gpui-unofficial` pinned `=1.28.1` behind `twake-gpui`, and keep the escape hatch to a git pin.**
- Why: it is the only source with an accessibility tree; the cost today is 6 changed lines and a +3 MB binary; `cargo deny` stays green and drops 6 ignores; build time and the preview bench show no regression (one clean run). The longer we wait on 0.2.2 the larger the migration (every component written against `Component`, `focus` without `cx`).
- What it costs: dependence on a repackager who is not Zed, and on Zed's cadence; GPUI is "pre-1.0, breaking changes" and Zed de-emphasises outside users (sourced above, second-hand). Mitigation: exact pin (`=1.28.1`, A003), every GPUI import already goes through `twake_gpui` (A002), a bump is a deliberate task with a VoiceOver pass and a bench.
- Alternatives: **stay** (0.2.2) if the a11y date is far away: zero cost now, a rework later and option 1 or 2 for a11y. **Hybrid**: stay on 0.2.2 in `main` and put the migration behind a Cargo feature in `twake-gpui` (two dependency sets): not recommended, the two APIs differ (`Component`, `focus`), a feature flag would hide that in `cfg`.
- Not recommended now: `gpui-ce` (more features, but a divergent API surface and it needs `gpui_platform` from git); Zed git directly (does not resolve here).

## 5. Still to verify
1. A VoiceOver pass on `--preview` on `gpui-unofficial` (roles on the list, bubbles, the field; focus on switching to the app, issue #66691). Set `.id()` and `.role()` first.
2. The bench again, machine idle and window in front, 3 runs each, plus a second binary-size reading with `release-fast`.
3. Why Zed git does not resolve (try a `[patch]` or a path checkout of Zed at `e8bc16a`).
4. The trust model of `gpui-unofficial`: who publishes, how, with which token; whether the published source equals the tag (diff the crate against the tag).
5. Windows and Linux builds and IME (candidate window) on the migrated GPUI.
6. Whether `gpui-unofficial` ships a `LICENSE` file in the crate (`cargo deny` read the field only).
