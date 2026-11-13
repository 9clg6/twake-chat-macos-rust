# Re-diff `gpui-unofficial` at every version bump

Why: `gpui-unofficial` is a repackaging of Zed's `crates/gpui*` by a third party (A022, A028). A bump is
accepted only when the 15 crates the macOS build pulls still equal the Zed tag, apart from the known
differences below. Derived from `gpui-unofficial-audit.md` (2026-11-12, 1.28.1); redo it for each new version
`X.Y.Z` before touching `crates/twake-gpui/Cargo.toml`. Work in a scratch directory, never in the repository.

## 1. Fetch the tag and the crates

```sh
V=X.Y.Z
git clone --depth 1 --branch v$V https://github.com/zed-industries/zed zed
git -C zed rev-parse HEAD        # must equal the commit in the README of the crates (generated from zed vX.Y.Z (<sha>))
```

Resolve the crate list from our own lockfile, not from memory: after the bump in a copy of the workspace,
`cargo tree --target aarch64-apple-darwin -e normal | grep gpui-unofficial` plus the non-prefixed names
(`collections`, `derive-refineable`, `refineable`, `scheduler`, `sum-tree`, `http-client`, `ztracing`, `ztracing-macro`,
`gpui-util`, `gpui-shared-string`, `gpui-macros`, `gpui-apple`, `gpui-macos`, `gpui-platform`, `gpui`), all with the
`-gpui-unofficial` suffix. For each crate `N`:

```sh
curl -sL -A "audit" -o N.crate https://crates.io/api/v1/crates/N-gpui-unofficial/$V/download
mkdir x/N && tar -xzf N.crate -C x/N --strip-components=1
diff -rq --exclude=Cargo.toml --exclude=Cargo.toml.orig --exclude=.cargo_vcs_info.json --exclude=Cargo.lock \
  x/N zed/crates/<zed directory of N>
```

(`gpui-unofficial` is `crates/gpui`, `gpui-macos` is `crates/gpui_macos`, `gpui-apple` is `crates/gpui_apple`,
`gpui-platform` is `crates/gpui_platform`, `derive-refineable` is `crates/refineable/derive_refineable`, the others
keep their Zed name with `_` for `-`.)

## 2. Accept only the known differences

For every differing file, `diff` it and check that it is one of:

1. the `inspector` cfg: `cfg(any(feature = "inspector", debug_assertions))` became `cfg(debug_assertions)`
   (`gpui`: `app.rs element.rs elements/div.rs inspector.rs styled.rs window.rs`; `gpui-macros`). Check
   mechanically: after `sed 's/any(feature = "inspector", debug_assertions)/debug_assertions/'` on the Zed side the
   files are byte-identical;
2. `include_bytes!` / `build.rs` paths adjusted for a self-contained crate (`../../../assets/fonts/...` to
   `../assets/fonts/...`, `.join("../gpui")` to `.join("gpui")`), and the files copied there (fonts, `gpui/src/*.rs`
   read by cbindgen) byte-identical to Zed's;
3. wgpu API adaptation (`gpui-wgpu`, Linux only, not compiled on macOS);
4. a README, `LICENSE-APACHE`, `Cargo.toml.orig`, `Cargo.lock`.

Anything else is a finding: new `.rs` code, a new `unsafe`, `Command`, network or env access, a new `build.rs`
line, a new dependency name. Stop the bump and report it.

## 3. Manifests and build scripts

- Dependency names: compare `Cargo.toml.orig` of each crate with the Zed `Cargo.toml` (names only, `tomllib`).
  No new name is accepted without a line in A028 or its successor.
- The four `build.rs` (`gpui`, `gpui-apple`, `ztracing`, `gpui-windows`) must still be identical to Zed's apart
  from the path line. `gpui-apple` runs `cbindgen` and `xcrun metal`/`xcrun metallib`: nothing else may spawn.
- Check that no manifest has `git =` or a `path =` outside the crate's own targets.
- Zed's `[patch.crates-io]` forks do not apply to the published crates (`async-task` resolves to crates.io):
  list the patches of the new tag's root `Cargo.toml` and note any new one.

## 4. Publisher

`curl https://crates.io/api/v1/crates/gpui-unofficial` and `/owners`: same owners (`iamnbutler`, `Vanuan`, team
`github:gpui-archipelago:gpui-unofficial-maintainers`)? Any new owner, a yanked version, or a publish that is not
by `iamnbutler` is a finding. Record the publication date: a version younger than a few days is waited on.

## 5. Our own checks after the bump

1. `cargo build`; migrate the API changes behind `twake-gpui` only where possible (A010).
2. `cargo deny check`: drop the A018 ignores reported as "not encountered", justify each new one (never a
   vulnerability); list new licences for the owner, do not allow them silently.
3. `TWAKE_PREVIEW_BENCH=1 twake-desktop --preview` (frame p50 / p95, 3 runs, quiet machine: check `uptime`
   first), release binary size, and window-only captures of `--preview` and `--gallery` against the previous
   build (text must be visible; compare pixel by pixel).
4. Accessibility: re-read `docs/dev/gpui-a11y.md` and the upstream issues it lists.
5. Update A028 (or add a decision) with the version, the date and what changed.
