# Spike S2/S3: matrix-sdk 0.19.1 and rustls

Branch `spike/matrix-sdk`, 2026-11-12, macOS arm64, rustc 1.99.1. Nothing here ran against a
real account or a production server. The only network calls were three unauthenticated `GET`s
(`/versions`, `/login`, `/.well-known/matrix/client`) on the local e2e Synapse.

## 1. Dependencies added to `twake-matrix` (the only crate)

```toml
matrix-sdk    = { version = "=0.19.1", default-features = false, features = ["e2e-encryption", "automatic-room-key-forwarding", "sqlite", "rustls-aws-lc-rs"] }
matrix-sdk-ui = { version = "=0.19.1", default-features = false, features = ["rustls-aws-lc-rs"] }
```

- `cargo xtask check-deps`: passes (P2 holds).
- Sliding sync (MSC4186) has **no cargo feature** in 0.19.1: it is always compiled. There is no
  `experimental-sliding-sync`.
- `default` of both crates turns on `rustls-aws-lc-rs`, `sqlite`, `e2e-encryption`; with
  `default-features = false` the four features above are the minimal set. The SDK offers no
  `ring` feature: the only rustls choice is `rustls-aws-lc-rs`.

### TLS

| Check (`-p twake-matrix`, and `--workspace`) | Result |
|---|---|
| `cargo tree -i openssl-sys` | fails: package absent |
| `cargo tree -i native-tls` / `-i openssl` | fails: package absent |
| `otool -L` of the probe | only `libsqlite3`, Security, CoreFoundation, libiconv, libSystem: no libssl |
| `aws-lc-rs` 1.19.0 / `aws-lc-sys` 0.47.0 | present (via `rustls` 0.23.46, `rustls-webpki`, `hyper-rustls`) |
| `ring` 0.17.14 in `twake-matrix` alone | absent |
| `ring` in `twake-desktop` | present, via `gpui` / `gpui_http_client` |

Finding: rustls is the only TLS stack, but **two crypto providers coexist in the desktop
binary** (aws-lc-rs from matrix-sdk, ring from gpui). With both features enabled rustls cannot
choose a default provider by itself; the composition root must call
`rustls::crypto::aws_lc_rs::default_provider().install_default()` once, before any client.
"One TLS stack" in ARCHITECTURE section 7 stays true for the library, not for the provider.
`aws-lc-sys` compiles C with cmake (built here without trouble; Windows CI needs checking).

## 2. Build cost

Cold means `target/` removed first (it took the better part of an hour to delete, see Risks).
Wall times from `time`; the dev profile uses `opt-level = 2` for dependencies.

| Build | Wall | CPU (user) | `target/` after |
|---|---|---|---|
| `cargo build -p twake-matrix --example probe` (debug), cold | 3 min 17 s | 1089 s | 2.1 GB (`debug` 2.0 GB) |
| same, release (thin LTO, 1 codegen unit), debug already built | 3 min 41 s | 651 s | 3.1 GB (`release` 1.1 GB) |
| debug rebuild after editing the probe | 2 to 3 s | | |
| release rebuild after editing the probe | 12 s | | |

Free disk stayed above 32 GB, the 15 GB threshold was never reached.

Binary size is **not measured**: `twake-desktop` does not link `twake-matrix` yet. The probe
(never polled) is 6.4 MB in debug and 1.9 MB in release (1.4 MB stripped), but its async body
is dead code, so the release figure is a floor and says nothing about the real adapter. It must
be measured when `twake-desktop` links the adapter.

About 407 crates are in the `twake-matrix` normal dependency tree.

## 3. `cargo deny check`

Result: `advisories FAILED`, `licenses FAILED`, `bans ok`, `sources ok`. 81 `duplicate`
warnings (windows-* crates, `rand`, `syn`, `thiserror`, `toml`, ...); bans only warn.

| Item | Kind | Cause | Action |
|---|---|---|---|
| RUSTSEC-2026-0319 `anymap2` 0.13.0 | unmaintained | `matrix-sdk` 0.19.1 (direct) | **applied** in `deny.toml` (transitive, unmaintained only) |
| RUSTSEC-2026-0459 `libsqlite3-sys` 0.38.2 (CVE-2026-119375) | **vulnerability** | `rusqlite` 0.40.2 <- `matrix-sdk-sqlite` 0.19.1 | **not ignored**; see below |
| `CDLA-Permissive-2.0` | licence not allowed | `webpki-root-certs` 1.0.12 <- `rustls-platform-verifier` <- `reqwest` <- `matrix-sdk` | **proposed, not applied** |

RUSTSEC-2026-0459: wrong pages can be returned when a database is opened with a `-wal` file of
another page size (a local attacker who plants the files). It concerns the **bundled** SQLite
(`libsqlite3-sys` `bundled` feature). Here `bundled` is off: the probe links the system
`/usr/lib/libsqlite3.dylib` (SQLite 3.51.0 on this macOS, below 3.54.0 as well, so the exposure
is the OS's). The fix is `libsqlite3-sys` >= 0.39 (rusqlite 0.41), which `matrix-sdk-sqlite`
0.19.1 does not allow. Options: (a) accept on macOS (local attacker only, system library) and
track the next matrix-sdk release; (b) none for Windows/Linux, where there is no system SQLite
and the `bundled-sqlite` feature brings exactly the affected copy. Do not ignore it in
`deny.toml`; it needs an owner's decision (see Recommendation).

Proposed licence line, if the decision is to keep `deny` green:

```toml
# CDLA-Permissive-2.0: permissive data licence of the Mozilla root certificate bundle
# (webpki-root-certs 1.0.12 <- rustls-platform-verifier <- reqwest <- matrix-sdk). Spike S2.
allow = [ ..., "CDLA-Permissive-2.0" ]
```

Also noticed: `Unicode-DFS-2016` is now reported as an unused allowance (warning, not new).
`oauth2-reqwest` is an `0.1.0-alpha` crate in the tree (OAuth/OIDC of the SDK).

## 4. Compile probe (`crates/twake-matrix/examples/probe.rs`)

`cargo build -p twake-matrix --example probe` succeeds (debug and release). It builds a
`Client` with a SQLite store and a `SyncService`, reads `state()` and `room_list_service()`,
gets a `Timeline` with `RoomExt::timeline()`, and calls `start()` / `stop()`. `main` creates
the future and drops it: no poll, no network.

Gaps against ARCHITECTURE:

1. **`#![recursion_limit = "256"]` is required** in any crate whose async fn holds these futures
   ("queries overflow the depth limit", depth 130). `twake-matrix/src/lib.rs` will need it.
2. **Store secret**: `sqlite_store(path, Some("passphrase"))` runs an expensive key derivation
   on every open. ARCHITECTURE says "random 32-byte passphrase". The SDK has
   `SqliteStoreConfig::key(Some(&[u8; 32]))` (raw key, no derivation) used through
   `ClientBuilder::sqlite_store_with_config_and_cache_path`. Use `key`, fed from the keychain bytes.
3. **SQLite is the system one**, not SQLCipher and not bundled (see section 3). Per-OS decision.
4. **`SyncService`**: the builder has `with_offline_mode()` (matches "offline mode is enabled");
   `state()` is an observable (`.get()` / subscribe), as assumed. `room_list_service()` returns an `Arc`.
5. **`Room::timeline()` is a trait method** (`matrix_sdk_ui::timeline::RoomExt`), easy to miss.
6. **SDK has an `sso-login` feature** (local loopback server on `axum` / `tower`). Not enabled.
   ARCHITECTURE plans its own loopback; adopting the SDK one would add `axum` to the tree.
   Undecided, leave it off until S8.
7. **Unread**: `Room::num_unread_messages()`, `num_unread_notifications()`, `num_unread_mentions()`
   and `unread_notification_counts()` exist. Whether they match `readState.ts` is the open
   half of S2 and needs a real account (not done here).

## 5. Local Synapse (not started, not changed)

`docker ps` works. Already up: `twake-chat-e2e-synapse-1` (`ghcr.io/element-hq/synapse:v1.162.0`,
healthy) on `127.0.0.1:8008`, with Postgres, Dex, LDAP, ToM, Meet, LiveKit, side service
(`127.0.0.1:3003`), mailpit. Three facts read from it:

- `unstable_features` has `org.matrix.simplified_msc3575`: sliding sync is on.
- Login flows: `m.login.sso` (`oidc-twake`), `m.login.password`, `m.login.token`.
- `.well-known` returns `base_url` `http://localhost:8008/` (plain HTTP).

A second container named `synapse` (`matrixdotorg/synapse:latest`, port 80, unhealthy) belongs to
another worktree (`.worktrees/threads/integration_test`); not used.

How the adapter could run against it (`~/Developer/twake-space-chat/e2e/docker/synapse/homeserver.yaml`
and `docs/e2e.md` were read only):

1. `ignored` integration tests in `crates/twake-matrix/tests/`, enabled by
   `TWAKE_TEST_HOMESERVER=http://127.0.0.1:8008`, password login with a seeded user
   (users are created by `e2e/scripts/seed.mjs` of the React repo).
2. Store in a temp directory under `$HOME` (Colima does not see `/var/folders`, see the React
   repo notes), key from a fixed test value, never the keychain.
3. ARCHITECTURE says HTTPS/WSS only: tests need a loopback `http` exception, compiled only
   for tests.
4. The stack is shared with other sessions: tests must use rooms of their own and never
   `stop.sh` / `start.sh` it.

## 6. Risks

- Exposure to RUSTSEC-2026-0459 on Windows/Linux (bundled SQLite) until matrix-sdk upgrades
  rusqlite. Pin `=0.19.1` delays the fix further.
- Two crypto providers in one binary; a missing `install_default` panics at the first TLS use.
- Build weight: about 3.5 minutes of wall time and 2 to 3 GB per profile for this crate alone;
  `aws-lc-sys` needs a C toolchain and cmake on every CI OS.
- `anymap2` unmaintained and `oauth2-reqwest` alpha are inherited from the SDK.
- Housekeeping: `rm -rf target` of this workspace took close to an hour (large tree, slow
  disk). Prefer `cargo clean -p <crate>` or a separate `CARGO_TARGET_DIR`.
- Not verified: sync behaviour, encryption sync vs the "no own `/sync`" rule, unread counts,
  SSO flow, any value of the real binary size.

## 7. Recommendation

**Stay** on `matrix-sdk` / `matrix-sdk-ui` 0.19.1 with rustls: everything ARCHITECTURE assumes
compiles, TLS is rustls only (no OpenSSL, no native-tls), sliding sync is built in, and the
local Synapse supports it. Conditions:

1. Owner decision on RUSTSEC-2026-0459 (macOS: accept and track; Windows/Linux: wait for
   rusqlite 0.41 in matrix-sdk-sqlite or carry a patched SQLite). Do not add it to `deny.toml`.
2. Allow `CDLA-Permissive-2.0` in `deny.toml` (permissive).
3. Install the aws-lc-rs provider explicitly at start-up (A019), enable `recursion_limit`,
   use `SqliteStoreConfig::key`.
4. Run the live half of S2 (unread counts, encryption sync) on the local Synapse before
   `twake-matrix` is built for real.
