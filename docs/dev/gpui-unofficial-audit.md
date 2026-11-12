# Audit: `gpui-unofficial` 1.28.1 against zed-industries/zed v1.28.1

Date: 2026-11-12. Read-only. Work dir: `.../scratchpad/s1-verify` (crates in `x/`, Zed clone in `zed/`).

## Verdict: DIFFERENCES BENIGNES (code Rust de macOS identique au renommage et à 2 mécanismes près)

For the 15 crates the macOS build pulls, the Rust code equals the Zed tag except: (a) the `inspector` cargo feature is removed (cfg now `debug_assertions` only), (b) asset/shader paths adjusted so the crate is self-contained, (c) dependencies come from crates.io (Zed's `[patch.crates-io]` forks are not applied). No added code, no added dependency name, no added build script, no network/process code. Residual risk: the publisher, not the content (see "Not verified").

## Method

- Tag `v1.28.1` cloned: commit `e8bc16acee5ae1937600d571d441c4b5d7b0d288` (equals the commit in the crate README).
- Crate list: temp project with the same two deps as `docs/dev/gpui-source-spike-unofficial.patch` (`gpui-unofficial =1.28.1`, `gpui-platform-gpui-unofficial =1.28.1`), `cargo generate-lockfile`, `cargo tree --target aarch64-apple-darwin`. 20 `*-gpui-unofficial` crates in the lock (all 1.28.1); 15 on macOS:
  `collections`, `derive-refineable`, `gpui-apple`, `gpui-macos`, `gpui-macros`, `gpui-platform`, `gpui-shared-string`, `gpui`, `gpui-util`, `http-client`, `refineable`, `scheduler`, `sum-tree`, `ztracing`, `ztracing-macro` (all `-gpui-unofficial`). The 5 others (linux, windows, ios, web, wgpu) were downloaded and diffed too.
- All 20 `.crate` files downloaded from `https://crates.io/api/v1/crates/<name>/1.28.1/download` (generic User-Agent), extracted, `diff -rq` against the matching Zed directory (excluding `Cargo.toml`, `Cargo.toml.orig`, `Cargo.lock`, `.cargo_vcs_info.json`), then `diff` of each differing file. Dependency names compared with `tomllib` (`Cargo.toml.orig` vs Zed `Cargo.toml`).

## Files added / removed / modified (vs Zed, per crate)

Added in every crate: `README.md` (generated, marker `<!-- gpui-archipelago:generated -->`), `LICENSE-APACHE` (10.5K, present in all 20 crates), `Cargo.toml.orig`, `Cargo.lock`. No `.cargo_vcs_info.json` in any crate (the commit claim in the README cannot be checked from the package metadata).

Added elsewhere:
- `gpui-unofficial/assets/fonts/{ibm-plex-sans,lilex}/*` : Zed's own `assets/fonts`, byte-identical (diff -r clean), with their `license.txt` / `OFL.txt`. Copied so `include_bytes!` works.
- `gpui-apple-gpui-unofficial/gpui/src/{scene,color,window,geometry,platform}.rs` : byte-identical copies of `crates/gpui/src/*` (read by cbindgen in `build.rs`).
- `gpui-wgpu-gpui-unofficial/assets/`: fonts as above.
Removed: `gpui_web/examples/`, `refineable/derive_refineable/` (separate crate, published as `derive-refineable`, which is identical).
Identical to Zed (no differing file): collections, derive-refineable, gpui-ios, gpui-linux, gpui-macos, gpui-platform, gpui-shared-string, gpui-util, gpui-windows, http-client, refineable, scheduler, sum-tree, ztracing, ztracing-macro, gpui-web (src).

## Rust code differences (all)

1. **Inspector feature removed** (`gpui`: `app.rs`, `element.rs`, `elements/div.rs`, `inspector.rs`, `styled.rs`, `window.rs`; `gpui-macros`: `gpui_macros.rs`). Every `cfg(any(feature = "inspector", debug_assertions))` became `cfg(debug_assertions)` (and the `not(...)` forms). Checked mechanically: after `sed` of that one pattern the six gpui files and gpui_macros are byte-identical to the published ones. Effect: the `inspector` feature no longer exists (Zed's `inspector = ["gpui_macros/inspector"]` dropped); inspector code compiles in debug builds only (same as Zed's default, where the feature is off). Benign. Reason is unstated (likely the publisher's feature rewrite); a release build cannot turn the inspector on.
2. **Asset include paths** : `include_bytes!("../../../assets/fonts/...")` became `"../assets/fonts/..."` in `gpui/src/svg_renderer.rs`, `gpui/examples/example_support/fonts.rs`, `gpui_apple/src/text_system.rs`, `gpui_wgpu/src/cosmic_text_system.rs`, `gpui_wgpu/benches/layout_line.rs`; `gpui_apple/build.rs` `.join("../gpui")` became `.join("gpui")`. Same bytes, new location. Benign.
3. **`gpui/examples/text.rs`**: Lilex font load (`add_fonts`) removed from the example. Example only, not compiled as a dependency. Benign.
4. **wgpu API adaptation** (`gpui_wgpu`: `wgpu_atlas.rs`, `wgpu_context.rs`: `default_queue: wgpu::QueueDescriptor::default()` removed; `wgpu_renderer.rs`: `format.has_srgb_suffix()` became `format.is_srgb()`). Zed uses wgpu from git rev `cddb2f5d…`; the crate uses crates.io `wgpu 30.0.1`. Dependency is `cfg(linux/freebsd)` only: **not compiled on macOS**. Benign for our target.
5. Lib names: `gpui-wgpu` lib is `gpui_wgpu_gpui_unofficial` (rename only; `gpui-unofficial` keeps lib name `gpui`).

No other Rust file differs. No `unsafe`, network, `Command`, or env logic was added anywhere.

## Cargo.toml differences

- Package renamed `X-gpui-unofficial`, version `0.2.2`/workspace -> `1.28.1`, edition 2024, repository -> `gpui-archipelago/gpui-unofficial`. Workspace inheritance flattened to explicit versions (same versions as Zed root `Cargo.toml`: `taffy =0.15.0`, `objc2-core-foundation =0.3.2`, `resvg/usvg 0.46.0`, `accesskit 0.24.0`, …).
- Dependency **names**: none added in any of the 20 crates. Removed from gpui: `bench_metrics`, `gpui_platform`, `gpui_web`, `reqwest_client`, `util_macros` (all dev/bench/test only; Zed-internal crates). `gpui-ios` loses a dev `gpui_platform`.
- New feature `proptest = ["dep:proptest"]` in gpui (additive, off by default); `inspector` removed (see above).
- `[workspace]` empty table and `[lints]` rust unexpected_cfgs added.
- **Resolution difference**: Zed's workspace `[patch.crates-io]` forks (e.g. `async-task` -> `zed-industries/async-task`, rev `015845a`) do not apply to the published crate: `async-task` resolves to crates.io 4.7.1. Our `Cargo.lock` therefore builds against upstream `async-task`, not Zed's fork. `font-kit` is the crates.io `zed-font-kit 0.14.1-zed` (same as Zed's package, owners `mikayla-maki`, `MrSubidubi`, `github:zed-industries:crates-io`); `zed-scap` likewise (optional, linux/windows). No `git =` or `path =` outside crate's own lib targets in any published manifest.

## Build scripts (code run at compile time)

Four `build.rs`, all byte-identical to Zed except the path line in item 2:
- `gpui-unofficial/build.rs` : only `cargo:rustc-cfg` / `check-cfg` / `rerun-if-*` lines; on Windows with feature `windows-manifest` embeds a `.rc` resource. No process spawn, no network, no file writes.
- `gpui-apple-gpui-unofficial/build.rs` (macOS/iOS): `cbindgen` generates `scene.h` in `OUT_DIR` from the bundled `gpui/src/*.rs` copies; then `xcrun metal` / `xcrun metallib` (lines 175, 200) compile `src/shaders.metal` (or stitch it with `runtime_shaders`). Same as Zed. Needs Xcode tools. Writes only in `OUT_DIR`.
- `ztracing-gpui-unofficial/build.rs`: sets cfg from env vars `ZTRACING`, `ZTRACING_WITH_MEMORY`. Harmless.
- `gpui-windows-gpui-unofficial/build.rs`: not compiled on macOS, identical to Zed.
`gpui-macros` / `derive-refineable` are proc macros (code at compile time) : source identical to Zed except the inspector cfg.

## Publisher and history (crates.io API, GitHub API)

- Repository: `https://github.com/gpui-archipelago/gpui-unofficial`; org `gpui-archipelago` created 2026-06-27, 10 public repos, no public members listed.
- crates.io owners of `gpui-unofficial`: user `iamnbutler` (Nate Butler, GitHub account since 2012, 473 followers, company `@githubnext`), user `Vanuan` (John Yani, account since 2010), team `github:gpui-archipelago:gpui-unofficial-maintainers`.
- Crate created 2026-04-03; 68 versions, none yanked; **all 68 published by `iamnbutler`**; 13,818 downloads. Versions `0.230.1` (2026-04-03) to `0.231.1` (2026-04-09), then `1.0.0` (2026-05-02) through `1.28.1` (2026-11-12); roughly one release per Zed tag, `-pre` for Zed pre-releases.
- 1.28.1 published 2026-11-12 (today): freshly published.
- Source of the README claim: `generated from zed v1.28.1 (e8bc16ac…)`, matches the tag found.

## Not verified

- That the crates.io tarball was built from the GitHub repo of the publisher (no `.cargo_vcs_info.json`; I did not diff against `gpui-archipelago/gpui-unofficial`, nor inspect its CI or the token used).
- Identity of the maintainers beyond public profiles; account-takeover risk; bus factor (2 users + a team).
- Transitive crates.io dependencies (taffy, resvg, objc2, zed-font-kit...) were not audited; only the `-gpui-unofficial` crates and their dependency names.
- Behavioural effect of `async-task` upstream instead of Zed's fork (builds fine per the spike; runtime impact not tested).
- The 20 crates for Linux/Windows/iOS/web were diffed but only macOS paths matter here; wgpu change not run.
- Hidden/Unicode tricks: not scanned beyond `diff` (diff is byte-exact for every non-listed file, so any such trick would appear as a differing file; none did).
- Zed's pre-existing code was not reviewed for vulnerabilities: only equality with upstream.

## Reproduce

```
curl -sL -A "audit" -o c.crate https://crates.io/api/v1/crates/gpui-unofficial/1.28.1/download
git clone --depth 1 --branch v1.28.1 https://github.com/zed-industries/zed
diff -rq --exclude=Cargo.toml --exclude=Cargo.toml.orig --exclude=.cargo_vcs_info.json --exclude=Cargo.lock <extracted> zed/crates/gpui
```
Recommended pin check at bump time: `cargo vendor`/`cargo-vet` the 15 crates, and re-run this diff for each new version.
