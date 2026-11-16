# Performance (from docs/*.md of the React repo)

Part of [feature-inventory.md](../feature-inventory.md).

---------------------------------------------------------------------------

### A.1 Method / governance
- Every budget is a test that fails on regression. Ceilings = reference-machine measure x 1.5.
- Reference machine: Apple M5, 32 GB, e2e Chromium, local stack (Synapse, Postgres, ToM, LiveKit). Measures dated 2026-10-06.
- On CI (`CI` set) tests run and log, ceilings not enforced (CI runner is 2-7x slower, D409). Production-image ceilings (`perf-large`) apply only with `E2E_APP_URL`.
- Every e2e run also runs `--tag perf --workers 1` alone, records `{name,value,unit}` to `current.jsonl`, appends `{date,commit,host,measures}` to `history.jsonl`. Regression = worse than median of last 5 runs on same machine by >25% (PERF_TOLERANCE) plus floor (10 ms, 2 MB, 5 kB), on TWO consecutive runs. Fallback baseline `perf-baseline.json` only when <3 runs on that host and same host name. Stack-dependent measures (encrypted room first read, heap_encrypted) never fail.
- Room open is repeated 3x in its test (from another room); median recorded.
- Instrumentation to replicate natively: marks `twake-chat:room-list`, `twake-chat:room-open` (from first render of route naming a room to commit showing first message or empty state; only last kept), `twake-chat:send-shown` (Enter -> bubble on screen), `twake-chat:send-sent` (Enter -> bubble marked sent by server; difference = round trip).
- Telemetry `performance_measured`: Web Vitals (INP, LCP, CLS, FCP, TTFB), `room_list`, `room_open` (sampled 1 in 10, `sampleRate: 10`).
- Audit file: docs/research/perf-audit-2026-10-30.md (26 findings, not read here). On that account a room switch = 0-51 ms of long tasks.

### A.2 Budget table (large account: 300 rooms in a space, one room with 5 000 messages; production image)
| Budget | Measure | Reference | Ceiling |
|---|---|---|---|
| Room list, first sign-in | mark `room-list` from navigation (first page of 100 rooms) | 653 ms | 1 000 ms |
| Every room on screen | scroll list to end, pages of 100 | +3 283 ms | 5 000 ms |
| JS heap after 10 s at rest | usedJSHeapSize | 125 MB | 190 MB |
| Main thread at rest, 5 s | sum of long tasks >50 ms (50 ms timer lag logged) | 0 ms (lag 34 ms) | 100 ms |
| Main thread of a room switch | six switches, long tasks >50 ms summed, per switch | not measured (0.6.2: 85-185 ms) | 50 ms |
| A page of history, ten pages back | "Load earlier messages", 30 msgs, until shown | 174 -> 390 ms | 750 ms |
| Search in 5 000-msg room | click -> first result over the 320 messages read | 144 ms | 1 000 ms |
| Encrypted room, new device, first message read | click -> first message readable (device just unlocked with ToM key) | 240 ms | 1 000 ms |
| Encrypted room, new device, 20 sync messages read | same, until nothing unreadable | 492 ms | 1 500 ms |
| Heap after that + a history page read | usedJSHeapSize | 111 MB | 170 MB |
| Demo account reload -> room list | mark `twake-chat:room-list` (handful of rooms) | - | 3 000 ms |
| Pin/link jump to far message (`perf-pin-jump`) | latest view | - | 1 500 ms, heap <190 MB |
| Call memory floor | bare LiveKit page in whole browser | 445 MB | Chat additions must not grow with account size |

Also: first-page JS checked by `npm run budget` (JS before sign-in + crypto WASM after). Initial JS 906 -> 900 kB gzip (D534).

### A.3 Startup / first sync (rules)
- Production, 87 rooms: reload took ~5.4 s to first rows (page+OIDC ~1 s; worker crypto + first sliding sync 1-3.7 s; `ready` 3.6 s; rows at 5.4 s). `fetchRoomList` in worker = 3-4 ms. The 1.8 s gap was the address-book read (ToM `/_twake/addressbook` 1.7 s, request timeout 15 s) awaited after `ready`.
- Rules:
  - Room list must NOT wait for contact names. Names read at app creation, in parallel with crypto and sync; cached on device (`twake-chat.contact-names:<account>`) and served instantly on next load. Without cache (first sign-in) list/chat view wait at most 300 ms (`FIRST_NAMES_WAIT_MS`), then show Matrix names; re-read when names land.
  - Names request personal contacts only (`/_twake/addressbook?personal=true`); contacts screen reads whole book (D546).
  - Room list auto-widens after first response: 100 rooms per step (~0.4-0.6 s, 820 KB each on 300 local rooms), one step at a time, 250 ms apart, never while a room is opening, up to 500 rooms (`ROOM_LIST_AUTO_MAX`). Beyond: scroll to 80% of list or footer in view widens. First paint waits for none. Skeleton until first rooms in; footer says how many still loading.
  - First sliding-sync request: 30 rooms + the open room (was 100 rooms x 20 msgs), D559. Reload to first message on 302-room account: 1.31-1.85 s -> 0.60-0.74 s (recent room), 1.09-1.83 -> 0.63-0.70 s (room far past window). Real server (94 rooms, no VPN): `[0,99]` 1.0-5.2 s / 1 613 KB vs `[0,29]` + room past window 0.57-1.54 s / 615 KB. Raw on local stack: 110 ms/592 KB for [0,99]; 30 ms/199 KB for [0,19]. Without to_device/e2ee extensions in those measures.
  - LCP of a signed-in reload = the first message = first sliding sync response. Page paint alone ~500 ms (cold, anonymous, chat-ng).
  - Preconnect to homeserver and SSO from runtime config before bundle runs. Session `ready` also waits for Twake SSO frame, 3 s max (D511).
  - Past 100 rooms, read the "room index" once after first response (D503): 2 000 rooms = 436 kB in 1.1-2.6 s, background. Separate connection without extensions, reads only bump_stamp + name of every room (feeds list search, D504).
- Native analogues: cache contact names/room list on disk and paint from cache; ask sync for 30 rooms + open room; widen 100/step @250ms; never block paint on address book.

### A.4 Room list (large accounts)
- Up to 500 rows mounted (D527). Rows use content-visibility:auto with intrinsic size 56 px (44 px compact or in a space): off-screen rows not laid out/painted; find-in-page, focus, list semantics preserved. Native: virtualize but keep a11y/focus/scroll-to-active and 80% auto-load behaviour.
- Avatar (`DeferredAvatar`) shows initials until row within one screen of the scroll container (`useNearViewport`, one shared IntersectionObserver per container); only then fetches picture and (direct rooms) presence; keeps them. Estimate 500 rows -> ~15 avatar fetches at mount, layout/paint ~30 rows.
- `roomListQuery` keeps identity of unchanged rooms (`keepUnchangedRooms`); rows memoised, handlers one object. A room switch redraws only the 2 rows that change, a message only its room row (#889).
- Switch/filter are low-priority transitions; search fields deferred (`useDeferredValue`).

### A.5 Timeline / history
- Page size 30 messages (`loadOlderMessages`). Page cost must not grow with depth (fixed: first 206 ms, tenth 1 036 ms before -> 174/390 ms). Keep identity of unchanged messages (`keepUnchangedMessages`), message item memoised, per-message actions cached; a page renders its 30 messages only. `fetchChatView` ~1 ms for 320 msgs.
- Message bubbles also content-visibility:auto. Timeline not windowed (known limit: DOM-growth layout cost; scrolling down 5 000 msgs loads 166 `/messages`, ~2.5 GB heap). Native should window the timeline.
- Jump to unloaded message (`?event=`, pin, search result) = one `/context` + one `/messages` below if window not filled (D605). Bench: 100 back 87-107 ms/2 requests (was 232-356, 3 req); 1 500 back 92-113 ms; 5 000 back 94-118 ms (was never shown, 10 `/messages`, 30 s); 10 jumps heap 72-78 MB (was 178-1 155 MB). "Jump to latest" ~45 ms, no request. No read receipt sent while view has newer messages (D605).
- Sync refetch granularity (D537): a change carries room + kind; typing/receipt/message in room B leaves open room A's chat view, threads, members alone; room list re-read once per burst, typing at most once per second. Change without room = `all`. Readers computed in one pass (`makeIsReadByOthers`, bisection per member), quotes/polls one index per read (`makeReadLookup`), search index writes messages in 100 ms batches.
- Encrypted rooms: sliding sync SDK only decrypts last event + those past read receipt, so client explicitly decrypts every served message (`decryptEventIfNeeded`), notifies when decrypted (`MatrixEventEvent.Decrypted`); undecryptable shows "cannot be read yet", never empty. Key of each session fetched from backup when a message needing it is shown; full backup restored in background.

### A.6 Media memory
- Video not downloaded when shown: message holds still (`thumbnail_url`/`thumbnail_file`, decrypted if encrypted), duration, play button; file fetched at first play / mini player / expand / fullscreen. Images, voice, avatars fetched when shown.
- Media kept as blob URL in cache entry for 5 min after last use (`MEDIA_GC_MS`), then revoked (unless corner player is playing). Before D525 every blob lived the session (>1 GB in video-heavy rooms).
- Worker keeps no copy of downloads; buffers transferred, not copied.
- Unencrypted media (video, non-SVG images, avatars): load by URL with auth token added by service worker (browser streams/caches natively), `loading=lazy`, `decoding=async` (D528). Encrypted media: decrypt, blob path. Fallback to download when URL fails.
- Avatar thumbnails: only two sizes, 96 px (up to 48 px display) and 320 px (above) (`avatarThumbnailSize`), so a person is fetched once for lists and messages.
- Thumbnail cache (Cache API `twake-chat.thumbnails:<userId>`): cache-first, key = URL without token, max 300 entries, each <=160 kB (48 MB), oldest out; per account, cleared by `forgetAccount` for that account only; nothing cached for unknown-account origins.
- Synapse 1.163 ignores `Range` on media (200 whole file): a seek past buffered waits for download.
- Fonts: Inter woff2 344 kB (was 859 kB TTF), preloaded, cached 30 days. Build writes .gz level 9; nginx gzip_static; no brotli.
- Lazy chunks fetched on idle (not imported elsewhere): call view, poll, Create pages, viewers, emoji picker (`ui/lazyPanels.tsx`). Menus/dialogs built at first opening (`useOpenedOnce`); a room without call mounts no call view. (First room opened used to pay 0.5-2 s of style work.)

### A.7 Calls
- Chat's overhead around a call must not scale with account: screens around call follow members only (`useCallUpdates`); only the call view follows media events (`useCallMediaUpdates`) otherwise each speaker change redraws the room list (D580).
- After-call slowdown not reproduced (memory notes); #1049 fixed 2 leaks.

### A.8 Send path
- Optimistic bubble on Enter (`send-shown`), then sent mark (`send-sent`). Time budgets (`perf-thread-send`) dev-machine only.

---------------------------------------------------------------------------
