# Auth, session, startup

Part of [feature-inventory.md](../feature-inventory.md). Final behaviour per decision (Dnn of the React repo `DECISION.md`); later decisions override earlier ones. `modified by` = later entries that replace, update, complete or narrow this one (read them together; the latest wins). Entries with `superseded-by` are dead and listed at the end.

## Final behaviours (9 entries)

- **D4** [auth] replaces: - | final: Sign-in through Synapse SSO (idp `oidc-twake`), delegating to Twake identity provider; matrix-js-sdk Rust crypto, Simplified Sliding Sync; calls MatrixRTC + LiveKit own UI. — *modified by: D217*
- **D20** [auth] replaces: - | final: user-interactive auth (sign out device, replace cross-signing keys) opens `/_matrix/client/v3/auth/m.login.sso/fallback/web?session=...` in a popup from a click, waits for `authDone`, replays request.
- **D72** [auth] replaces: D16 | final: Matrix session (homeserver, user, device, access token) sealed AES-GCM in IndexedDB `twake-chat-session` under a non-extractable key; closing all tabs does not sign out; several tabs share one session. If storage is refused (private window, denied frame) sign-in still works for the life of the page. Read at boot, checked with `whoami`, forgotten when invalid / at sign-out / hard or soft logout. Per-tab flags (no auto sign-in after sign-out, return path, take over) in sessionStorage. Frame of Twake Space reads the same store (partitioned). Two tabs signing in at the very same moment: one may go through the SSO again. — *modified by: D218, D579*
- **D217** [auth] replaces: D4 (narrows) | final: SSO redirect carries `state=<uuid>` kept per tab (sessionStorage); the callback token is exchanged only with the same state, spent once. Otherwise login screen says "This sign-in link was not asked for in this tab". Embed sign-in uses its own `state=embed-...`.
- **D278** [auth] replaces: - | final: First loading state (session starting) = Flutter splash on page background (`background.paper`): Lottie `lottie-chat.json` 176 px centred, played once (last frame still under prefers-reduced-motion), Twake Workplace logo 210 px wide 40 px from bottom. No visible text; accessible status "Connecting to …". Animation aria-hidden. Same in dark and in embed. Other spinners unchanged.
- **D280** [auth] replaces: completes D269,D271 | final: Confirm buttons of the reset and of signing devices out open the SSO popup at once (blank), loaded when server asks; reset waits up to 10 s for sync echo before asking server. Popup closes if server asked nothing; no second popup while one is open.
- **D425** [auth] replaces: - | final: Re-auth opens `/_matrix/client/v3/auth/{stage}/fallback/web` of SSO stage, else first other stage (terms, captcha), then replays with UIA session; first attempt without proof stays; no password fallback. Login warns (never blocks) if server lacks `v1.11` or `org.matrix.simplified_msc3575`. Well-known read `app.twake.tom` before `t.server`, `app.twake.federated_identity_services` before `m.federated_identity_services`.
- **D469** [auth] replaces: D53,D36 | final: Scribe/Drive on-demand Twake SSO sign-in opens in a popup everywhere (page and typed text stay; option runs after sign-in). Page-load sign-in for platform bar still by redirection.
- **D511** [auth] replaces: D64 | final: Standalone with platform: Twake SSO asked in hidden frame `prompt=none` beside account start, page waits for it; frame returns on `/oidc/callback` with `state` starting `embed-` (static relay). After 3 s or `login_required` page redirects to SSO once per tab. Safari/Firefox/private Chrome get `login_required` and fall back to redirect. — *modified by: D559*

## Superseded

- **D16** superseded by D72. Found: reactions sent in clear text in encrypted rooms (edits/replies encrypted); a reaction taken back must not show "🎉 0"; thread reply `m.in_reply_to` with `is_falling_back` is not a quote. —  [auth] replaces: - | final: Matrix session kept so reloads don't re-login (Synapse rate-limits: 429 tells the user how long to wait). superseded-by: D72. Found: reactions sent in clear text in encrypted rooms (edits/rep

## Acceptance criteria (from e2e test bodies)

### e2eA: Auth / session

- [chat.e2e.ts] Given a fresh account When the user signs in through the SSO Then `room-list` is visible within 30 s, at least one `room-row` shows, and `device-unlocked` is attached within 60 s (first device finishes building encryption before the app can be closed).
- [chat.e2e.ts] Given a new device When it signs in Then it unlocks itself from the key ToM keeps: `device-unlocked` attached (60 s), `security-banner` and `keep-recovery-key` both hidden.

### e2eA: Sign-in security / loading

- [sign-in-state.e2e.ts] Given a login token of alice obtained elsewhere When /callback?loginToken=T (no state) or /callback?state=x&loginToken=T is opened Then an alert contains "This sign-in link was not asked for in this tab" (30 s) and `room-list` count 0 (login CSRF, D4); the normal `sign-in` button flow still works afterwards.
- [loading-screen.e2e.ts] Given a reload of / Then `splash-animation` is attached (the Flutter loading-screen animation) until `room-list` is visible (30 s).
- [first-room-open.e2e.ts] Given the first room opened Then no `call-stage`; message menu (hover + `message-more`) shows at least one menuitem, Escape removes all; starting a call (`startCall`, `join-call`) shows `call-stage` within 30 s.

