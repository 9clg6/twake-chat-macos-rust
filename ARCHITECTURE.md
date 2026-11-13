# ARCHITECTURE

Status: **normative**. Every rule below is binding for all code in this repository.
A rule is changed only by a numbered decision in `docs/decisions.md` (section 14) merged
*before* the code that needs it. "MUST" and "MUST NOT" are checked by CI where a check is
named; where none is named, the reviewer checks it and a check is added.

Traceability: every FR and NFR of `docs/prd-*.md` maps to a crate and a section here in
`docs/traceability.md` (written with the workspace skeleton, checked by `xtask`).

Inputs: `docs/product-brief-*.md`, `docs/prd-*.md`, `docs/spec/feature-inventory.md`,
`docs/spec/design-system.md`, `docs/research/rust-architecture-sota.md` (claims there are
marked verified / secondhand / inferred; this file only relies on the verified ones, and
lists the rest as spikes in section 13).

## 1. Goals, in priority order

1. **Correct and safe**: no lost messages, no leaked keys, no plaintext secrets.
2. **Fast and light**: start, scroll and idle memory beat the React app in a browser tab.
3. **Same product**: feature and design parity with the React app (`docs/spec/`).
4. **Portable**: macOS first, but nothing in the shared crates may assume macOS.
5. **Modular**: small crates with one reason to change, dependency direction checked by machine.

## 2. Principles

- **P1 Ports and adapters.** The core defines what it needs (traits); adapters implement it.
  The core never names Matrix, GPUI, Tokio runtime, keychain or LiveKit.
- **P2 One external system, one crate.** `matrix-sdk*` is only in `twake-matrix`, `gpui*` only in
  `twake-gpui`, `livekit` only in `twake-calls`, `keyring*` only in `twake-store`,
  `reqwest` only in `twake-services`.
- **P3 Data in, data out.** Layers talk with plain owned domain types (`Send + 'static`), never
  with SDK or UI handles.
- **P4 Parse at the boundary.** Outside data (network, disk, OS) becomes a domain type once, where
  it enters. Inside, trust it. Make illegal states unrepresentable (enums, newtypes).
- **P5 No hidden global state.** State lives in named GPUI entities or in the composition root.
  `static` mutable state and `lazy` singletons are forbidden outside `twake-desktop`.
- **P6 Tokens, not values.** Colours, sizes, radii and type come from `twake-tokens`. No literal
  hex or pixel value in `twake-ui*` (except layout maths on tokens).
- **P7 Measure, then optimise.** Every performance claim has a benchmark or a trace in the PR.
- **P8 Boring first.** Standard library and already-used dependencies before a new crate.
  A new dependency needs a line in `docs/decisions.md` (licence, size, maintenance, why not std).

## 3. Layers and crates

Flat `crates/` directory, one directory per crate, directory name = crate name.
The table governs **internal edges** (which workspace crate may depend on which) and the
**exclusive external systems** of P2. Other third-party crates (`serde`, `futures`,
`async-trait`, OS-specific crates, Fluent, store backends…) are free under P8 and the
supply-chain gates, provided they do not break a hard rule below.
Arrow = "MAY depend on". An internal edge not listed is forbidden.

```
                      twake-desktop  (composition root, binary)
                              |
        +---------+-----------+-----------+-----------+---------+
        |         |           |           |           |         |
  twake-matrix twake-services twake-media twake-store twake-platform twake-calls
        \         |           |           |           |         /
         +--------+-----+-----+-----------+-----------+--------+
                        |
                    twake-app   (use cases + port traits)
                        |
                   twake-domain (pure types and rules)

  UI side:   twake-ui  ->  twake-ui-kit  ->  twake-gpui          twake-i18n
                |               |
                |               +->  twake-tokens, twake-icons (pure data, no GPUI)
                +-> twake-app, twake-domain, twake-i18n
```

| Crate | Responsibility | May depend on |
|---|---|---|
| `twake-domain` | Ids, `RoomSummary`, `MessageVm`, read state, power levels, search query, errors. Pure. | std, `serde`, `serde_json`, `url`, `unicode-normalization`, `thiserror`, `imbl` |
| `twake-app` | Use cases grouped as the web `ChatApp` (`rooms`, `messages`, `threads`, `encryption`, `calls`…) and the **port traits**. | `twake-domain` |
| `twake-matrix` | Adapter over `matrix-sdk` / `matrix-sdk-ui`: sync, room list, timeline, crypto, SQLite store, SSO login. Produces domain snapshots and diffs. | `twake-app`, `twake-domain`, `matrix-sdk*`, `futures`, `thiserror`, `tokio` (`sync`), `rustls` (`aws_lc_rs`); dev: `wiremock`, `tempfile`, `serde_json` |
| `twake-services` | HTTP adapters: ToM, side service (assistants, suggestions, analytics), Meet REST, GIF proxy. | `twake-app`, `twake-domain`, `reqwest` (rustls) |
| `twake-media` | Scheduling, decode off-thread, thumbnails, pixel-budget cache, waveform. Gets bytes through the `Media` port (`twake-app`); never does HTTP itself. | `twake-app`, `twake-domain`, `image` |
| `twake-store` | `SecretStore` over `keyring-core`, preferences file, store-passphrase generation. | `twake-app`, `twake-domain`, `futures`, `thiserror`, `keyring-core`, `zeroize`; dev: `twake-testing` |
| `twake-platform` | Notifications, deep links, single instance, tray, badge, accent colour. `cfg(target_os)` modules. | `twake-app`, `twake-domain` |
| `twake-calls` | MatrixRTC signalling + LiveKit media. Optional (`calls` feature). | `twake-app`, `twake-domain`, `livekit` |
| `twake-i18n` | `Translator`, Fluent bundles, locale detection, plural rules. | `twake-domain` |
| `twake-tokens` | Design tokens (colour schemes, type scale, radii, shadows, motion, breakpoints). **Generated** from `docs/spec/tokens.json`. No GPUI. | none (`serde` behind an optional feature) |
| `twake-icons` | The twake-icons SVG set as static data + lookup. **Generated**. No GPUI. | none |
| `twake-gpui` | The only crate naming the GPUI source; re-exports it; runtime glue (section 5). | `gpui*`, `tokio` |
| `twake-ui-kit` | Design-system components mirroring `@linagora/twake-mui` and `ds/` (Button, IconAction with tooltip, Avatar, Chip, ListRow, TextInput…). No Matrix, no use cases. | `twake-gpui`, `twake-tokens`, `twake-icons`, `twake-domain` |
| `twake-ui` | Screens and entities (room list, timeline, composer, room info, settings, onboarding). Calls use cases only. | `twake-ui-kit`, `twake-gpui`, `twake-app`, `twake-domain`, `twake-i18n` |
| `twake-testing` | Fakes of every port, fixtures (10k-message room), builders, **contract suites**. dev-dependency only. | `twake-app`, `twake-domain` |
| `twake-desktop` | Composition root binary: runtime, wiring, CLI (`--preview`, `--profile`). | all |
| `xtask` | `check-deps`, `check-size`, `tokens`, `icons`, `package`, `notarize`, `release`. | `cargo_metadata` |

Hard rules (checked by `cargo xtask check-deps` in CI, plus `cargo-deny` `[bans]` wrappers):

1. `twake-domain` and `twake-app` MUST NOT depend on an adapter, on GPUI, or on `tokio` at all
   (only `futures`; decision A011/Q6).
2. `twake-ui*` MUST NOT depend on an adapter crate. Only `twake-desktop` imports adapters.
3. `twake-tokens` and `twake-icons` MUST stay GPUI-free and free of dependencies (optional `serde` feature only; A011/Q7) so another Twake
   Rust app (or UI toolkit) can reuse them.
4. No dependency cycles; ports live in `twake-app` so cycles cannot form.
5. `unsafe` is forbidden everywhere except `twake-platform` (section 9).

### Design-system extraction (alignment with twake-mui)

- `twake-tokens` is generated by `cargo xtask tokens` from `docs/spec/tokens.json`, itself
  produced by `docs/spec/export-tokens.mjs` from `@linagora/twake-mui` (version recorded in the
  generated file header). Nobody edits the generated file; a change of token = rerun the export.
- `twake-icons` is generated the same way from `@linagora/twake-icons`.
- `twake-ui-kit` components are 1:1 with `ds/` components, same names, same props where Rust
  allows, same states and sizes (`docs/spec/design-system.md`). Gaps in twake-mui are reported
  upstream, not patched here (no overrides of what twake-mui already provides).
- When a second Twake Rust app needs them, `twake-tokens`, `twake-icons` and `twake-ui-kit`
  move to their own repository and are consumed by version (second-app rule). Until then they
  stay here, and their `Cargo.toml` already has no path to the app crates.

## 4. Domain and use-case conventions

- Domain types are immutable values (`Clone`, `Eq` where meaningful). Collections that change
  incrementally use `imbl` (structural sharing) so snapshots are cheap.
- Ids are newtypes (`RoomId`, `EventId`, `UserId`), never bare `String`.
- Ports are `trait`s whose async methods return **boxed `Send` futures** (`#[async_trait]` or an
  explicit `BoxFuture`), because the UI holds them as `Arc<dyn Port + Send + Sync>` and native
  `async fn` in traits is **not dyn-compatible** (checked on rustc 1.99.1: error E0038; a method
  returning `Pin<Box<dyn Future + Send>>` compiles behind `Arc<dyn …>`). Decision A009. Grouped by topic like the web `ChatApp`
  (`Rooms`, `Messages`, `Threads`, `Encryption`, `Session`, `SessionStorePort` (stored sessions, up to 5 accounts), `Media`, `SecretStore`,
  `Notifier`, `CallMedia`, `Analytics`). A port returns domain types and `Result<_, DomainError>`.
- Errors: `thiserror` enums per port; no `anyhow` in libraries; `anyhow` only in `twake-desktop`
  and `xtask`. No `unwrap`/`expect`/`panic!` in non-test code of any crate except
  `twake-desktop` start-up (clippy deny).
- Streams to the UI are `Stream<Item = Batch>` where `Batch` = a snapshot or a `Vec<Diff>`.
- Every port has a **fake** in `twake-testing` and a **contract suite** run on both the fake
  and the real adapter. A bug found because fake and adapter disagree adds a contract case,
  and the fake follows the adapter.
- Feature parity is tracked by inventory ids: a use case or screen cites the `Fxxx` / `Dnn` it
  implements in its module doc.

## 5. Runtime model (GPUI + Tokio)

```
GPUI main thread                       Tokio runtime (built in twake-desktop)
Entity<RoomListModel>  <-- batch ---   twake-matrix: RoomListService stream
Entity<TimelineModel>  <-- batch ---   twake-matrix: Timeline::subscribe diffs
UI action -> use case -> port (async) -> spawned on Tokio -> Task<Result<..>>
```

1. **One** Tokio multi-thread runtime for the process, 2 to 4 workers, created in
   `twake-desktop` and handed to adapters as a `Handle`. No `#[tokio::main]`, no `full` feature.
2. `twake-gpui` provides `spawn_io(cx, future) -> Task<Result<T>>` that runs on Tokio and
   **aborts the Tokio task when the GPUI task is dropped**.
3. Adapters MUST NOT touch GPUI types; the UI MUST NOT touch SDK types.
4. Streams cross into the UI through coalescing channels (`watch` or a ~16 ms batch). One GPUI
   foreground task per subscription applies the batch and calls `cx.notify()` once.
5. `block_on` is forbidden outside `twake-desktop` start-up (clippy `disallowed_methods`).
   Blocking work uses `cx.background_spawn` or `spawn_blocking`.
6. Channels are bounded or coalescing. No unbounded queue between SDK and UI.
7. Shutdown order is explicit: UI → calls → sync service stop → store flush → runtime.

## 6. UI architecture

- **Many small entities**, not one `AppState`: one per account session, room list, open
  timeline, composer, thread window, call. `cx.notify()` re-renders only observers.
- Views are thin: they read an entity snapshot and emit actions. No business rule in `render`.
- Lists use `list(ListState, …)` (variable height) with `splice` driven by diffs, keyed by
  event id, not by index; room switch = `reset`. Pages of 30 messages; never build an
  unbounded timeline.
- Rows are cheap: no allocation per frame, text runs precomputed in the view model, avatars
  and media resolved from the media cache entity.
- Actions use GPUI actions and key bindings, declared in one `keymap` module per screen.
  Every action is reachable by keyboard. Every icon-only button is `IconAction` with a tooltip
  on hover and focus (accessibility rule inherited from the web app).
- Layout constants (769 px two-pane threshold, 360 px info pane, floating threshold) come from
  `twake-tokens` layout section, not from literals.
- Windows: main window, thread windows (D3, D173, D309), floating call window (D571). Window
  state is restored from preferences.
- `--preview` starts the UI on `twake-testing` fakes and the 10k-message fixture, without
  account, so UI work, benchmarks and screenshots never need a live server.

## 7. Matrix adapter rules (`twake-matrix`)

- Versions of `matrix-sdk`, `matrix-sdk-ui` and companions are **exact-pinned** (`=0.19.1`);
  an upgrade is its own PR with the changelog read and the contract suite green on a real Synapse.
- Follow the Matrix spec and MSCs named in the inventory; custom data under `app.twake.*`;
  unstable names read under both names. Deviations are decisions (section 14).
- `SyncService.state()` is always observed; on error it restarts with back-off; the UI shows
  the state. Offline mode is enabled.
- Sliding sync (MSC4186) only: the SDK's `SyncService` (room-list sync + encryption sync) is the
  only sync owner. We never add our own `/sync` loop (it would take the to-device messages).
  Spike S2 checks that the encryption sync does not clash with this rule.
- **SSO sign-in**: loopback redirect on `127.0.0.1:<random port>` (all 3 OS, no scheme
  registration), `loginToken` exchanged by `loginWithToken`; `twake://` is for deep links only.
  Provisional until spike S8 passes against the Twake Synapse.
- **Multi-account (D373)**: up to 5 accounts, one `AccountSession` per user id, each with its own
  `Client`, store directory, sync service and keychain entries. The UI shows one; the others run
  in the background (sync, invitations, notifications, rings). No shared mutable state between
  sessions; the composition root owns the map.
- Store: SQLite with a **random 32-byte passphrase kept only in the OS keychain**. The session
  (user id, device id, access token) is also in the keychain; the device id is reused at next
  sign-in so E2EE keys survive.
- Recovery key: held in memory for one verification, never stored, zeroized after use.
- Never show a "verified" state the crypto layer did not confirm.
- TLS: rustls only, across the whole process (one TLS stack).
- Read state, unread counts: first use what the SDK exposes; recompute (port of
  `src/domain/readState.ts`) only where the SDK is shown to differ (spike S2).

## 8. Performance budgets (proposed; confirmed or changed by spike S1 and then fixed)

| Metric | Budget | Where measured |
|---|---|---|
| Cold start to first room list, warm store, M1 | ≤ 800 ms | `--profile` trace + CI job |
| Scroll 10k-message fixture | no dropped frame at display rate | bench + manual trace |
| Idle RSS, 1 account, 50 open rooms | ≤ 250 MB | `dhat`/`samply` run |
| Image cache | 16.7 M pixels (64 MiB RGBA), 256 entries, 4 parallel fetches, 1 decode permit | unit tests |
| Decoder caps | 8192 px per side, 64 MiB allocation, 16 MiB input, 15 s fetch timeout | unit tests |
| Diff application (1k diffs) | < 2 ms | `divan` bench |

Rules: `log_if_slow(name, started, threshold)` around every UI-thread operation over 8 ms;
benchmarks in `benches/` of the crate they test; a hot path changed without a before/after
number in the PR is not merged. `tracing` spans, no `println!`. Profiling tools: `samply`,
`dhat`, `cargo-instruments` (macOS).

## 9. Security

- `unsafe_code = "forbid"` at workspace level. `twake-platform` alone sets `deny`, every
  `unsafe` block carries `// SAFETY:` and sits in an audited module (`undocumented_unsafe_blocks = deny`).
- Secrets only through the `SecretStore` port (keychain / Credential Manager / Secret Service).
  **No plaintext fallback**: if no store is available, fail with a clear error.
- Matrix access token is never sent to Twake services; they receive a Matrix **OpenID token**
  (or the LiveKit JWT) and nothing else. Tokens live in memory only.
- Encrypted media is decrypted in memory; no plaintext media on disk. The local search index
  (D21, D218) is encrypted with a key derived from the crypto store key and never leaves the device.
- Network: HTTPS/WSS only (rustls). The homeserver is user-chosen (multi-account, D373) so
  no allow-list of hosts, but redirects to non-HTTPS are refused.
- Supply chain: `Cargo.lock` committed, `--locked` everywhere, git dependencies pinned by `rev`,
  `cargo-deny` (licences, bans, sources, advisories) and `cargo-audit` on every PR,
  `cargo-vet` on auth, crypto and update crates.
- Analytics only after consent stored in account data; events sent to the side service, never
  to a third party directly.
- Release: Developer ID signing + hardened runtime + notarization (macOS), Authenticode (Windows),
  signed update manifest verified in-app. Unsigned builds are never published.

## 10. Cross-platform

- Shared crates contain no `cfg(target_os)`. All OS differences live in `twake-platform`,
  `twake-store` (backend selection) and `twake-gpui` (backend selection), each behind a trait.
- Support tiers (updated by decision when reality changes):

| | macOS | Windows | Linux |
|---|---|---|---|
| Builds in CI on every PR | yes | yes | yes |
| Unit + contract tests in CI | yes | yes | yes |
| Release artifact | signed `.dmg` | after tier-1 acceptance | after tier-1 acceptance |
| Accessibility | VoiceOver target | not promised at launch | not promised at launch |

- "Compiles", "runs", "live account" and "physical acceptance" are tracked separately per OS
  in `docs/acceptance.md`.
- Menus: native on macOS (`set_menus`); in-window menu on Windows/Linux. Notifications behind a
  `Notifier` port (first adapter `notify-rust`, native adapters where it falls short). Deep link
  `twake://` registered per OS by `xtask package`. Single instance enforced on all OS.
- Linux needs Vulkan; the error path when absent is a message, not a crash.
- Calls are optional (`calls` feature); macOS first.

## 11. Quality gates

Enforced in CI, ratchet style (limits can only tighten; a suppression list can only shrink):

- `cargo fmt --check`; `cargo clippy --locked --workspace --all-targets -- -D warnings`.
- Clippy: `unwrap_used`, `expect_used`, `panic`, `dbg_macro`, `todo` deny in non-test code;
  `cognitive_complexity` ≤ 10 per function, `too_many_lines` ≤ 60; `xtask check-size` fails a
  production `.rs` file over **300 lines**. Current offenders live in `quality-suppressions.toml`.
- Tests: `cargo nextest run --workspace` + `cargo test --doc`. TDD at unit level: the failing
  test is written first, from the need (decision/behaviour), never from the implementation.
- Property tests (`proptest`) for domain (read state, diff application, formatting).
- Project workflow tools run on **changed files only**: `crapper` (every function CRAP < 10),
  `dryer` (duplication rank < 0.8), `mutator` (survivors killed, dead code deleted). They cover
  Rust. `cargo-mutants` is the fallback if `mutator` cannot drive a crate.
- Mutation testing on `twake-domain` and `twake-app` first, then wherever logic lives.
- Snapshot tests (`insta`) of view models, not pixels.
- `cargo xtask check-deps`, `cargo deny check`, `cargo audit`.
- Docs: public items in core crates documented (`missing_docs` warn → deny).
- E2E against a real Synapse (reuse the web repo's stack) runs **nightly on `main` only**.

## 12. Build, CI, release

- `edition = "2024"`, `resolver = "3"`, `rust-version` pinned in `[workspace.package]`;
  `rust-toolchain.toml` pins the toolchain.
- Profiles: `dev` with `[profile.dev.package."*"] opt-level = 2`, `debug = "limited"`,
  `split-debuginfo = "unpacked"`; `release` thin LTO, `codegen-units = 1`; `release-fast`
  (no LTO) for the performance loop. Lints via `[workspace.lints]` + `[lints] workspace = true`.
- CI (GitHub Actions): `ubuntu-24.04`, `macos-15`, `windows-2022` on every PR (fmt, clippy,
  nextest, deny, audit, check-deps, build). Caching with `Swatinem/rust-cache`.
- Tags `vX.Y.Z` publish; the version of the tag MUST equal the workspace version (publish fails
  otherwise). Never tag by hand.
- Packaging in `xtask` (`.app` → signed notarized `.dmg`; later MSI/NSIS, deb/AppImage).
  Auto-update choice is spike S4.

## 13. Spikes (each ends with a decision in section 14)

| Id | Question | Done when |
|---|---|---|
| S1 | GPUI source: crates.io `gpui` 0.2.2 vs `gpui-unofficial` vs pinned Zed git. | 10k-message list, build time, binary size, scroll trace, VoiceOver pass on each; one chosen behind `twake-gpui`. |
| S2 | Do `SyncService` + `RoomListService` unread/notification counts suffice? | Compared with web `readState` on a real account. |
| S3 | `keyring-core` store crates per OS. | Store/read/delete passes on 3 OS. |
| S4 | Updater: Sparkle vs `cargo-packager-updater` vs Velopack. | Signed update applied on macOS from a test feed. |
| S5 | `livekit` with rustls only; libwebrtc cost. | A 1:1 call on macOS; CI minutes and bundle size measured. |
| S6 | Tray and native menu next to GPUI's loop (Windows/Linux). | Prototype per OS. |
| S7 | Inter font licence and bundling. | Licence read; font shipped or fallback chosen. |

| S8 | SSO loopback against the Twake Synapse (redirect URL allowed by the IdP?). | Sign-in completed on macOS with a loopback port. |
| S9 | i18n format: read the web `en.json` (plurals, interpolation, ICU?) before fixing Fluent vs generating from the web files. | Decision A006 written with a sample of 20 real strings. |

**S1 does not gate UI work (decision A010, Clément, 2026-11-12).** UI development starts now on
`gpui = "0.2.2"` (what builds today in this repo), behind `twake-gpui`. GPUI 0.2.2 was published
2025-10-22 and has no AccessKit, so the VoiceOver target of section 10 is unreachable on it, and
moving to another source later (Zed git, a fork) may force UI rework; that cost is accepted.
To keep it small: every GPUI import goes through `twake_gpui`, GPUI-specific code stays in
`twake-ui-kit` and `twake-ui`, and the tokens, icons, domain and adapters stay GPUI-free.
S1 runs in parallel, time-boxed to 2 days, and ends with a decision to stay or migrate; a
migration is its own PR.

## 13b. Provisional rules

These come from proposals, from the React repo's workflow or from unchecked mechanisms. They
bind like the rest until a decision changes them, but each is to be confirmed by Clément or
by a spike:

- Performance budgets (section 8) and PRD NFR-001 to NFR-006 (one set: the PRD numbers govern,
  section 8 repeats them for convenience).
- "No plaintext media on disk" (section 9): a product decision with an offline-media cost; the
  web app has a media cache.
- `cargo-deny` `[bans] wrappers` as the enforcement of P2 (mechanism unchecked); `xtask
  check-deps` is the primary check and does not depend on it.
- Nightly-only E2E and the tag/version rule (inherited from the React repo).
- Fluent for i18n (A006, after S9).

## 14. Decisions and change process

- `docs/decisions.md` holds numbered decisions (`A001`, …): context, decision, consequences,
  risk. Product decisions of the React app keep their `Dnn` and are referenced, not renumbered.
- A deviation from this file = a new `Axxx` merged first. A decision of the React app
  (`DECISION.md`) that the code here contradicts is **pointed out**, never settled silently.
- Seed decisions (to be written as `A001`–`A010` in the first commit of the workspace):
  A001 flat crates + prefix `twake-`; A002 GPUI behind `twake-gpui`; A003 exact-pinned
  matrix-sdk, one adapter; A004 one Tokio runtime, 2–4 workers; A005 keychain-only secrets;
  A006 Fluent for i18n; A007 tokens/icons generated and GPUI-free; A008 nightly-only E2E; A009 ports return boxed Send futures (dyn-compatible); A010 UI starts on gpui 0.2.2, S1 does not gate it.
- Every PR states which sections of this file it touches and, for UI, which inventory ids it ports.

## 15. Definition of done for a feature

1. Acceptance criteria from the inventory exist as failing tests first.
2. Port, fake and contract case exist (if a port changed).
3. Gates of section 11 are green on changed files; performance numbers attached if a hot path moved.
4. Works in `--preview` and, where relevant, against a real Synapse.
5. Docs and inventory status updated; deviations recorded in section 14.
