# Twake Chat (macOS)

Native macOS client for Twake Chat, a Matrix client of the Twake suite.

Stack: Rust and [GPUI](https://crates.io/crates/gpui) (no Electron, no webview).
The rules of the codebase are in [ARCHITECTURE.md](ARCHITECTURE.md); numbered
decisions are in [docs/decisions.md](docs/decisions.md).

## Layout

Cargo workspace. One crate per directory under `crates/`:

- Core (no GPUI, no runtime): `twake-domain`, `twake-app` (use cases and port traits)
- Adapters: `twake-matrix`, `twake-services`, `twake-media`, `twake-store`, `twake-platform`, `twake-calls`
- Design data (dependency-free): `twake-tokens`, `twake-icons`; `twake-i18n`
- UI: `twake-gpui` (the only crate naming GPUI), `twake-ui-kit`, `twake-ui`
- `twake-testing`: fakes, fixtures, contract suites
- `twake-desktop`: the binary and composition root
- `xtask/`: repository tasks (`cargo xtask check-deps | check-size | tokens | icons`, not implemented yet)

## Run

```bash
cargo run -p twake-desktop
```

Checks: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
On Linux, GPUI needs `libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-dev libvulkan-dev libfontconfig1-dev libdbus-1-dev`.
