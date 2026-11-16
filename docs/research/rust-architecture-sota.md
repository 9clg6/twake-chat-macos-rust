# Rust architecture for a native Twake Chat client (GPUI + matrix-sdk-ui): state of the art, 2026-11-12

Scope: macOS first, then Windows and Linux. Stack under study: GPUI, matrix-sdk and matrix-sdk-ui 0.19.1
(sliding sync, Rust crypto, SSO). Goals: modular, fast (RAM, startup, long-timeline scroll), secure.

Legend used on every claim:

- **[V]** verified in this session: I read the source, the manifest or the docs page cited next to it.
- **[V2]** secondhand: read in a search-result summary, not on the page itself.
- **[I]** inference or recollection: not checked here. Treat it as a hypothesis to test in a spike.

---

## 0. The finding that shapes everything: "GPUI" is no longer one thing

- crates.io `gpui` is at 0.2.2, published 2025-10-22, the latest version in the registry. [V] (crates.io API)
- I unpacked `gpui-0.2.2.crate`. It has **no AccessKit** (0 hits), **no wgpu** (0 hits), and a Blade renderer
  (`blade-graphics`, feature `macos-blade`). Platform code is inside the crate (`src/platform/{mac,linux,windows}`). [V]
- Zed's `main` has moved on. `crates/` now holds `gpui`, `gpui_platform`, `gpui_macos`, `gpui_windows`, `gpui_linux`,
  `gpui_wgpu`, `gpui_web`, `gpui_ios`, `gpui_tokio`, `gpui_util`. [V] (GitHub contents API)
  - `gpui_platform` picks the backend per OS; features `wayland`, `x11`, `font-kit`, `test-support`. [V]
  - `gpui`, `gpui_windows` and `gpui_linux` depend on `accesskit`. Linux uses `accesskit_unix`, `gpui_wgpu` and `ashpd`. [V]
  - On crates.io, `gpui_macos` exists (0.1.1). `gpui_platform`, `gpui_windows`, `gpui_linux`, `gpui_tokio` and `gpui_wgpu`
    do **not** exist. [V] (crates.io API)
- The ecosystem has therefore forked:
  - Rivet uses `gpui = "0.2.2"` with `gpui-component 0.5.1`. [V]
  - theGrid uses `gpui-unofficial 1.11` (now 1.28.1 on crates.io, repo `gpui-archipelago/gpui-unofficial`). [V]
  - Twayblade uses `gpui-kit =0.9.1`, which depends on `gpui-pre =0.3.12`, `gpui-fast` and `gpui-pre-platform`. [V]
  - `gpui-component` 0.9.1 and `gpui-kit` 0.9.1 were published 2026-11-09. [V]
- A git dependency on Zed means copying Zed's `[patch.crates-io]`. It patches `async-task`, `calloop`, `async-process`, `notify`,
  `cpal`, and the LiveKit trio `livekit`, `libwebrtc`, `webrtc-sys` (forks in `zed-industries/livekit-rust-sdks`). [V]
  Zed's toolchain is `1.98.1` and its `.cargo/config.toml` sets `--cfg tokio_unstable` and v0 symbol mangling. [V]

**Decision G1 (my recommendation, [I] on the trade-off).**
Make a single crate `twake-gpui` that is the only place naming the GPUI source (`pub use gpui::*` plus the platform bootstrap).
Everything else imports `twake_gpui`. Run a 2-day spike on macOS with a 10k-message list, comparing:
(a) `gpui 0.2.2` (stable, no a11y, Blade), (b) `gpui-unofficial`, (c) Zed git at a pinned rev with copied patches.
Pick (c) or (b) if accessibility or Windows/Linux quality is a requirement, which it is for a "high quality" client.
Pick (a) only for the macOS-only MVP.
Reason: 0.2.2 cannot give screen-reader support, and the fixes land only in Zed `main`.

---

## 1. Cargo workspace layout and build hygiene

### What the references do

| Project | Evidence |
|---|---|
| matklad, "Large Rust Workspaces" | Flat layout wins between 10k and 1M lines: virtual manifest, `members = ["crates/*"]`, one directory per crate, dir name equals crate name. Reasons: the crate namespace is flat, and `ls ./crates` is a bird's-eye view. [V] https://matklad.github.io/2021/08/22/large-rust-workspaces.html |
| Zed | About 250 crates in `crates/`, `resolver = "2"`, `[workspace.lints.clippy]` (`dbg_macro`/`todo` = deny; style lints left off "as it slows down shipping"), dylint custom lints kept out of the workspace on purpose. [V] https://github.com/zed-industries/zed/blob/main/Cargo.toml |
| Zed profiles | `dev`: `split-debuginfo="unpacked"`, `codegen-units=16`, `debug="limited"`, `build-override` mirrored "or cargo compiles ~400 crates twice". Proc-macro crates (`syn`, `proc-macro2`, `gpui_macros`...) at `opt-level=3` even in dev. `release`: `lto="thin"`, `codegen-units=1`, `debug="limited"`. Extra `release-fast` profile (no LTO). [V] same file |
| Zed and workspace-hack | `workspace-hack` appears 0 times in Zed's root `Cargo.toml`. They do not use it. [V] |
| Zed and nextest | `.config/nextest.toml` exists. [V] |
| Twayblade | `resolver = "3"`, edition 2024, `rust-version = "1.99"`, `[workspace.lints.rust] unsafe_code = "forbid"`, dev profile `[profile.dev.package."*"] opt-level = 2` and `debug = 0` so deps are fast and own crates compile fast. Exact-pinned deps (`=0.19.1`). [V] https://github.com/zerw0/twayblade |
| Rivet | 3 crates (`rivet-core`, `rivet-ui`, `rivet-call`), `members = ["crates/*"]`, `resolver = "2"` with edition 2024. [V] |

### Cargo facts [V] (Cargo Book)

- Resolver `"3"` is the edition-2024 default (needs Rust 1.84+). It changes `resolver.incompatible-rust-versions` from `allow`
  to `fallback`, so the resolver prefers dependency versions compatible with your `rust-version`.
  For a virtual workspace set it in `[workspace]`. https://doc.rust-lang.org/cargo/reference/resolver.html
- `[workspace.package]` inherits `version`, `edition`, `license`, `rust-version`, `publish`...
  `[workspace.lints]` plus `[lints] workspace = true` has been respected since Cargo 1.74.
  https://doc.rust-lang.org/cargo/reference/workspaces.html
- `cargo-hakari` (0.9.39) generates a `workspace-hack` crate so that feature unification is the same whichever member you
  build. This avoids rebuilding a dependency with a different feature set. https://docs.rs/cargo-hakari/latest/cargo_hakari/about/index.html [V]
  Whether it pays off for us is [I]: Zed, the biggest GPUI workspace, does not use it.
  Adopt it only if `cargo build -p twake-matrix` then `cargo build -p twake-desktop` shows duplicated rebuilds.

### Recommendations

1. Flat `crates/` directory, about 12-15 crates, `members = ["crates/*", "xtask"]`, `resolver = "3"`, edition 2024. [V sources, I choice]
2. Root `[workspace.lints]`: `unsafe_code = "forbid"`, and clippy `dbg_macro`, `todo`, `unwrap_used`/`expect_used` as `warn`
   (deny in domain/app), `undocumented_unsafe_blocks = "deny"`. The platform crate overrides to `deny` with audited modules,
   the Twayblade way (see its `crates/platform/Cargo.toml` comment). [V pattern]
3. Profiles: copy Zed's `dev` and `release` shape. Add `[profile.dev.package."*"] opt-level = 2` (Twayblade's note: GPUI,
   the Matrix SDK, image decode and crypto are "unusably slow unoptimised" [V]). Add `release-fast` (no LTO) for the perf loop,
   and `split-debuginfo = "unpacked"` on macOS. [V sources]
4. Linker: Zed's config notes `lld` is needed to link `libwebrtc.a` on aarch64-linux. [V] (`.cargo/config.toml` comment)
   `mold` on Linux dev machines and `sccache` or `Swatinem/rust-cache` in CI are standard, but I did not benchmark them here. [I]
5. Test runner: `cargo-nextest` (per-test process isolation, "up to 3x faster than cargo test"). The nexte.st home page
   currently shows a security notice to upgrade to 0.9.150, so pin a version at or above it in CI. [V] https://nexte.st/
   Nextest does not run doctests; keep `cargo test --doc` as a separate step. [I]

---

## 2. GPUI application architecture

### GPUI primitives [V] (gpui 0.2.2 source and docs)

- Everything is owned by `App`. You hold `Entity<T>` handles (Rc-like). `entity.update(cx, |state, cx| {...})` gives `&mut T` plus
  `Context<T>`. `cx.notify()` tells observers and re-renders views; `observe` and `subscribe` react to changes.
  (`src/_ownership_and_data_flow.rs`.)
- `AsyncApp` and `AsyncWindowContext` come from `cx.to_async()`. They are `'static`, can be held across `.await`, and calls on
  them are fallible because the app or window may be gone. https://github.com/zed-industries/zed/blob/main/crates/gpui/docs/contexts.md
- Executors: `cx.spawn` and `cx.spawn_in(window, ...)` run on the **foreground** (main-thread) executor.
  `cx.background_spawn` runs on the GPUI background pool. (`src/executor.rs`, `src/app/context.rs`.)
- Globals (`Global` trait, `cx.set_global`), actions (`#[action]`/`actions!`, `cx.on_action`, `cx.bind_keys`), menus (`cx.set_menus`),
  `on_open_urls`, `prompt_for_paths`, `TestAppContext` (feature `test-support`) all exist in 0.2.2. [V]
- Lists: `list(ListState, render_item)` for variable heights, `uniform_list` for fixed heights. `ListState::new(item_count, alignment, overdraw)`,
  `splice` and `reset` when items change height or count; "elements outside the scrolled area must not change height". [V] (`src/elements/list.rs`)
- Images: `img`, `elements/image_cache.rs` (retain-all cache type exists). [V file names]
- IME: `EntityInputHandler` in `src/input.rs`. [V] A system tray API: `platform/mac/status_item.rs` exists but I did not confirm any public API. [I]

### Bridging GPUI and Tokio (matrix-sdk needs Tokio)

matrix-sdk runs on Tokio (reqwest, background tasks). GPUI's executors are not Tokio. [I, universally known; Rivet, theGrid
and Twayblade all create a Tokio runtime, [V] below]. Three patterns are in the wild:

1. **Zed's `gpui_tokio`** [V]: a `Global` holding a Tokio multi-thread runtime with **2 worker threads** ("two executors, keep
   the footprint small"), or a borrowed `Handle`. `Tokio::spawn(cx, fut)` spawns on Tokio, returns a GPUI `Task`, and **aborts
   the Tokio task if the GPUI task is dropped** (via `defer`). Not on crates.io. theGrid vendored a copy (`gpui_tokio/` with
   `LICENSE_APACHE`). [V]
2. **Rivet**: `tokio::runtime::Builder::new_multi_thread()` in `main.rs`, then `runtime.enter()`, so Tokio calls work inside GPUI
   tasks. UI uses `cx.spawn(...)` and `cx.notify()`. [V] (grep of `crates/rivet-ui/src/main.rs`)
3. **Twayblade**: the `Backend` struct owns `Runtime`. It uses `runtime.spawn(...)` for timeline and thread streams. It has
   `block_on` calls (lines ~638, ~946 of `backend.rs`). The UI calls `cx.background_spawn(async move { job(backend) })`.
   A comment says it uses `watch` channels, which coalesce updates, "so there is no unbounded event queue". [V grep; the
   `block_on` safety is [I]]

**What they got right and wrong** (observations, [V] unless marked)

| | Right | Wrong or risky |
|---|---|---|
| Zed `gpui_tokio` | Cancellation tied to task drop; tiny footprint; one `Global` | Spawn returns `JoinError`-wrapped results, so every call site unwraps two layers; not published |
| Rivet | 3-crate split with `benches/` (`timeline_diff`, `avatar_pipeline`, `room_hierarchy`), `perf.rs` `log_if_slow`, uses `list()` | Its own README says the codebase "sucks beyond compare"; `rivet-ui` holds auth, security, rooms, timeline, settings in one crate; matrix-sdk 0.16 (stale vs 0.19.1); last push 2026-03-18; `tokio features=["full"]` |
| theGrid | 6 crates with RTC split into `thegrid_rtc_livekit` and `thegrid_screen_share`; one CI workflow per OS; `keyring-core`; i18n via Parlance | Git deps on `contemporary-rs`; `cntp_i18n_gen = "*"` (wildcard version); LiveKit with `native-tls` while matrix-sdk 0.19 is rustls-only, so two TLS stacks [I on consequence]; `tokio = full` |
| Twayblade | Pinned `=0.19.1`; `unsafe_code = forbid` outside `platform`; `watch` coalescing; offline `--preview` mode; docs separate "compiles", "runs", "live", "physical" acceptance | `app.rs` is >2,200 lines (line 2228 seen) and `backend.rs` >1,000; `block_on` on a runtime held in the UI-reachable struct; depends on an obscure `gpui-pre` fork via `gpui-kit`; unsigned builds; Linux/Windows acceptance pending (its own DEVELOPMENT.md) |

### Recommended pattern [I, built on the verified pieces]

```
GPUI main thread                 Tokio runtime (owned by twake-matrix)
-----------------                ------------------------------------
Entity<RoomListModel>  <------   RoomListService stream  --(coalesce 16 ms)--> Snapshot / Diff batch
Entity<TimelineModel>  <------   Timeline::subscribe()   --(Vec<VectorDiff>)--> Diff batch
cx.spawn(async { rx.recv().await; model.update(cx, |m, cx| { m.apply(batch); cx.notify() }) })
UI action -> app.use_case(cmd) -> port (async fn) -> Tokio::spawn(...) -> Task<Result<..>>
```

Rules:

1. One Tokio runtime for the whole process, built once in `twake-desktop`, passed as a `Handle` to the adapter (Zed's
   `init_from_handle` shape). 2-4 workers, not `full` defaults.
2. The adapter never touches GPUI types. The UI never touches `matrix_sdk` types. Cross the boundary with plain `Send + 'static`
   snapshots and diffs (domain types).
3. Streams to the UI go through a bounded or `watch`-style channel with coalescing. One GPUI foreground task per subscription
   calls `cx.notify()` once per batch.
4. Never `block_on` from a thread that GPUI owns. Run blocking work with `cx.background_spawn`.
5. Dropping a GPUI `Task` must cancel its Tokio work (copy `gpui_tokio`'s `defer` abort).

### Layering (domain / application / adapters / ui), ports and adapters in Rust

This mirrors the web app's AGENTS.md (screens call `ChatApp` use cases; only `adapters/matrix` imports the SDK). [V from the repo's AGENTS.md]

- `domain`: pure types and rules (read-state computation, room kind, message model). No `async`, no I/O.
- `app` (application): use cases as plain structs or functions over **port traits** (`trait Rooms`, `trait Messages`,
  `trait SecretStore`, `trait Notifier`). Use native `async fn` in traits. `Arc<dyn Port>` is fine at the composition root. [I]
- `adapters`: one crate per external system, each implementing ports.
- `ui`: GPUI views and entities calling use cases only.
- Enforce direction mechanically, not by convention. Options: `cargo deny` `[bans]` with `wrappers` to allow only
  `twake-matrix` to depend on `matrix-sdk*` [I, from memory of cargo-deny's feature; check the book], plus a CI script that
  runs `cargo tree -i matrix-sdk` and fails if anything else is in the path.

---

## 3. matrix-sdk-ui 0.19.1 usage and pitfalls

Verified from docs.rs and the changelogs unless noted.

- Crate description: "GUI-centric utilities on top of matrix-rust-sdk **(experimental)**". Modules: `sync_service`,
  `room_list_service`, `timeline`, `encryption_sync_service`, `notification_client`, `spaces`, `unable_to_decrypt_hook`.
  https://docs.rs/matrix-sdk-ui/latest/matrix_sdk_ui/ [V]
- API churn is real: 0.19.0 removed `EncryptionSyncService::run_fixed_iterations`, removed `Room::load_thread_list` for a new
  `ThreadListService`, and made `rustls` the only TLS backend (`native-tls` removed). https://github.com/matrix-org/matrix-rust-sdk/blob/main/crates/matrix-sdk-ui/CHANGELOG.md [V]
  **Pin exact versions** (`=0.19.1`), as Twayblade does, and wrap all of it in one adapter crate.
- **SyncService** is the "opinionated" unified runner for Room List sync plus Encryption sync. It signals errors via its `state`,
  which "the user MUST observe". After an error or termination you must call `start()` again, or enable
  `SyncServiceBuilder::with_offline_mode`. `stop()` is now infallible. [V] https://docs.rs/matrix-sdk-ui/0.19.1/matrix_sdk_ui/sync_service/index.html
  Pitfall: forgetting to watch `state` leaves a client that silently stopped syncing.
- **RoomListService** has one sliding-sync list, `all_rooms`, starting `Selective` then `Growing` to load the rest in the
  background. `RoomList::entries_with_dynamic_adapters` returns a sorted, filterable stream with a controller whose filter can
  change over time. [V] https://docs.rs/matrix-sdk-ui/0.19.1/matrix_sdk_ui/room_list_service/index.html
  The web app computes unread itself (Synapse sends none with sliding sync, [V] repo AGENTS.md). Check what the SDK's room
  info gives before re-implementing `readState` in Rust. [I]
- **Timeline** `subscribe()` returns `(Vector<Arc<TimelineItem>>, Stream<Vec<VectorDiff<...>>>)`. The stream batches: "no delays, it
  consumes as many updates as possible". Items are `Arc`, state is `imbl::Vector` (structural sharing). Map each diff to
  `ListState::splice` and keep your own view model per item id, not per index. [V for API, I for the splice mapping]
  `paginate_backwards` / `live_back_pagination_status` drive history loading; the focused (permalink) mode returns `None` for
  live status. [V]
- **Stores**: `ClientBuilder::sqlite_store(path, passphrase: Option<&str>)` and `sqlite_store_with_cache_path(path, cache_path, passphrase)`
  separate cache data from state/crypto. Needs feature `sqlite` (`bundled-sqlite` to avoid a system libsqlite). [V]
  A passphrase encrypts the store: always pass `Some(...)` (see section 5). A CodSpeed page shows the SQLite linked-chunk lazy loader at
  about 15 ms for 1,000 events vs about 2.6 ms for the memory store. [V2] Another run shows creating a timeline from 10,000
  initial events at about 800-850 ms. [V2] So: never build an unbounded timeline; page in 30-50 items.
- **Media cache**: Twayblade disables the SDK's disk media cache for encrypted media, decrypts only in memory, keeps its own LRU of
  16,777,216 total pixels (64 MiB RGBA) and 256 entries, four fetches in parallel, one decode semaphore, decoder caps (8,192 px per side, 64 MiB
  allocation, 16 MiB input), 15 s fetch timeout. [V] (PLAN.md lines 87, 459, 542, 614) Copy these numbers as defaults; they come from a real client and
  fit our "no plaintext media on disk" stance. Whether that is acceptable for our product (offline media) is a product decision. [I]
- **SSO**: matrix-sdk feature `sso-login` (theGrid enables it). Rivet and Twayblade implement a loopback server
  (`auth/sso/local_server.rs`, `auth/loopback.rs`). [V file names] For Twake, the Synapse SSO returns a `loginToken` to a
  redirect; a native app can use a `127.0.0.1:port` loopback or the `twake://` scheme. Loopback is simpler on all 3 OSes. [I]
- Notifications while closed: `notification_client` module exists. Push to a desktop client without a window is OS-dependent and
  was not researched here. [I]
- Calls: both Twayblade and Rivet implement MatrixRTC natively (`calls/` modules, `rivet-call`). The web app's AGENTS.md
  states MatrixRTC with LiveKit, own UI, ToM-issued token, Meet for recording and captions. [V repo AGENTS.md]

---

## 4. Performance

### Virtualized timeline [V sources]

- Use `list(ListState, ...)` (variable height). `overdraw` renders extra pixels beyond the viewport. Measure heights once
  (`measure_all` exists) and call `splice` on prepend or edit, `reset` on room switch. (`elements/list.rs`)
- Keep message rows cheap: no per-frame allocation, precomputed text runs, avatars resolved from a cache entity.
- Twayblade ships a 10,000-message virtualization smoke fixture (PLAN.md line 251) and an offline `--preview` mode. Rivet ships
  `benches/timeline_diff.rs`. Copy both ideas: a fixture mode and a diff-application benchmark. [V]

### Avoiding re-render [I, based on the GPUI model above]

- Many small entities (one per visible row model, one for composer, one for room list) instead of one giant `Entity<AppState>`.
  `cx.notify()` re-renders observers of that entity only.
- Coalesce diffs per frame (about 16 ms) before notifying.
- Do not `notify` on no-op diffs (compare ids and edit-version).

### Images and decode [V Twayblade numbers above, rest I]

- Fetch on viewport (cap 128 ids per surface, concurrency 4), decode off the UI thread, thumbnail to display size, LRU by pixel budget.
  `image` crate with `default-features = false` and only needed codecs (Twayblade: png, jpeg, webp, gif). [V]

### Startup [I]

- Show the window from the cached store before sync: restore session from keychain, open the SQLite store, render the room list from
  `RoomListService` state. Lazy-initialize crypto and calls. Measure with `tracing` spans written to a ring buffer.

### Profiling and benchmarking

- `samply` (sampling profiler with Firefox Profiler UI), `dhat` (heap profiling crate), Tracy (frame profiler via `tracing-tracy`),
  `cargo-instruments` on macOS. I did not fetch their docs (samply's README URL 404ed in my fetch), so versions and flags are [I].
- Benchmarks: `divan` (attribute macros, supports allocation counting via a global allocator wrapper per its README; I read only the
  README intro) vs `criterion` (more mature statistics, CI integration). Rivet's `benches/` use one of them. Recommendation: `divan` for
  micro-benchmarks, plus CodSpeed-style CI tracking if desired (the SDK itself tracks benchmarks on CodSpeed [V2]). [I]
- Rivet's `perf::log_if_slow(name, started, threshold, detail)` pattern is cheap and effective: log only slow operations. Copy it. [V]
- Budgets to write down before coding (my proposals, [I]): cold start to first room list under 800 ms on an M1 with a warm store;
  scroll at 120 Hz on 10k messages with no dropped frame; idle RSS under 250 MB with one account and 50 open rooms.

---

## 5. Security and supply chain

### Secrets [V unless marked]

- `keyring` crate is at **4.2.0**. Its docs now say applications that want to choose their credential stores should **not** link
  `keyring` at all, but `keyring-core` plus the specific store crates. `keyring` itself offers modes `v1` (old behaviour) and `cli`. [V]
  https://docs.rs/keyring/4.2.0/keyring/ . theGrid uses `keyring-core = "1"`. Twayblade pins `keyring =3.6.3` with `apple-native`,
  `windows-native`, `sync-secret-service` + `crypto-rust` features. [V]
  Recommendation: `keyring-core 1.x` behind a `SecretStore` port in `twake-store`, with one store crate per OS (names to confirm in the
  keyring-rs wiki, [I]).
- Linux needs an **unlocked Secret Service provider**; Twayblade states a headless DBus alone does not provide one. Fail with a clear error;
  do not fall back to plaintext (Twayblade: "no plaintext credential fallback"). [V]
- macOS: ad-hoc signed rebuilds change the code requirement, so Keychain re-prompts at each rebuild. Twayblade ships
  `macos/create-dev-certificate.sh` to make a stable self-signed identity. [V] Copy that for developers.
- Pattern: generate a random 32-byte store passphrase at first login, keep it **only** in the OS keychain, pass it to
  `sqlite_store(..., Some(passphrase))`, and zeroize in memory (`zeroize` crate, used by Twayblade). Keep the Matrix session
  (access token, device id, user id) in the keychain too, so the device id is reused and E2EE keys survive. [I, mirrors the web app's rule that the device id is kept, [V] AGENTS.md]
- The recovery key is never stored; held in memory for one verification only. [V repo AGENTS.md] Keep that rule.

### Supply chain

- `cargo-deny` (licenses, bans, advisories, sources) in CI with a `deny.toml`; `cargo-audit` for RustSec advisories; `cargo-vet`
  (Mozilla) if we want recorded human audits of each dependency (heavy; the dependency tree with GPUI and matrix-sdk is thousands of crates). [V that the tools do this: https://embarkstudios.github.io/cargo-deny/ and https://mozilla.github.io/cargo-vet/. The adoption order is my proposal, [I]: deny + audit now, vet only for the crypto, auth and update path.]
- Commit `Cargo.lock`, build with `--locked` (Twayblade does). [V] Pin git dependencies by `rev`. If we use Zed git, keep a script that diffs Zed's
  `[patch.crates-io]` on every bump. [I]
- `unsafe`: `unsafe_code = "forbid"` at the workspace, `deny` only in `twake-platform` with `// SAFETY:` comments (Twayblade's exact policy). [V]
- No Matrix access token to call services; LiveKit JWT and OpenID tokens only in memory (Twayblade AGENTS.md; same as our web rules). [V]
- Update path is a security boundary: signed manifests, HTTPS, key pinned in the binary. See packaging.

### Signing, packaging, auto-update

| OS | Package | Tool | Status |
|---|---|---|---|
| macOS | `.app` in a signed `.dmg`, Developer ID, hardened runtime, notarized (`notarytool`), stapled | `cargo-packager` (DMG, `.app`) or own `xtask`; updater **Sparkle** (EdDSA-signed appcast) | Apple's notarization page loaded empty in my fetch, so exact steps are [I]. Sparkle docs fetched: https://sparkle-project.org/documentation/ [V]. Sparkle from Rust needs an Obj-C bridge; unproven here. [I] |
| Windows | NSIS `.exe` or WiX `.msi`; MSIX for Store | `cargo-packager` lists NSIS and MSI via WiX [V]; MSIX tooling not checked [I]; Authenticode or Azure Trusted Signing [I] | |
| Linux | `.deb`, AppImage, pacman via `cargo-packager` [V]; Flatpak as a second channel [I] | | Flatpak needs portal permissions for secret service, notifications [I] |
| All | `cargo-packager` also ships an updater crate (`cargo-packager-updater`) [V]; `velopack` crate 1.2.161 published 2026-10-17 [V crates.io], its Rust support level not verified [I] | | |

cargo-packager 0.11.8 dates from 2025-11-27 (almost a year old). [V crates.io] Check its macOS notarization support before depending on it. [I]
Twayblade's builds are unsigned, with a README saying "right-click Open". [V] We must not ship that.

---

## 6. Cross-platform status (November 2026)

| Topic | macOS | Windows | Linux |
|---|---|---|---|
| Zed support | Full | Stable "fully supported" since the 2025 blog; the blog says DirectX 11 and DirectWrite [V] https://zed.dev/blog/zed-for-windows-is-here. Zed `main` today has `gpui_windows` with AccessKit; its renderer status was not re-checked [I] | Wayland and X11; **needs Vulkan**; failures like `NoSupportedDeviceFound` documented [V] https://zed.dev/docs/linux |
| GPUI 0.2.2 (crates.io) | Blade/Metal, no AccessKit [V] | in crate, no AccessKit [V] | Blade, wayland/x11 features, no AccessKit [V] |
| Accessibility (Zed `main`) | AccessKit; open issue #66691 (window reported as focused element, AccessKit subclasses the content view) [V] | AccessKit; older issue #41138 "Windows: Screen reader accessibility missing completely" is open [V] | `accesskit_unix`; open #66729 (X11: no window screen position) [V] |
| Cross-cutting a11y bugs | #66730 (clipped elements reported as visible), #66690 (cached views drop their a11y nodes) are open [V] | | |

Accessibility verdict: **real but immature**. On Zed `main`, GPUI integrates AccessKit on all three OSes, and Zed's tracker has four
open a11y bugs filed on 2026-11-10/11. The crates.io release has none. [V] Budget time for upstream fixes. Test with VoiceOver first.

### Platform features and crates

- **Native menus**: `cx.set_menus` is in gpui (macOS native menu bar, dock menu `set_dock_menu`). [V in source] On Windows and Linux there is no native
  menu bar in GPUI; `muda` supports Windows/macOS/Linux (GTK 3/4), but on Windows its accelerators need `TranslateAcceleratorW` in the
  message loop, which GPUI owns. [V muda README] Plan: custom in-window menu on Windows/Linux. [I]
- **Notifications**:
  - macOS: `UNUserNotificationCenter` (needs a signed app bundle; Twayblade uses `mac-usernotifications 0.3.1` and `objc2-user-notifications`). [V manifest]
  - Windows: Twayblade uses the `windows` crate `UI_Notifications` (toasts). [V manifest]
  - Linux: `ashpd`/`zbus` (XDG portals, D-Bus). Twayblade notes `ashpd` must use `async-io` because GPUI's Linux backend enables it and `ashpd` refuses both runtimes. [V manifest comment]
  - `notify-rust` is the simpler cross-platform option: supports actions, close callbacks, `wait_for_response()`, inline replies on its two macOS backends. [V README] https://github.com/hoodie/notify-rust
  Recommendation: `Notifier` port, with `notify-rust` as the first adapter and native adapters where `notify-rust` falls short (macOS reply and click-to-room). [I]
- **Deep links `twake://`**: gpui has `on_open_urls`. [V] You must also register the scheme per OS: `CFBundleURLTypes` in Info.plist; registry or MSIX
  protocol on Windows; `.desktop` `x-scheme-handler/twake` on Linux; plus single-instance forwarding (Twayblade refuses a second instance on macOS [V]). [I for registration details]
- **Tray**: `tray-icon` supports Windows, macOS, Linux (AppIndicator or KSNI); on Windows and AppIndicator it needs an event loop on the same thread. [V README]
  Interaction with GPUI's own run loop is unproven. Treat tray as phase 2. [I]
- **File dialogs**: `prompt_for_paths` in gpui; `rfd` is the standard alternative (crate exists, README fetched). [V]
- **IME**: `EntityInputHandler` exists in gpui. Test CJK and dead keys on each OS; this is the single most common GPUI text-input bug source. [I] (Clément's keyboard is US-International, so dead keys
  are an existing pain point per project memory.)
- **Calls**: `livekit` Rust SDK supports Windows, macOS, Linux (plus iOS, Android); README lists hardware encode/decode on macOS (VideoToolbox) and
  NVIDIA/AMD on Linux, and "Adaptive Streaming" unchecked. [V] https://github.com/livekit/rust-sdks
  Crate `livekit` is 0.10.1 (2026-11-04); Rivet uses 0.7.29, theGrid 0.7. [V] Zed patches `livekit`, `libwebrtc`, `webrtc-sys` from a fork. [V]
  Twayblade documents that the LiveKit build downloads a prebuilt libwebrtc and has per-OS acceptance pending. [V]
  The Twake Meet web iframe is **not available natively**; use the same split as the web app: ToM issues the LiveKit token, our own call UI renders media, Meet's
  REST side (recording, captions, raised hand) is called with the LiveKit token. [I; web architecture [V] from repo AGENTS.md]
  Put calls behind a `CallMedia` port in its own crate so the libwebrtc build cost is optional (`--features calls`).
- **i18n**: `rust-i18n` 4.2.4 (2026-09-29; Rivet uses 3) is simple (macro, YAML or JSON locale files). `fluent` 0.17.0 (2025-05-23) handles plurals and gender per locale properly. [V versions]
  The web app has 7 locales (en, fr, de, es, it, ru, vi) with Russian plural rules [V from git status of locale files]. Recommendation: Fluent (`fluent-bundle`) behind a `Translator` trait. It is more work than
  `rust-i18n`; revisit if plurals are rare in the UI. [I]
- **Testing**: `#[gpui::test]` / `TestAppContext` (feature `test-support`, used by `gpui-kit`'s `capture` feature in Twayblade) for view logic;
  `insta` for snapshot of view models (not pixels); `proptest` for domain (read state, diff application); `cargo-mutants` for the domain and app
  crates only (its book is fetched, https://mutants.rs/ [V]). A fake `MatrixPort` in `twake-testing` must follow the adapter, with a contract suite run on both
  (same rule as the web repo's port contracts, [V] AGENTS.md D446). Real-Synapse integration runs reuse the existing e2e stack. [I on integration]
- **CI matrix**: Twayblade's `check.yml` runs `ubuntu-24.04`, `macos-15`, `windows-2022` with `cargo fmt --check`, `clippy --locked --workspace --all-targets --all-features -D warnings`,
  `cargo test --locked --workspace --all-features`, `cargo build --locked -p twayblade`; its `build.yml` produces per-OS artifacts. [V] Linux needs
  `libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libx11-dev libvulkan-dev libfontconfig1-dev libdbus-1-dev` and similar (list in DEVELOPMENT.md). [V]
  Add: nextest, cargo-deny, a headless GPUI test job on Linux (xvfb) [I], nightly E2E against Synapse on macOS and Linux only at first.

---

## 7. Recommended crate list and dependency rules

Flat `crates/` directory, prefix `twake-`. Arrows mean "may depend on". Anything not listed is forbidden.

| Crate | Responsibility | May depend on | Notes |
|---|---|---|---|
| `twake-domain` | Pure types and rules: ids, `RoomSummary`, `MessageVm`, read state, power levels, search query. | `std`, `serde`, `thiserror`, `imbl` | No async, no I/O. Fully property-tested. `forbid(unsafe)` |
| `twake-app` | Use cases and **port traits** (`Rooms`, `Messages`, `Session`, `Encryption`, `Media`, `SecretStore`, `Notifier`, `CallMedia`, `Analytics`). | `twake-domain` | Mirrors web `ChatApp` groups (`rooms`, `messages`, `threads`, `encryption`...). Native `async fn` in traits |
| `twake-matrix` | Adapter: only crate that depends on `matrix-sdk*`. `SyncService`, `RoomListService`, timeline, crypto, SQLite store, SSO login. Owns nothing GPUI. Produces domain snapshots and diffs. | `twake-app`, `twake-domain`, `matrix-sdk`, `matrix-sdk-ui`, `tokio` | Exact-pinned SDK versions |
| `twake-services` | HTTP adapters of Twake services: ToM, side service (assistant, suggestions, analytics), Meet REST, GIF proxy. | `twake-app`, `twake-domain`, `reqwest` (rustls) | One TLS stack for the whole app |
| `twake-media` | Fetch scheduling, decode, thumbnailing, pixel-budget LRU, waveform. | `twake-domain`, `image` | Numbers from section 3 |
| `twake-store` | `SecretStore` adapter (`keyring-core` + per-OS stores), preferences file, passphrase generation, zeroize. | `twake-app`, `keyring-core`, `zeroize` | |
| `twake-platform` | OS integration: notifications, deep links, single instance, tray, accent colour, badge count. `cfg(target_os)` modules. | `twake-app` | `unsafe_code = "deny"` with audited modules; the only crate allowed `unsafe` |
| `twake-calls` | MatrixRTC signalling plus LiveKit media adapter. | `twake-app`, `twake-domain`, `livekit` | Optional feature; heavy libwebrtc build |
| `twake-i18n` | `Translator`, Fluent bundles, locale detection. | `twake-domain` | |
| `twake-gpui` | Single facade over the chosen GPUI source plus `gpui_tokio`-style runtime glue. | `gpui*` | Swap point for G1 |
| `twake-ui-kit` | Design system: theme tokens (light/dark), icon button with tooltip, list rows, avatar, text input. No Matrix, no use cases. | `twake-gpui`, `twake-domain` | Build from Figma tokens; keep the "icon buttons get tooltips" rule |
| `twake-ui` | Screens and entities: room list, timeline, composer, room info, settings, onboarding. Calls use cases only. | `twake-ui-kit`, `twake-app`, `twake-domain`, `twake-i18n` | Split into a second crate (`twake-ui-timeline`) once it passes ~15k lines to keep incremental builds short |
| `twake-testing` | Fakes for every port, fixtures (10k-message room), contract suites, builders. | `twake-app`, `twake-domain` | dev-dependency only |
| `twake-desktop` | **Composition root** binary: builds the Tokio runtime, wires adapters to ports, starts GPUI, handles CLI (`--preview`). | everything | The only place that knows all crates |
| `xtask` | Packaging, notarization, release, `cargo xtask check-deps` (dependency-direction guard). | `cargo_metadata` | |

Dependency rules, enforced in CI:

1. `twake-domain` and `twake-app` depend on no adapter, no GPUI, no Tokio runtime features (only `futures`/`tokio::sync` types if needed).
2. `matrix-sdk*` appears only under `twake-matrix`. `gpui*` appears only under `twake-gpui`. `livekit` only under `twake-calls`. `keyring*` only under `twake-store`.
3. `twake-ui*` never imports an adapter crate. Only `twake-desktop` imports adapters.
4. Cycles are impossible by construction (ports live in `twake-app`, which depends only on `twake-domain`).
5. Per-crate quality gate, ratchet style like the web repo's (`complexity` 10, 300 lines per file [V repo AGENTS.md]): clippy `cognitive_complexity`, `too_many_lines`,
   plus a file-size check in `xtask`. Twayblade's 2,000+ line `app.rs` is the warning example. [V]

### Top 10 risks and mitigations

| # | Risk | Mitigation |
|---|---|---|
| 1 | **GPUI source fragmentation**: crates.io 0.2.2 is stale and lacks AccessKit; the newer split crates are unpublished; forks vary. [V] | `twake-gpui` facade; 2-day spike (G1); pin git rev or fork version; track Zed `main` breaking changes monthly; budget a person-week per bump. |
| 2 | **Accessibility** gaps in GPUI (VoiceOver, NVDA, Orca): four open a11y bugs, Windows issue #41138. [V] | Treat a11y as a milestone with VoiceOver tests; contribute fixes upstream; keep every action reachable by keyboard; do not promise Windows/Linux screen-reader parity at launch. |
| 3 | **matrix-sdk-ui is "experimental"** and breaks between minors (0.19.0 removed several APIs). [V] | Exact pins; one adapter crate; contract test suite run against a real Synapse (reuse the e2e stack); upgrade in a dedicated PR with changelog read. |
| 4 | **Tokio and GPUI executors mixed badly**: deadlocks from `block_on`, unbounded queues, leaked tasks. [V patterns, I on failure modes] | Section 2 rules; `gpui_tokio`-style cancel-on-drop; coalescing channels; a lint banning `block_on` outside `twake-desktop`. |
| 5 | **Long-timeline performance and RAM**: 10k-event timeline creation about 800 ms, SQLite loader slower than memory [V2]; unbounded image memory. | Paginate 30-50 items; virtualized `list`; pixel-budget LRU (64 MiB default); `dhat` + `samply` budgets in CI; benches for diff application. |
| 6 | **Cross-platform calls**: libwebrtc build size and time, per-OS device and permission issues, TLS stack mixing (livekit `native-tls` vs rustls). [V partly, I on TLS] | Calls in an optional crate and feature; start macOS only; use the `rustls` features of livekit if available (check); physical acceptance per OS as Twayblade documents. |
| 7 | **Secret handling**: keychain prompts after rebuilds, no Secret Service on some Linux setups, `keyring` v4 API change. [V] | `SecretStore` port; stable dev signing identity; clear error, no plaintext fallback; start from `keyring-core`; encrypt SQLite store with a keychain-held passphrase. |
| 8 | **Distribution trust**: unsigned builds, update channel compromise, `cargo-packager` aging (last release 2025-11-27). [V] | Developer ID + notarization and Authenticode from the first beta; Sparkle EdDSA signing on macOS; manifest signature checked in-app; `cargo-deny`/`cargo-audit` gates; `cargo-vet` for auth and update crates. |
| 9 | **Windows/Linux parity**: Vulkan required on Linux; no native menu; tray and event loop conflicts; deep-link registration differs per OS. [V partly] | macOS GA first; CI builds all 3 OSes from day one (compile gate); a "physical acceptance" checklist per OS kept separate from "it compiles"; custom in-window menu for Windows/Linux. |
| 10 | **Scope and complexity creep in the UI crate** (Twayblade's 2k-line files, Rivet's single UI crate). [V] | File and complexity ratchet; split `twake-ui` by feature at 15k lines; offline `--preview` mode and fixtures so UI work does not need a live account; every feature needs a port fake and a contract test. |

---

## 8. Open questions to settle in a spike (not answered by this research)

1. Which GPUI source (G1): measure build time, binary size, scroll smoothness and a VoiceOver pass for options (a), (b), (c).
2. Does `SyncService` + `RoomListService` give unread counts and notification counts good enough that the Rust client need not
   recompute read state like the web client does?
3. Exact keyring store crates and features for `keyring-core` on each OS (read the keyring-rs wiki).
4. Is Sparkle usable from a Rust GPUI app, or is `cargo-packager-updater` / Velopack the better fit for all three OSes?
5. Can `livekit` 0.10 be built with rustls only, and what does its prebuilt libwebrtc cost in CI minutes and bundle size?
6. Tray and native menu behaviour next to GPUI's run loop on Windows and Linux.

## Sources actually opened in this session

- https://matklad.github.io/2021/08/22/large-rust-workspaces.html
- https://doc.rust-lang.org/cargo/reference/resolver.html and /workspaces.html
- https://docs.rs/cargo-hakari/latest/cargo_hakari/about/index.html
- https://nexte.st/ ; https://mutants.rs/ ; https://embarkstudios.github.io/cargo-deny/ ; https://mozilla.github.io/cargo-vet/
- https://github.com/zed-industries/zed (Cargo.toml, `.cargo/config.toml`, `crates/gpui*`, `crates/gpui_tokio`, `crates/gpui/docs/contexts.md`, issues #66730 #66691 #66729 #66690 #41138)
- gpui 0.2.2 source from https://static.crates.io/crates/gpui/gpui-0.2.2.crate
- https://github.com/Mjoyufull/Rivet ; https://github.com/theCheeseboard/theGrid ; https://github.com/zerw0/twayblade
- https://docs.rs/matrix-sdk-ui/0.19.1/matrix_sdk_ui/ (`sync_service`, `room_list_service`, `timeline::Timeline`) ; https://docs.rs/matrix-sdk/0.19.1/matrix_sdk/struct.ClientBuilder.html
- matrix-rust-sdk CHANGELOGs for `matrix-sdk-ui` and `matrix-sdk`
- https://docs.rs/keyring/4.2.0/keyring/ ; https://github.com/open-source-cooperative/keyring-rs
- https://github.com/livekit/rust-sdks ; https://github.com/crabnebula-dev/cargo-packager ; https://github.com/velopack/velopack ; https://sparkle-project.org/documentation/
- https://github.com/hoodie/notify-rust ; https://github.com/tauri-apps/tray-icon ; https://github.com/tauri-apps/muda ; https://github.com/PolyMeilex/rfd
- https://zed.dev/blog/zed-for-windows-is-here ; https://zed.dev/docs/linux
- crates.io API for version numbers and dependency lists
- Web-search summaries (V2 items only): CodSpeed pages for matrix-rust-sdk benchmarks.
