# Multi-Account (D373) (from docs/*.md of the React repo)

Part of [feature-inventory.md](../feature-inventory.md).

---------------------------------------------------------------------------

### C.1 Status and scope
Phases 1-6 implemented (2026-10-23), e2e `multi-account.e2e.ts`; phase 7 (native OAuth 2.0 / MAS) NOT done. Up to 5 accounts at once, any homeserver, same browser; one on screen, others keep syncing. Differences from original plan: phases 2-6 shipped in one PR; push/notification opened with tabs closed names its account via `?account=` (read once at start); a ring answered from a non-displayed account puts it on screen; joining a call leaves another account's call immediately without asking (plan said ask first).

### C.2 Data model
- Account key = Matrix user id (`@local:server`). Session = `(userId, deviceId, accessToken, homeserver base URL)`. Every per-account key, store, lock, rpc target derives from user id; base URL is in the session record.
- Session store (phase 2): sealed record `session:${userId}`; device id `twake-chat.matrix.device-id:${userId}`; `twake-chat-session` holds list of sealed sessions + `active`; API `list()`, `setActive()`, `forget(userId)`. Read-time migration from old `session:${hs}` / device-id-by-hs (copy to new keys; old keys stay written/read while an older build may run; removed at sign-out). Session is sealed under a key the browser cannot export (D72).
- Sign-in record: SSO `state` is an opaque UUID (D217); under it in sessionStorage `{ baseUrl, expectedUserId?, mode }` (`mode: 'add'` for add flow). Callback takes base URL and device id from there, never from the URL.
- Store lock: `holdStoreLock(account)` takes `twake-chat:${hs}` shared then `twake-chat:${userId}` exclusive (D124 steal-by-name preserved; older builds hold hs lock exclusive and stay out).
- Per-account (suffix `:${userId}`): `open-room`, `draft:*`, `declined-calls`, `folded-spaces`, `folded-sections`, `answer:*`, `answered:*`, `drive-reopen`, `session-ended`, `no-auto-sign-in`, `take-over`; Rust crypto prefix, search index, room list cache, query keys, analytics consent (account data), thumbnail cache, contact-names cache.
- Per sign-in flow: `return-to`, `sso-state`, `twake-sso-tried`.
- Global (not per account): `display`, `recent-emojis`, `personalized`, `local-language`, `local-theme`, `notifications` (browser permission per origin).
- Worker: one SharedWorker, one WASM, `Map<userId, Running>`; RPC targets prefixed `${userId}/chatRepository`; `start(credentials)`, `stop(userId)`. Without SharedWorker the page runs the same map.
- Per-account state machine: `ready`, `session-ended`, `taken-over`, `keys-unreadable` (STORE_KEY_UNREADABLE), `keys-diverged` (DEVICE_KEYS_DIVERGED, D282). One account's soft logout asks only that account to sign in again; `reload()` only when every account needs it (new build, D124).

### C.3 Lifecycle rules
- First account signed in = default: opened at start, by old links, by the frame of Twake Space. No privileged account otherwise ("Matrix client; Twake platform is only a server"). Each account uses Twake services of its OWN homeserver well-known (ToM, LiveKit, Meet, side service...).
- Deployment services (twake-bar, Drive, Mail, Scribe, analytics, GIF proxy) go ONLY to the account whose user id matches the Twake SSO identity (`twake-oidc`, one session per page) and to the first account of the configured homeserver (`isDeployment`); other accounts never send tokens/OpenID tokens to them. Twake SSO runs only if such an account exists or is being added. Login token issued by the deployment never handed to another server (`claimLoginToken`); account whose server part doesn't point at its homeserver refused (`servesUser`).
- Add an account (phase 4): account menu -> "Add an account" -> page `/accounts/new` (server name or user id). `discoverHomeserver`: matrix-js-sdk AutoDiscovery (well-known -> `m.homeserver.base_url` -> `/versions`; states PROMPT/IGNORE/FAIL_PROMPT/FAIL_ERROR; IGNORE tries `https://<server>`) -> `GET /_matrix/client/v3/login` flows -> IdP buttons (name; mxc icon falls back to name) and/or password form. Discovered domain shown beside buttons. SSO in a popup (page and accounts keep running; returns on `/accounts/callback`). Password: `m.login.password` + `m.id.user`; errors M_FORBIDDEN, M_USER_DEACTIVATED, M_LIMIT_EXCEEDED (wait retry_after_ms). Login answer carries user_id + device_id (no whoami).
  - Same IdP second account: IdP session still alive so SSO signs first account in again -> say "already added", offer password form if available, or "sign out of the IdP first".
  - Refuse account already added; name no store before login answers with user id; on cancel/error keep nothing. New account runs `unlockDevice` as usual.
- Sign-out of one account: forget its session, device id, room-list cache, drafts; remove only its pusher. Browser push subscription unsubscribed only with the last account. Sign-out wipes that account's data only.
- Push: `POST /pushers/set` with `append: true` (else Synapse deletes other users' pushers with same app_id+pushkey; pushkey = p256dh shared by browser). `user_id` in `data.default_payload` (Sygnal webpush merges it). Push/notification click carries user_id: app switches account then opens room.
- Switching: ChatAppProvider keyed by user id, then navigate to saved `open-room:${userId}` or `/` (a room of A never opens in B). No account in URL: active account is state; links open in the active account if it is in the room, else first account that is, else active one (join screen).
- Background accounts (phase 5): per account, outside active tree: LiveUpdates (sync), NotificationsWatcher, AssistantQuestionHost, IncomingCallBanner, invitations, rings, assistant checks. Notifications name the account; click switches. Badge of Chat, favicon, title = sum of all accounts. `EmbedBadges` stays for the account of the Space frame.
- Account menu (app bar): every account (avatar, user id, unread count), switch, "Add an account", "Sign out" this one, "Sign out of all". With platform bar, list sits at top of room list. Settings has an "Accounts" section.
- One call per device across accounts: Web Lock `twake-chat.call` held by worker of account in call; joining on another account leaves the current call at once (D131 extended). Incoming ring of background account names it; answering switches.
- Invitations: own-server invitations auto-joined (`acceptInvites`), other-server ones wait in InvitesSection (per account).
- Cap 5 accounts. Password login when server offers it. Native OAuth 2.0/MAS (phase 7, not done): `auth_metadata`, auth code + PKCE in popup, dynamic client registration, device scope `urn:matrix:client:device:<id>`, refresh tokens in sealed session; MAS compat layer still serves m.login.sso/password so legacy login works on matrix.org-like servers.
- Switcher not merged inbox: one active account on screen.
- Analytics: `account_added` (method sso|password), `account_switched`, `account_signed_out` (scope this|all), sent only by the deployment account (Decision 3) / account on screen.
- Each account has its own device + E2EE keys; device id re-sent only for its own account.

---------------------------------------------------------------------------
