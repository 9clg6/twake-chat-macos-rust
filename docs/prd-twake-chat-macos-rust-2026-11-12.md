# Product Requirements Document: twake-chat-macos-rust

**Date:** 2026-11-12
**Author:** clement
**Version:** 1.0
**Project Type:** desktop-app
**Project Level:** 4
**Status:** Draft

---

## Document Overview

This Product Requirements Document (PRD) defines the functional and non-functional requirements for twake-chat-macos-rust, a native desktop client for Twake Chat written in Rust with GPUI. It serves as the source of truth for what will be built and provides traceability from requirements through implementation.

**Related Documents:**
- Product Brief: `docs/product-brief-twake-chat-macos-rust-2026-11-12.md`
- Feature inventory (behaviour source): `docs/spec/feature-inventory.md` and `docs/spec/features/*.md`
- Design system export: `docs/spec/design-system.md`
- Architecture research: `docs/research/rust-architecture-sota.md`

**Conventions used in this document**
- `Dnn` = numbered decision of `DECISION.md` of the React repo (`~/Developer/twake-space-chat`); the latest decision wins. Inventory IDs (A1, R2, T7...) point to the tables of `feature-inventory.md`; "area file" means `docs/spec/features/<area>.md`.
- Priority mapping from the inventory: P0 = Must Have, P1 = Should Have (in scope of the first release, per the brief), P2 = Could Have, P3 and web-only = Won't Have (this release).
- Acceptance criteria are Given/When/Then lines. "(e2e)" = condensed from an e2e line of the area file; "(dec)" = written from the behaviour text of the decision, because the inventory gives no Given/When/Then line for it. Test ids of the web app are dropped.
- "proposed" on a target means the sources give no number; it must be confirmed (see Open Questions).

---

## Executive Summary

Twake Chat is the Matrix client of the Twake suite. It exists today as a React web app and a Flutter app; there is no native desktop client. This project builds one: a Cargo workspace in Rust, UI on GPUI, Matrix through `matrix-sdk` / `matrix-sdk-ui` (sliding sync MSC4186, Rust crypto). The UI is a 1:1 port of the React app (same features, same look from `@linagora/twake-mui` tokens), driven by the feature inventory. macOS ships first (signed, notarized); Windows and Linux are designed in from day one, compiled in CI from the first commit, and released once GPUI platform support is verified.

The PRD holds 140 functional requirements in 16 epics (Must: usable chat with E2EE; Should: daily features; Could: polish; Won't: web-only and embed-only items) and 40 non-functional requirements with measurable targets. Targets without a number in the sources are marked "proposed".

---

## Product Goals

### Business Objectives

1. Feature parity with the React app on P0 and P1 areas, macOS first.
2. Identical design, with UI tokens generated from twake-mui, never copied by hand.
3. A lighter and faster client than a browser tab: lower memory, faster start, smooth long timelines.
4. A signed, notarized macOS build, then Windows and Linux builds.
5. A reusable Rust/GPUI design-system layer (`twake-tokens` and components) for other Twake apps.

### Success Metrics

| Metric | Target | Status |
|---|---|---|
| Idle memory, one account, 500 rooms | Lower than the React app in a browser tab on the same account (React reference: JS heap 125 MB at rest on 300 rooms, ceiling 190 MB; whole-tab RSS to be measured) | Brief; absolute value proposed in NFR-001 |
| Cold start to room list (warm store) | Under 800 ms on an M1 | proposed (research) |
| Scroll a 10,000-message timeline | No dropped frame at 60 fps | requested; 120 Hz target proposed |
| `unsafe` in application crates | Zero | Brief |
| `cargo-deny` and `cargo-audit` | Clean on every PR | Brief |
| Parity checklist (inventory P0 and P1) | 100 percent of Must and Should FRs pass their acceptance tests | Brief |
| New user flow | Sign in, see rooms, read and send in an encrypted room on macOS | Brief |

---

## Functional Requirements

Functional Requirements (FRs) define **what** the system does - specific features and behaviors.

Each requirement includes:
- **ID**: Unique identifier (FR-001, FR-002, etc.)
- **Priority**: Must Have / Should Have / Could Have / Won't Have (MoSCoW)
- **Description**: What the system should do
- **Acceptance Criteria**: How to verify it's complete
- **Source**: area file and decision (Dnn), with inventory ID

---

### EPIC-01 Foundation, design tokens and shell

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-001 | Must | Read client config (`config.json`) and runtime env at start: homeserver, Twake service URLs (ToM, side service, LiveKit via well-known). | Given a config naming a homeserver, When the app starts, Then sign-in targets that homeserver and Twake services are discovered from its well-known (dec). | A9, D18; feature-inventory 2.1; `twakeServices` in 3.2 |
| FR-002 | Must | Send tokens only to https services; error reports never carry a Matrix id. | Given a service URL in http, When a token would be sent, Then the call is refused (dec). | A10, D210, D214; auth.md |
| FR-003 | Must | `twake-tokens` crate generated from twake-mui (colours, typography, spacing, shape, shadows, motion) with a script; light, dark, system. | Given light then dark scheme, When any component renders, Then every colour comes from a token and none is hard-coded; re-running the export reproduces the crate (dec). | 2.20, D245, D311, D38; design-system.md 1 and 4.1 |
| FR-004 | Must | Brand blue `#0A84FF` and the twake-mui palette untouched; wallpaper = light-blue paper in the timeline (dark variant). | Given the default theme, When the timeline shows, Then the primary colour is `#0A84FF` (dec). | 2.20, D245, D183; theming.md |
| FR-005 | Must | Window shell: app bar, bottom bar (Contacts, Chat, Settings; Activity is the bell of the room list, D318 reverses D182), three panes square, 4 px apart. | Given a window of 1500 px, When Chat opens, Then list (360 px), conversation and optional side pane (360 px) show with 4 px gaps (dec). | 1.2, D194, D318, D182; roomlist.md |
| FR-006 | Must | 769 px rule: side pane is a fixed 360 px column beside the conversation from 769 px; below 1104 px it floats over the conversation with shadow, Escape closes; below 769 px one pane at a time. | Given 1024 px with info open, When viewed, Then the pane 360 px floats over (1024 < 1104) (e2e). | 1.2, D220; info.md |
| FR-007 | Should | Bundle Inter as the UI font with the twake-mui stack as fallback; Apple Color Emoji for emoji. | Given a Mac without Inter installed, When any text renders, Then it uses the bundled Inter (dec). Licence: see OQ-02. | design-system.md 1.2 |
| FR-008 | Must | Loading screen (Flutter-style animation) and message skeleton shimmer until the first sync is read. | Given a cold start, When the first sync is not in yet, Then the splash shows until the room list is visible (e2e). | A7, D278, D428, D465, D518; auth.md |
| FR-009 | Must | Translation layer for the 7 locales; language follows the OS, English when unsupported; capitalised names; buttons 1-2 words on one line. | Given OS language French, When the app starts, Then the UI is French; Given an unsupported language, Then English (e2e adapted: common-settings fr-FR gives lang fr). | 2.19, D67, D68, D209, D383; settings.md |
| FR-010 | Should | Banner stack: one bar for all notices, paged by arrows (safety first, then what reads the room, then info). Toasts for action errors, never in dialogs. | Given two notices, When shown, Then one bar with a pager shows the safety notice first (dec). | 1.2, D356, D393, D488, D591 |
| FR-011 | Must | Single running instance per app-support directory (replaces Web Locks and shared worker); second launch focuses the first. | Given the app running, When a second launch starts, Then the first window is focused and no second client opens (dec). | feature-inventory 5, D19, D131 |

### EPIC-02 Sign-in and session

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-012 | Must | Homeserver discovery (`.well-known`), then login by Synapse SSO in the system browser session (redirect to the app); password form when the server offers it. | Given a fresh account, When the user signs in through the SSO, Then the room list is visible within 30 s and at least one room row shows (e2e). | A1, D4, D16; auth.md |
| FR-013 | Must | Sign-in callback accepted only if it answers a request this app started (state check). | Given a login token obtained elsewhere, When the callback opens without the app's state, Then an alert says the link was not asked for and no room list shows (e2e). | A6, D217, D4; auth.md |
| FR-014 | Must | Session (access token, user id, device id) persisted in the system keychain; device id reused at next sign-in so E2EE keys survive. | Given a signed-in user, When the app restarts, Then the room list shows without a new sign-in and the same device id is used (dec). | A2, D72, D218; encryption.md |
| FR-015 | Must | Sign out and session lifecycle machine (pure state machine); a session ended on the server returns to sign-in. | Given a session ended on the server, When sync fails with 401, Then within 60 s the sign-in screen shows with "session has ended" (e2e). | A3, D442, D396; auth.md |
| FR-016 | Should | Re-authentication (UIA) for cross-signing reset: SSO page opened from the user's click. | Given a reset of identity, When the server asks for authentication, Then the SSO page opens from the click and the reset continues after it (dec). | A4, D20, D280, D425 |
| FR-017 | Could | Twake SSO (OIDC PKCE) for the identity of the user toward Twake services; tokens in memory. | Given a Twake service call, When it needs identity, Then an OpenID/OIDC token is used and never stored on disk (dec). | A5, D4, D469, D511 |
| FR-018 | Should | Show cached room list and timeline before the first sync completes (window from the cached store). | Given a warm store, When the app starts, Then rows paint from cache without waiting for the network (dec). | R17, D533; performance.md A.3 |

### EPIC-03 Room list, search and invites

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-019 | Must | One list without titles: favourites first, then rooms by last message (not last event), spaces where their latest room would be, low priority last. Optional "Group by kind". | Given pinned, recent and low-priority rooms, When the list renders, Then the order is favourites, recent, low priority (dec). | R1, D190, D101, D37, D310, D337; roomlist.md |
| FR-020 | Must | Row: avatar, name, last message with "You:" prefix, time, ticks, typing, media icon, pin, unread count (blue; red mention; grey muted), `@`, kind badge (space, channel, public, broadcast, voice, encrypted). | Given a room with 3 unread and a mention, When its row renders, Then the count is red with `@` (dec). | R2, D31, D86, D191, D219, D246, D598, D615; roomlist.md |
| FR-021 | Must | Direct chat creation: never twice for one person, never becomes a group; says if encrypted. | Given an existing direct room with Bob, When the user starts a chat with Bob, Then that room opens and no second room is created (dec). | R14, D291, D397; roomlist.md |
| FR-022 | Must | Unread computed by the client: main timeline plus followed threads; "Mark as read" sends receipts for main and unread threads; "Mark as unread" shows a dot. | Given an unread thread and main messages, When Mark as read is chosen, Then receipts go for the main timeline and each unread thread (dec). | R21, D615, D558; notifications.md. See OQ-07 (SDK counts). |
| FR-023 | Should | Row states: hover, selected row to column edges, AA text colours of the NavItem. | Given a selected row, When shown, Then it reaches both column edges (dec). | R3, D230, D241, D603 |
| FR-024 | Should | Filter chips: All, Unread, Broadcasts, Directs, Spaces, Favourites, then user folders; 32 px pills. | Given the list, When filter chips are shown, Then the order is All, Unread, Broadcasts, Directs, Spaces, Favourites (e2e). | R4, D191, D221, D375; roomlist.md |
| FR-025 | Should | Space row opens a card with its channels; a new space comes with "General"; sub-spaces the member may join show before joining. | Given create > space > name, When submitted, Then a space card shows with a channel "General", and opening it shows its conversation (e2e). | R7, D170, D542; roomlist.md |
| FR-026 | Should | Row menu (right click, "..."): mark read/unread, mute, pin, favourite, folder, leave/delete chat, report. | Given a direct room, When the row menu opens, Then "Delete chat" is offered and "Leave" is not (e2e). | R8, D107, D150, D231, D247; roomlist.md |
| FR-027 | Should | Room-list search over a local index of all rooms: sections Groups, Direct messages, Messages, Global; matched words marked; public directory. | Given the typed word "bob", When searching, Then matching rooms and contacts show with the word marked (dec). | R10, D147, D199, D503, D504; roomlist.md |
| FR-028 | Should | Invitations: own-server invites joined at once (never from an ignored person); other-server invite waits in an Invites section with Join / Decline and inviter id. | Given an invitation to a room on the own server, When it arrives, Then it is joined at once: a row appears and no invite element shows (e2e). | R11, D235, D248, D556; roomlist.md |
| FR-029 | Should | Delete chat hides history for the user alone (account data `app.twake.chat.deleted_chats`), with undo; a direct room is deleted, not left. | Given Delete chat on a room, When confirmed, Then its row leaves the list and history stays hidden until undone (dec). | R15, D186, D225, D374 |
| FR-030 | Should | Room list cache and progressive loading: first request 30 rooms plus the open room; widen by 100 per step every 250 ms up to 500; never wait for contact names more than 300 ms; skeleton until first rows. | Given 300 rooms, When the user scrolls to 80 percent of the list, Then more rooms load (dec); first paint does not wait for the address book. | R17, D503, D518, D533, D559; performance.md A.3 |
| FR-031 | Should | Sidebar header: title, count badge, search, bell (Activity), compose pen. | Given unread rooms, When the header renders, Then the badge shows the count (dec). | R18, D246, D318, D343 |
| FR-032 | Should | Join by link (`matrix.to`, `?via=`): preview via MSC3266 summary, join, read history. | Given a link to a room the user is not in, When opened, Then a preview shows and Join works through the `via` servers (dec). | I10, D581; info.md |
| FR-033 | Could | Folders (`u.<name>` tags), built-in filters as folders, section folding, selection mode with bulk actions. | Given a new folder with 2 chats ticked, When created, Then a chip filters to them (dec). | R5, R6, R9, D237, D306, D348, D354 |
| FR-034 | Could | Browse public groups; archive of left rooms with rejoin; voice rooms listed with participants; tombstoned room leads to the old history. | Given the public directory, When opened, Then rooms not yet joined are listed (dec). | R12, R16, R19, R20, D202, D76, D61, D338 |

### EPIC-04 Timeline

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-035 | Must | Conversation header: avatar, name, subtitle (members, or `status · presence` for a direct room), video call button, search, "..." menu (Room info, Delete chat, Leave, Report; direct: Contact info, Block). | Given a room header, When another member types, Then the subtitle reads "bob is typing..." (e2e). | T1, D186, D315, D466, D473; timeline.md |
| FR-036 | Must | Message bubbles: author, avatar (32 px at the last message of a run, with tail), time and ticks inside the bubble, own messages right, rounded 16 px. | Given a run of 3 messages by Bob, When rendered, Then only the last carries avatar and tail (dec). | T2, D183, D500, D519; timeline.md |
| FR-037 | Must | Day dividers, "New messages" divider, system lines (created, joined, left, identity changed, missed call). | Given messages of two days, When rendered, Then a divider separates them (dec). | T3, D292, D380, D577 |
| FR-038 | Must | Scroll rules: follow new messages only at the bottom; open where the user left, else at first unread, else at bottom; stay put as images load. | Given 20 tall messages, When the room opens, Then it opens at the last message, 0 px from bottom; a reader scrolled to the top is left there (e2e). | T4, D83, D523; timeline.md |
| FR-039 | Must | History paging: 30 messages per page back; load earlier on scroll without click; jump to latest; no receipt while the window is not live. | Given 40+ messages, When the wheel scrolls up, Then older messages load until the first message is attached (e2e). | T5, D290, D319, D532, D605 |
| FR-040 | Should | Jump to an event (pin, search result, link) with one `/context` call and an anchored window; load newer messages until joined with live. | Given a pin 500 messages back, When opened, Then the window anchors on it without paging through the history (dec). | T6, D605; timeline.md |
| FR-041 | Must | Send states: local echo at once, clock while sending, failed with retry/discard; ticks: server tick, blue double tick = read. | Given Enter on a message, When the server confirms, Then the clock turns into a tick; on failure retry and discard show (dec). | T7, D494, D495, D522 |
| FR-042 | Must | Read on screen: a timeline reads up to the last fully visible message, only while the window is visible, used within 30 s and focused. | Given the window unfocused, When messages arrive, Then no receipt is sent (dec). | T9, D558, D575; timeline.md |
| FR-043 | Must | Message kinds rendered: text, emote, formatted HTML (sanitised), image, video, audio, file, voice, poll, location, quote, album, code, unsupported, undecryptable, call-thread root, missed call. | Given each kind, When received, Then it renders with its component and unknown kinds show "unsupported" (dec). | T20, D105, D403; timeline.md |
| FR-044 | Must | Undecryptable message shows why (SDK failure code and device state). | Given an undecryptable message, When rendered, Then it reads "Verify this device..." with the reason (e2e). | T21, D403 |
| FR-045 | Must | Mentions render as user pills, `@room` pill, room links `#name:server` that preview and join; contact names shown. | Given a mention of bob, When rendered, Then "bob" is a link to the matrix.to user (e2e). | T23, I11, D358, D401, D487, D491 |
| FR-046 | Should | Author shown with the current name and avatar; none in a 1-1; names cut at 100 characters. | Given Bob renamed, When old messages show, Then the new name shows (dec). | T24, D297, D351, D596 |
| FR-047 | Should | Typing indicator in the header subtitle. | See FR-035 (e2e). | T25 |
| FR-048 | Could | Read receipts "Seen by" avatars; private receipts option; jumbo emoji; presence dot and status line; link previews (none in encrypted rooms). | Given a message read by Bob, When the menu opens "Seen" on hover, Then readers show (e2e). | T8, T10, T22, T26, D55, D499, D211; timeline.md |
| FR-049 | Could | Live announcements: new message read aloud as "author: text"; timeline silent for older pages. | Given a new message from bob, When it arrives, Then a polite announcement "bob: hello" is made (e2e adapted from accessibility.md B24). | T27, D378, D433, D597; accessibility.md B23-B24 |

### EPIC-05 Composer and sending

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-050 | Must | Composer pill: "+" menu, placeholder "Message", emoji at right; round button = microphone when empty, Send when text. | Given the composer empty, Then a mic shows; at the first typed letter a round send arrow replaces it; emptied again the mic returns (e2e). | C1, D181, D167, D451, D463; composer.md |
| FR-051 | Must | Reply: quote above the bubble, click goes to the original; reply banner in the composer; an attachment sent during a reply answers it. | Given a reply in progress, When sent, Then the message quotes the original (dec). | T14, D88, D138, D276 |
| FR-052 | Should | Edit own message ("edited" mark, edit history), delete (shows "deleted"), moderators delete others'. | Given an own message, When edited, Then "edited" shows and history lists versions (dec). | T15, D77; timeline.md |
| FR-053 | Should | Formatting (bold, italic, strike, link, lists, quote, inline code, code block); always-shown bar; Markdown sent as HTML body. | Given selection "docs", When the user types "www.x.org" as link and confirms, Then the composer holds `[docs](https://www.x.org)` (e2e). | C2, D35, D97, D331, D564; composer.md |
| FR-054 | Should | Suggestions in their own layer: `@` people (`@room` last), `#` rooms, `:` emoji shortcodes, `/` bot commands; up to 25 mentions; Escape closes only them. | Given "@bo" typed, Then a suggestion contains "bob"; Enter inserts "bob " and the sent message renders a link to the user (e2e). | C3, D27, D431, D502, D607, D621; composer.md |
| FR-055 | Should | Drafts per room, kept across restarts. | Given text typed in room A, When the user switches room and back or restarts, Then the draft is restored; Enter empties it (e2e adapted). | C5, D382, D512; composer.md |
| FR-056 | Should | Disabled composer explains why ("Only moderators can post..."). | Given a broadcast room and a plain member, When opened, Then the composer is disabled with the reason (dec). | C7, D194 |
| FR-057 | Should | Typing notifications sent; focus returns to the field after send or cancel. | Given typing, When keys are pressed, Then a typing notice is sent (dec). | C8, C15, D477 |
| FR-058 | Should | Paste and drag-drop files ("Drop to send"), paste links. | Given a file dragged on the window, When dropped, Then the attachment dialog opens (dec). | C9; media.md |
| FR-059 | Should | Emoji picker: only emojis the system draws, recents, search field focused on open. | Given the picker open, When ":smi" is typed in the composer, Then shortcode suggestions show and Escape keeps the text (e2e). | C10, D276, D481, D514, D561 |
| FR-060 | Should | Voice message recording: `m.audio` with MSC3245 and waveform, recording bar, Opus. | Given the mic pressed, When recording stops, Then an `m.audio` message with waveform is sent (dec). | C12, D54; composer.md |
| FR-061 | Could | Slash commands `/me`, `/shrug`, `//`; other `/word` refused; `/discardsession` local. | Given "//", When sent, Then one "/" is sent (dec). | C4, D27, E8 |
| FR-062 | Won't | GIF picker via twake-common-proxy; owner can turn GIFs off per channel. | Deferred: needs the proxy. | C11, D63, D146 |
| FR-063 | Won't | Scribe rewrite chip; mail compose from a profile e-mail; Twake Drive picker. | Deferred: external Twake services. | C13, C14, M10, D53, D65, D36 |

### EPIC-06 Encryption and recovery

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-064 | Must | Rust crypto via matrix-sdk with a SQLite store sealed by a passphrase kept in the keychain; a store that diverges from the server's device is abandoned and a new device signs in. | Given a store whose device keys differ from the server's, When the app starts, Then it abandons the store and signs in with a new device (dec). | E1, D8, D218, D282; encryption.md |
| FR-065 | Must | Recovery key held by ToM (`RecoveryVault`) and unlocked after sign-in; held in memory only; new account reserves the key before building secret storage. | Given a new device, When it signs in, Then it unlocks itself from the key ToM keeps and no security banner shows (e2e). | E2, D14, D279, D281; auth.md |
| FR-066 | Must | Set up recovery for a new account: cross-signing, recovery key, key backup; refuses to replace an existing one. | Given an account with a backup, When setup is asked, Then it fails with `already_set_up` (dec). | E3, D269, D271, D282 |
| FR-067 | Must | Encryption defaults: direct rooms in the clear with "Encrypt this chat"; groups clear unless the switch is on; assistant room encrypted; on for good, never off; never encrypted and public. | Given a new direct chat, When its empty card shows, Then it says "This conversation is not encrypted." with "Encrypt this chat" (dec). | E7, D203, D397 |
| FR-068 | Must | Never show "verified" the crypto did not confirm; own-device trust only for a signed assistant device. | Given an unverified device, When shown, Then it is not labelled verified (dec). | E10, D212 |
| FR-069 | Must | Encrypted attachments (AES-CTR, SHA-256 hashes) sent and read. | Given an encrypted room, When an image is sent, Then the file is encrypted and decrypts for another member (dec). | M13, D8 |
| FR-070 | Should | Device verification: recovery key, QR show/scan, emoji comparison; user-to-user emoji. | Given settings > security, Then the device list shows "This device" as Verified (e2e). | E4, D28, D427; encryption.md |
| FR-071 | Should | Devices list: rename, remove, block; notice names a new device. | Given a new device signing in, When the user is online, Then a notice names the device (dec). | E5, D87, D426, D457, D600 |
| FR-072 | Should | Identity reset: nothing destroyed before the server accepts; a device nothing can unlock is led to reset; identity-changed alert once, accept in one press, line in the conversation. | Given a device nothing can unlock, When signed in, Then a banner says no recovery key and no key backup, with a reset button (e2e). | E6, D269, D271, D284, D292 |
| FR-073 | Should | Restore history from key backup. | Given a backup, When restore is chosen, Then older messages become readable (dec). | E9, D281 |
| FR-074 | Could | `/discardsession`: next message uses a new session. | See FR-061. | E8 |

### EPIC-07 Message actions, pins and threads

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-075 | Should | Hover on a message: quick reactions (heart, thumbs up, thumbs down, laugh, wow, sad; recent first), Reply, "..."; right click opens the same menu at the pointer; text never moves on hover. | Given an own message, When hovering and pressing "...", Then the menu offers a quick reaction, more reactions, reply, edit, delete, select (e2e). | T11, D169, D234, D489, D492; timeline.md |
| FR-076 | Should | Message menu: Forward, Copy, Report, Pin/Unpin, Select, Reply, Start thread, Edit, Copy link, Save, Edit history, Seen by, Download, Delete (red, last). | Given the menu, Then Delete is last and red (dec). | T12, D163, D171, D183, D274, D391 |
| FR-077 | Should | Reactions as chips under the bubble; tooltip = emoji and three names. | Given a reaction by 4 people, When hovering the chip, Then emoji and three names show (dec). | T13, D118, D168, D524, D583 |
| FR-078 | Should | Forward to several rooms. | Given a message, When Forward is chosen with 2 rooms ticked, Then it is sent to both (dec). | T17, D234, D444, D585 |
| FR-079 | Should | Pins: several per room, pinned bar (opens all in the side pane when more than one), pin icon before time; far-back pins jump. | Given selected messages, Then bulk pin reads "Pin" (or "Unpin" when all are pinned) and pinned ones show a pin mark (e2e). | T18, D183, D229, D345, D461; timeline.md |
| FR-080 | Could | Multi-select bar: "N selected", Forward, Download, Copy, Delete, pin all; Escape leaves. | Given 3 selected messages, Then the bar says "3 selected" (dec). | T16, D183, D260 |
| FR-081 | Could | Saved messages (account data) with "Save" in the menu. | Given Save on a message, Then it appears in saved messages (dec). | T19, D59 |
| FR-082 | Should | Thread card under a root (count, last author/text/time, unread, resolved; no title) opens a floating thread window (420 px, 320 folded); several at once; expand into the centre column. | Given a root with replies, Then a thread card under the root shows no title (e2e). | TH1, D3, D173, D309, D406; threads.md |
| FR-083 | Should | Reply in a thread with typing and a composer that keeps its send button free. | Given a thread window, When a reply is sent, Then it appears in the thread and the card count grows (dec). | TH2, D372, D509 |
| FR-084 | Could | Resolve / reopen a thread. | Given "Mark as resolved", Then a check shows in the panel and on the card (e2e). | TH3, D56; threads.md |
| FR-085 | Could | Follow / mute thread; unread-thread banner; channel threads list; call thread (`app.twake.chat.call_thread`) as the call chat. | Given a muted thread, Then it adds no unread count (dec). | TH4, TH5, TH6, D60, D172, D105 |

### EPIC-08 Media and attachments

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-086 | Must | File chip (name, size), download, editable caption. | Given a file message, When the chip is activated, Then the file downloads (dec). | M9, D93, D136, D330 |
| FR-087 | Should | Attachment dialog: tile per file (video frame), caption, add or remove files before send. | Given several files, Then several tiles show, one can be removed, more added (e2e). | M1, D264, D342, D390; media.md |
| FR-088 | Should | Upload shown in the timeline with progress, stop, retry; thumbnails uploaded for large images and videos. | Given a 50 MB upload, Then progress shows with stop; on failure retry shows (dec). | M2, M6, D322, D513, D429 |
| FR-089 | Should | Image viewer full screen, dark, never enlarges, SVG drawn, download; media only under safe types. | Given an image sent, When clicked, Then the viewer opens with the image named by caption, "Download" and "Close" (e2e). | M4, D213, D227, D334, D389, D613 |
| FR-090 | Should | Video: plays only when the user starts it (muted autoplay is an option), first frame as poster, `.mov` handled, opens in the viewer. | Given a video sent, Then the tile shows play and duration and no video element plays on its own (e2e). | M5, D227, D259, D366, D525; media.md |
| FR-091 | Should | Audio file player and voice player, one sound at a time. | Given two audio messages, When one plays and the other starts, Then the first stops (dec). | M7, D258 |
| FR-092 | Should | Authenticated media (`/_matrix/client/v1/media`), on-disk cache, thumbnails cached; media leave memory off screen. | Given a media message, When it scrolls out of view, Then its decoded pixels may be freed and it reloads from disk cache (dec). | M11, D525, D526, D536 |
| FR-093 | Could | Image albums (images sent together form a grid), PDF viewer. | Given several images, Then Send makes one album with the text on the last one as caption (e2e). | M3, M8, D89, D364 |
| FR-094 | Won't | Locations rendered on a map; Twake Drive picker. | Deferred. | M12, M10 |

### EPIC-09 Room creation, info and management

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-095 | Should | Create page: group chat, feed, space, broadcast, then contacts A-Z; two-step group ("Add people", "Create a group"); encryption switch off by default; encrypted and public exclude each other. | Given step 2, Then encrypted and public exclude each other; a created encrypted group has `m.room.encryption` (e2e). | R13, D187, D203, D208, D256; roomlist.md |
| FR-096 | Should | Info pane on the Contact-info template: avatar, name, tiles Message/Video/Search/More, description, share link, tabs Members / Media / Links / Files, card with Matrix id, phone, e-mail and copy. | Given room info, Then tiles Video, Search, More show and no Message tile in a room (e2e). | I1, D75, D103, D161, D188, D233; info.md |
| FR-097 | Should | Edit view closed by "Done": picture, name, description, encryption switch (on for good), roles via Assign roles > Permissions, who can join. | Given the owner's Edit page, Then picture, name, description and encryption switch (off by default) show (e2e). | I2, D104, D203, D236; info.md |
| FR-098 | Should | Members: list, invite, remove, ban (only behind `enable_bans`, no reason), unban, set role, read-only member, ownership transfer. | Given a member profile in a group, Then Role shows (read only) with Message, Video, Search tiles (e2e). | I3, D95, D252, D346; info.md |
| FR-099 | Should | Notification level per room (all, mentions, none) in push rules; mute. | Given More > notifications, When "mentions" is chosen, Then the room rule changes (dec). | I5, D22, D162 |
| FR-100 | Should | Block user (ignore) and ignored list. | Given Block user, Then the person is ignored and listed in Settings (dec). | I7, D186 |
| FR-101 | Could | Shared content tabs, contact card (share, chats in common, edit name), report room or message, broadcast kind (`app.twake.chat.page`). | Given the Media tab, Then images of the room list with sender and date (dec). | I4, I6, I8, I9, D165, D299, D591, D90 |
| FR-102 | Won't | Public web page of a room, follow requests UI, Sentry feedback button. | Deferred. | I9, I13, D178, D388, D40 |

### EPIC-10 Search

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-103 | Should | In-room search: local index sealed on the device (never message text in clear), server search for rooms in the clear, match bar older/newer, search older. | Given the search index files, Then records hold the room id but never message text in clear; "COFFEE" finds "Who wants coffee?" (e2e). | S1, D21, D560, D611; encryption.md, timeline.md |
| FR-104 | Could | Quick switcher (Cmd+K): conversations unread first then recent; typing filters and adds public-directory channels of the own server; Enter opens. | Given the switcher open and empty, Then unread rooms are listed first, then the most recent (dec). | S3, D42, D515 |

### EPIC-11 Notifications and badges

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-105 | Must | Local notifications for live decrypted events, once per event, sound on by default, click opens the room or message. | Given a message in a non-open room, Then a system notification shows the sender and text; a click opens that message (dec). | N1, D22, D71, D137, D240; notifications.md |
| FR-106 | Should | Notification settings: default levels, switches for mentions, invitations, member events, notices, `@room`; message text shown by default. | Given settings > notifications, Then "show message text" is checked by default (e2e). | N3, D85, D401 |
| FR-107 | Should | Dock badge with the unread count. | Given 5 unread rooms, Then the Dock badge shows 5 (dec). | N5, D66 |
| FR-108 | Could | Push with the app closed (Matrix pusher with `append`, APNs/UnifiedPush gateway); ring notification with an answer button. | Given the app closed, Then a push shows sender and room (dec). Needs a backend gateway (OQ-09). | N2, D62, D70, D578 |
| FR-109 | Won't | Notification centre in the app bar. | Deferred. | N4, D126 |

### EPIC-12 Calls

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-110 | Should | MatrixRTC signalling and LiveKit media with the token from the side service `/sfu/get`; media connected before the user is announced. | Given a started call, Then the call stage shows within 30 s (e2e). | CL1, D25, D30, D179, D387; calls.md |
| FR-111 | Should | Call UI: tiles wrap on rows, shared screen spans the whole width, own camera mirrored, conversation column (380 px), device menus for mic and camera chosen during the call and remembered. | Given two people in a call, Then the stage is wider than twice the conversation column and the list folds (e2e). | CL2, D570, D573, D574; calls.md |
| FR-112 | Should | Ring: incoming call notification rings 60 s, once; Decline tells the caller; missed call inferred client-side. | Given a ring unanswered for 60 s, Then it stops and a missed call line shows (dec). | CL3, D106, D577, D579; notifications.md |
| FR-113 | Should | Floating call window: drag and resize to the edges, corner menu, sizes persisted, Full screen, Detach. | Given a live call, When Detach is pressed, Then the call goes to the window and the conversation takes the room back (e2e). | CL5, D571, D576 |
| FR-114 | Should | One client holds the call; a vanished client leaves; a call ends when the last membership expires. Voice and video buttons in headers; permission failures reported. | Given a camera permission refusal, Then an error toast explains it (dec). | CL8, CL9, D131, D609, D184 |
| FR-115 | Could | Encrypted call when its room is encrypted (media keys by to-device). | Given an encrypted room, Then the call is encrypted and keeps only raised hand of Meet (dec). | CL4, D430, D216 |
| FR-116 | Could | Pop out of the call into a separate always-on-top window (button only; no tab-switch trigger on native). | Given a call, When "Pop out" is pressed, Then an always-on-top window shows it (dec). | CL6, D618, D619 |
| FR-117 | Won't | Meet extras (recording, captions, transcription) and voice rooms joined from the row. | Deferred. | CL7, PR2, D30, D572, D61 |

### EPIC-13 Multi-account

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-118 | Should | Up to 5 Matrix accounts at once on any homeserver, keyed by user id; one on screen, others in background (sync, invitations, notifications, rings). | Given the account menu, When "add account" is done with a password, Then a second account shows with its own rooms and switching back shows only the first's (e2e). | AC1, D373, D396; multi-account.md |
| FR-119 | Should | Add account by the server's `/login`; a login token the deployment issued is never handed to another server; an account whose server part does not match its homeserver is refused. | Given a token the deployment accepts, Then the new device is signed out and the dialog says so (dec). | AC2, D381, D395, D425 |
| FR-120 | Should | Account switcher in the account menu; changing account ends a call; only the first account of the configured homeserver gets deployment services. | Given a call, When the account changes, Then the call ends (dec). | AC3, AC4, D373 |
| FR-121 | Could | Per-account unread and badges for background accounts. | Given an unread in account B, Then B's badge shows in the switcher (dec). | AC5, D373 |

### EPIC-14 Contacts, activity, polls and presence

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-122 | Could | Contacts list A-Z from ToM address book, directory search, each opening its direct room; create contact; invite by e-mail or phone; invite link with QR code. | Given the Contacts tab, Then rows show the address book, never the user; opening one opens the DM (e2e). | CT1, D78, D185, D205, D289; contacts.md |
| FR-123 | Could | Contact names shown everywhere and cached on device. | Given a cached name, When the app starts, Then it shows before the address book answers (dec). | CT2, D302, D358, D527, D546 |
| FR-124 | Could | Activity feed: mentions, replies, reactions, missed calls; filters; counts what the bell counts. | Given a mention by Bob, When the Activity bell is pressed, Then the mention row shows and opens the timeline at it (e2e). | AV1, D57, D58, D224, D577; contacts.md |
| FR-125 | Could | Polls MSC3381 (read stable and unstable, send unstable): several answers, voters shown unless anonymous, only the author ends. | Given a poll, When a vote is cast, Then the count updates (dec). | PO1, D54, D207, D344, D455; polls-status.md |
| FR-126 | Could | Presence dot, status (`status_msg`, 60 characters, presets) refreshed 15 s focused / 1 min unfocused for people on screen; status always resent on presence PUT. | Given a profile with a status, Then it shows, updated every 15 s focused (e2e). | PR1, D54, D80, D474, D624; polls-status.md |

### EPIC-15 Settings, personalization and accessibility

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-127 | Should | Settings screen (610 px blocks): Profile, Appearance, Notifications, Security, Privacy, Folders, Archive, Accounts, Ignored users, Help/Shortcuts, Analytics consent, Sign out. | Given settings, Then each section opens and Sign out ends the session (dec). | 2.16, D32, D41, D303, D304 |
| FR-128 | Should | Appearance: theme light/dark/system, density Comfortable/Compact, text size, room order, wallpaper, high-contrast palette, reduced motion, 24 h clock. | Given settings > appearance, Then choices exist for clock, density and order; compact shortens the rows (e2e). | 2.16, 2.20, D38, D272, D328, D437, D34; settings.md, theming.md |
| FR-129 | Should | Privacy: private receipts, autoplay, contacts visibility; shared settings with common Twake settings where defined. | Given private receipts on, Then no public receipt is sent (dec). | D55, D259, D78; settings.md |
| FR-130 | Should | Keyboard: one tab stop per conversation, F6 / Shift+F6 cycle regions, skip link, shortcuts dialog, focus ring 2 px in the primary colour, dialogs trap and restore focus. | Given the keyboard only, Then the user reaches a room, reads, reacts, crosses regions and writes (e2e). | accessibility.md B3, B5, B6, B9-B16 |
| FR-131 | Should | Screen reader (VoiceOver): every icon button has name and tooltip on hover and focus; toggles expose state; unread badges read with meaning; a message reads as author, text, time, read state. | Given VoiceOver, Then the timeline reads "bob: hello, 10:02, read" and badges read "3 unread messages" (dec). | accessibility.md B17-B22, T27 |
| FR-132 | Should | Interaction targets at least 24 px; layout holds at minimum window width and 200 percent text scale without clipping. | Given the minimum window width, Then no control is clipped or unreachable (dec). | accessibility.md B1, B4, D436 |
| FR-133 | Could | Personalize dialog (cards with preview, step by step); archive settings; question popups setting. | Given Personalize, Then a preview card shows per choice (dec). | D261, D272, D316, D349 |
| FR-134 | Could | Analytics consent bar and plumbing (account data `app.twake.chat.analytics_consent`); events sent to the side service only after consent; nothing before. | Given no consent, Then no analytics request leaves the app (dec). | 2.17, D5, D24, D383, D404, D476; analytics.md |

### EPIC-16 Distribution and cross-platform

The brief ties these to milestone 7. They come from the brief, the research (sections 5 and 6) and the native-equivalents table (inventory 5), not from a P-labelled inventory row.

| ID | Pri | Description | Acceptance criteria | Source |
|---|---|---|---|---|
| FR-135 | Must | Signed, notarized, stapled macOS `.app` in a `.dmg` (Developer ID, hardened runtime). | Given the release build, When `spctl --assess` runs, Then it reports "accepted, source=Notarized Developer ID" (dec). | brief; research 5 |
| FR-136 | Should | Auto-update with signed manifests (EdDSA) over HTTPS; replaces the new-version dialog. | Given a newer signed release, Then the app offers the update; an unsigned manifest is refused (dec). | A8, D594, D595; research 5 |
| FR-137 | Should | Windows and Linux builds compiled on every PR; release builds when GPUI platform support is verified. | Given a PR, Then CI builds the workspace on macOS, Windows and Linux (dec). | brief; research 6 |
| FR-138 | Could | Native menu bar and Dock menu on macOS; in-window menu on Windows and Linux; `twake://` deep links; Windows and Linux packages. | Given a `twake://` link, Then the app opens the room (dec). | research 6; inventory 5 |
| FR-139 | Won't | Embed mode inside Twake Space (`/embed/*`), platform bar, overlay page, service-worker and CSP mechanisms. | Not built on native. | embed.md, infra.md, D39, D43, D47, D64, D158 |
| FR-140 | Won't | Assistant, bots, Scribe auto-translate and suggestions (Twake Assistant features). | Deferred; see OQ-04. | B1-B5, T28, C13, assistant.md |

---

## Non-Functional Requirements

Non-Functional Requirements (NFRs) define **how** the system performs - quality attributes and constraints.

Reference = measure of the React app on its reference machine (Apple M5, 32 GB, local stack, `performance.md` A.2). Native targets reuse the same budgets so that "no worse than the web" is testable. Marked **proposed** where the sources give no number.

### Performance

| ID | Target | Value | Basis |
|---|---|---|---|
| NFR-001 | Idle resident memory, 1 account, 500 rooms, 10 s at rest | Under the React tab on the same account; absolute ceiling 250 MB RSS with 50 open rooms | Brief; absolute value proposed (research 4) |
| NFR-002 | Cold start to room list, warm store, M1 | Under 800 ms | proposed (research 4) |
| NFR-003 | First sign-in to first page of rooms | Under 1 000 ms (React: 653 ms; ceiling 1 000 ms) | performance.md A.2 |
| NFR-004 | Scroll 10 000-message timeline | No dropped frame at 60 fps; target 120 Hz on ProMotion | 60 fps requested; 120 Hz proposed |
| NFR-005 | One page of history (30 messages), tenth page back | Under 750 ms and not growing with depth (React: 390 ms) | performance.md A.2, A.5 |
| NFR-006 | Room switch, main thread blocked | Under 50 ms per switch | performance.md A.2 |
| NFR-007 | First sync request | 30 rooms plus the open room; widen 100 per step every 250 ms to 500 rooms; first paint waits for none | performance.md A.3 |
| NFR-008 | Room list never waits for contact names beyond 300 ms | 300 ms | performance.md A.3 |
| NFR-009 | Image cache | Decoded pixel LRU budget 64 MiB; fetch at most 128 ids per surface, 4 concurrent; decode off the UI thread | research 4, 7 (default proposed) |
| NFR-010 | Search in a 5 000-message room | First result under 1 000 ms | performance.md A.2 |
| NFR-011 | Encrypted room, new device, first message readable | Under 1 000 ms; 20 messages under 1 500 ms | performance.md A.2 |
| NFR-012 | Send echo | Enter to bubble on screen under 100 ms | proposed (mark `send-shown` exists on web) |
| NFR-013 | Jump to far message (pin/link) | Under 1 500 ms | performance.md A.2 |
| NFR-014 | Method | Every budget is a CI benchmark (divan, 10k-message fixture, diff-application bench); regression = worse than the median of the last 5 runs by more than 25 percent plus a floor, on two consecutive runs; ceilings enforced on the reference machine only | performance.md A.1; research 4 |

### Security

| ID | Target | Value | Basis |
|---|---|---|---|
| NFR-015 | Secrets | Access token, device id, store passphrase in the OS keychain (`keyring-core` behind a `SecretStore` port); passphrase 32 random bytes, zeroized in memory; no plaintext fallback | Brief; research 5 |
| NFR-016 | `unsafe` | `unsafe_code = "forbid"` at the workspace; `deny` only in `twake-platform` with `// SAFETY:` comments; zero in application crates | Brief; research 5 |
| NFR-017 | Supply chain | `cargo-deny` and `cargo-audit` clean on every PR; `Cargo.lock` committed, `--locked` builds; git dependencies pinned by `rev`; `cargo-vet` for crypto, auth and update crates | Brief; research 5 (vet scope proposed) |
| NFR-018 | Distribution trust | Developer ID, hardened runtime, notarization and stapling; Sparkle EdDSA-signed updates; no unsigned build shipped | Brief; research 5 |
| NFR-019 | Tokens | Recovery key in memory only; LiveKit and OpenID tokens in memory only; the Matrix access token never sent to services; https only | Inventory A10, D14, D210 |
| NFR-020 | Search index | Sealed on device with a key derived from the crypto store key; message text never in clear in the index; never sent anywhere | D21, D218 |
| NFR-021 | Public repository | No secret, token or key in the repository (secret scan in CI) | Brief; scan tool proposed |

### Quality

| ID | Target | Value | Basis |
|---|---|---|---|
| NFR-022 | Lints | `cargo fmt --check`, `clippy --workspace --all-targets -D warnings`, tests with `nextest`, all in CI on macOS, Windows, Linux | research 6 (CI matrix) |
| NFR-023 | Complexity ratchet | Function complexity at most 10 and file at most 300 lines in production code; frozen exceptions list that may only shrink | Web ratchet D582; values carried over, proposed |
| NFR-024 | Coverage | Line coverage on domain and application crates at least 90 percent; no drop on any PR | proposed |
| NFR-025 | Mutation | Mutation score on domain and application crates at least 80 percent on changed files (`cargo-mutants`) | proposed |
| NFR-026 | Port contracts | Each port has a contract suite run on its fake in `cargo test` and on its adapter against a real Synapse; a bug from a fake-adapter gap adds a case | D446; research 6 |
| NFR-027 | Architecture rules | Layers enforced by CI: only the Matrix adapter crate depends on matrix-sdk; only `twake-gpui` names the GPUI source; only the calls adapter depends on `livekit`; `block_on` banned outside `twake-desktop` | Brief; research 7 (lint design proposed) |
| NFR-028 | UI testing | View logic tested with `#[gpui::test]`; `insta` snapshots of view models; `proptest` on read state and diff application; an offline preview mode with fixtures | research 6 |

### Interoperability

Support matrix. "Compile" = built in CI on every PR. "Release" = signed installer shipped.

| Topic | macOS | Windows | Linux |
|---|---|---|---|
| Status | Release first (milestone 7) | Compile from day one; release after verification | Compile from day one; release after verification |
| Minimum OS | macOS 14 (proposed) | Windows 10 22H2 or 11 (proposed) | Ubuntu 24.04 with Vulkan (proposed; Vulkan is required by GPUI) |
| GPUI | Metal; AccessKit on Zed main only | In crate; DirectX 11 per Zed blog; AccessKit on main | Wayland and X11, Vulkan; failures like `NoSupportedDeviceFound` documented |
| Secrets | Keychain | Credential Manager | Secret Service (unlocked provider required, clear error otherwise) |
| Sign-in | System browser session and custom scheme / loopback | same | same (portal) |
| Notifications | `UNUserNotificationCenter` (signed bundle) | Toasts | XDG portal / D-Bus |
| Menus | Native menu bar and Dock menu | In-window menu | In-window menu |
| Calls | First (VideoToolbox) | After macOS acceptance | After macOS acceptance |
| Packaging | `.dmg`, Sparkle | NSIS or MSI | `.deb`, AppImage, Flatpak |
| Screen reader | VoiceOver, tested | Not promised at launch (GPUI issue #41138) | Not promised at launch |

- **NFR-029** GPUI source: one `twake-gpui` facade crate is the only place naming the GPUI source; the choice (crates.io 0.2.2, `gpui-unofficial`, pinned Zed git) is decided by a 2-day spike with a 10k-message list; a bump costs up to one person-week and is done monthly (proposed).
- **NFR-030** matrix-sdk and matrix-sdk-ui pinned to exact versions (0.19.1 at time of writing); upgrades in a dedicated PR with changelog read; one adapter crate; Matrix spec and `app.twake.*` names respected (D418).
- **NFR-031** Calls are in an optional crate behind `--features calls` so the libwebrtc build cost is optional; start macOS only.

### Accessibility

| ID | Target | Value | Basis |
|---|---|---|---|
| NFR-032 | Standard | WCAG 2.1 AA plus 2.2 target size at least 24 px; RGAA 4.1 reference; no compliance rate claimed until audited | accessibility.md |
| NFR-033 | Gate | macOS Accessibility Inspector audit clean on every screen, light and dark, as the native replacement for axe at the end of each acceptance test; VoiceOver pass per milestone | accessibility.md B1-B2; research 6 |
| NFR-034 | Reduced motion, high contrast, text scale | Honoured on every screen; high-contrast palette (light primary `#0055B8`, dark `#7CC0FF`) | D34, D437, D272; theming.md |
| NFR-035 | Windows and Linux screen readers | Not promised at launch (AccessKit immature in GPUI); tracked as upstream work | research 0 and 6 |

### Internationalization

- **NFR-036** Locales: en, fr, de, es, it, ru, vi (the web app's 7); English and French complete for the macOS beta, the other five before general availability (proposed split; web marks en and fr first).
- **NFR-037** Same 1 277 strings in 56 groups as `en.json`; Russian plural rules; language follows the OS; English fallback; a CI check fails when a key is missing in any locale (proposed).
- **NFR-038** Dates and numbers formatted by locale (`chrono`); keyboard input tested with US-International dead keys and CJK IME on each OS.

### Observability and analytics

- **NFR-039** Analytics: every user action is an event of one catalogue documented in a doc; sent through the side service (`POST /analytics/events`, batched, OpenID token) only after consent; consent in account data `app.twake.chat.analytics_consent`; nothing kept before; no third-party SDK imported (D404). A test fails when an event is undocumented.
- **NFR-040** Diagnostics: `tracing` spans in a ring buffer, slow-operation log (`log_if_slow`); crash and error reports carry no Matrix id (D214); crash reporting opt-in (proposed); Sentry not used at start.

---

## Epics

Epics are logical groupings of related functionality that will be broken down into user stories during sprint planning (Phase 4).

Each epic maps to multiple functional requirements and will generate 2-10 stories. Order follows dependency and the milestones of the brief (M2 skeleton and spike, M3 sign-in/list/read-only, M4 composer and E2EE, M5 threads/media/search/notifications, M6 calls and multi-account, M7 signed beta and other OSes).

| Epic | Name | Milestone | Goal | FRs |
|---|---|---|---|---|
| EPIC-01 | Foundation, design tokens and shell | M2 | Workspace, GPUI facade, tokens, shell, i18n, CI on 3 OS | FR-001 to FR-011 |
| EPIC-02 | Sign-in and session | M3 | SSO, keychain session, lifecycle | FR-012 to FR-018 |
| EPIC-03 | Room list, search and invites | M3 | Sync, list, unread, invites | FR-019 to FR-034 |
| EPIC-04 | Timeline | M3 | Read-only conversation, paging, read state | FR-035 to FR-049 |
| EPIC-05 | Composer and sending | M4 | Compose, send, edit, drafts, voice | FR-050 to FR-063 |
| EPIC-06 | Encryption and recovery | M4 | Crypto store, recovery, verification | FR-064 to FR-074 |
| EPIC-07 | Message actions, pins and threads | M5 | Menu, reactions, pins, threads | FR-075 to FR-085 |
| EPIC-08 | Media and attachments | M5 | Upload, viewers, cache | FR-086 to FR-094 |
| EPIC-09 | Room creation, info and management | M5 | Create flows, info pane, members | FR-095 to FR-102 |
| EPIC-10 | Search | M5 | In-room index, quick switcher | FR-103, FR-104 |
| EPIC-11 | Notifications and badges | M5 | Local notifications, Dock, push | FR-105 to FR-109 |
| EPIC-12 | Calls | M6 | MatrixRTC and LiveKit UI | FR-110 to FR-117 |
| EPIC-13 | Multi-account | M6 | Up to 5 accounts | FR-118 to FR-121 |
| EPIC-14 | Contacts, activity, polls and presence | M7 | Social surfaces | FR-122 to FR-126 |
| EPIC-15 | Settings, personalization and accessibility | M7 (a11y from M3) | Settings, density, VoiceOver, keyboard | FR-127 to FR-134 |
| EPIC-16 | Distribution and cross-platform | M7 | Signing, update, Windows and Linux | FR-135 to FR-140 |

There are 16 epics against a target of 12 to 15. EPIC-10 (Search, 2 FRs) may be folded into EPIC-03 at sprint planning, which gives 15.

---

## User Stories (High-Level)

User stories follow the format: "As a [user type], I want [goal] so that [benefit]."

These are preliminary stories. Detailed stories will be created in Phase 4 (Implementation).

1. As a daily user, I want to sign in with my company SSO so that I do not manage another password. (EPIC-02)
2. As a daily user, I want the app to remember my session so that it opens straight on my rooms. (EPIC-02)
3. As a user with hundreds of rooms, I want the list to appear fast and stay smooth so that I find a conversation at once. (EPIC-03)
4. As a reader, I want a long timeline to scroll without stutter so that I can go back in history. (EPIC-04)
5. As a writer, I want my message to appear at once and show when it is sent and read so that I trust delivery. (EPIC-04, EPIC-05)
6. As a security-minded user, I want my keys in the system keychain and my encrypted rooms readable on this device so that privacy holds. (EPIC-06)
7. As a team member, I want to react, reply, forward and pin so that I can handle messages without leaving the chat. (EPIC-07)
8. As a team member, I want threads in floating windows so that side discussions do not break the main flow. (EPIC-07)
9. As a user, I want to send and view images, videos and files so that I can share work. (EPIC-08)
10. As an owner, I want to create groups and spaces and manage members and roles so that I can organise my team. (EPIC-09)
11. As a user, I want to search a room and switch quickly with Cmd+K so that I find things. (EPIC-10)
12. As a user, I want notifications with a click-through and a Dock badge so that I do not miss mentions. (EPIC-11)
13. As a user, I want to call a room with camera, mic and screen share and keep the call in a floating window so that I can keep chatting. (EPIC-12)
14. As a consultant, I want several accounts at once so that I follow clients on different servers. (EPIC-13)
15. As a user, I want contacts, activity and statuses so that I see who is around and what I missed. (EPIC-14)
16. As a VoiceOver user, I want every control named and reachable by keyboard so that I can use the app. (EPIC-15)
17. As an administrator, I want signed, notarized, auto-updating builds so that I can deploy safely. (EPIC-16)

---

## User Personas

1. **Camille, daily Twake Chat user (primary).** Works on a Mac all day in 40 to 300 rooms, many unread, long threads, frequent calls. Wants speed and low memory; judges the app by start time and scroll feel. Needs: FR-019 to FR-044, NFR-001 to NFR-006.
2. **Hugo, multi-account consultant.** Uses 2 to 5 accounts on different homeservers; cares about notifications of background accounts and clear account switching. Needs: EPIC-13, FR-105, FR-107.
3. **Sam, security-conscious admin or user.** Encrypted rooms, device verification, recovery key; wants proof of signing and no plaintext secrets. Needs: EPIC-06, NFR-015 to NFR-021, EPIC-16.
4. **Alex, Windows or Linux user (secondary).** Same features; accepts later release and partial screen-reader support at launch. Needs: FR-137, interoperability matrix.
5. **Dev, Matrix developer (secondary).** Wants a fast open-source client on matrix-rust-sdk and a reusable Rust/GPUI design-system layer. Needs: NFR-027, FR-003.
6. **Noor, assistive technology user.** VoiceOver and keyboard only. Needs: FR-130 to FR-132, NFR-032 to NFR-035.

---

## User Flows

**Flow 1 - Sign in with SSO**
1. The user starts the app; the splash shows (FR-008). No session in the keychain.
2. The user enters or accepts the homeserver; discovery reads the well-known (FR-001, FR-012).
3. The SSO opens in the system browser session; the user authenticates.
4. The redirect returns a login token with the app's own state; a token without that state is refused (FR-013).
5. The app logs in with the token, stores session and store passphrase in the keychain (FR-014, NFR-015).
6. Crypto starts; the device unlocks from the key ToM keeps, or a new account sets up recovery (FR-064 to FR-066).
7. The room list appears.

**Flow 2 - First sync and room list**
1. The cached rows paint if a store exists (FR-018); otherwise a skeleton shows (FR-008).
2. Sliding sync asks for 30 rooms plus the open room (FR-030, NFR-007).
3. Contact names are read in parallel and never block paint (NFR-008).
4. The list widens by 100 rooms per step, 250 ms apart, to 500 (FR-030).
5. Invitations of the own server are joined at once; others wait in the Invites section (FR-028).
6. Unread counts are computed (FR-022); Dock badge updates (FR-107).

**Flow 3 - Send a message in an encrypted room**
1. The user opens the room; the timeline opens at first unread, else at the bottom (FR-038).
2. The user types; drafts are kept (FR-055); typing notices go out (FR-057).
3. Enter: the bubble shows at once with a clock (FR-041, NFR-012).
4. The message is encrypted with the room session and sent; the clock becomes a tick.
5. When Bob reads on screen, the tick turns blue (FR-042).
6. If the identity of a member changed, an alert shows once and a line records it in the conversation (FR-072).

**Flow 4 - Thread**
1. The user hovers a message and chooses "Start thread" (FR-076).
2. A floating thread window opens (420 px) over the panes (FR-082).
3. The user writes a reply; the root shows a thread card with count and last reply (FR-083).
4. The user opens a second thread; both windows stay; expand moves one into the centre column.
5. The user resolves the thread (FR-084).

**Flow 5 - Call**
1. The user presses the video button in a room header (FR-114); permissions are asked.
2. MatrixRTC membership is sent after the LiveKit media is connected (FR-110).
3. Others hear a 60 s ring; Decline tells the caller (FR-112).
4. The call stage shows tiles, devices menu, conversation column (FR-111).
5. The user leaves to another room: the call moves to the floating window (FR-113).
6. The last member leaves or memberships expire; the call ends (FR-114).

---

## Dependencies

### Internal Dependencies

- EPIC-01 (workspace, `twake-gpui`, `twake-tokens`) before every UI epic.
- EPIC-02 and the EPIC-06 crypto store before EPIC-03 can read encrypted content.
- EPIC-03 and EPIC-04 before EPIC-05 (sending needs a timeline) and EPIC-07.
- EPIC-08 depends on EPIC-05 (composer attachments) and FR-092 media cache.
- EPIC-11 depends on EPIC-03 unread logic and EPIC-13 for background accounts.
- EPIC-12 depends on EPIC-06 (encrypted calls) and the optional `calls` crate; EPIC-13 ends calls on account change.
- EPIC-16 depends on EPIC-01 CI and on the GPUI source decision (OQ-01).
- `ARCHITECTURE.md` (next BMAD phase) must fix crate boundaries before sprint planning.

### External Dependencies

- Twake homeserver (Synapse) with sliding sync MSC4186 and SSO; ToM (contacts, recovery key vault); twake-chat-side-service (`/sfu/get`, analytics, assistants); LiveKit server; Meet.
- Crates: `gpui` (source undecided), `matrix-sdk` / `matrix-sdk-ui` 0.19.1, `livekit` 0.10, `keyring-core`, `image`, `rust-i18n` or `fluent`.
- Apple Developer ID and notarization account; Sparkle; Authenticode certificate for Windows.
- Figma Library Twake and twake-mui as the design source; Inter font.
- APNs/UnifiedPush gateway (backend, not owned) for FR-108.

---

## Assumptions

- The homeserver, ToM, side service and LiveKit endpoints behave as the React app expects (brief).
- Inter can be shipped with the app (to confirm, OQ-02).
- A spike confirms the GPUI source (OQ-01).
- The inventory (snapshot 2026-11-12, branch `fix/mention-list-25`) is current enough; DECISION.md is the source of truth when it and the inventory differ.
- Mismatch noted from the inventory (section 7): AGENTS.md describes Activity as a tab of the bottom bar (D182) while the code follows D318 (Activity is the bell of the list). This PRD follows the code (FR-005).
- The Rust client may rely on SDK unread counts instead of recomputing them, if the spike confirms it (OQ-07); FR-022 states the behaviour, not the mechanism.

---

## Out of Scope

- Mobile apps.
- Embedding the Meet iframe; calls use LiveKit directly.
- Embed mode in Twake Space and its contract; web mechanisms (service worker, Web Locks, CSP, VAPID web push, Document PiP triggered by tab switch).
- Won't Have in this release: GIFs, Twake Drive picker, mail composer, Scribe and auto-translate, assistant, bots and suggestions, Meet extras (recording, captions, transcription), voice rooms joined from the row, public room pages, Sentry feedback, platform bar, notification centre in the app bar (all P3).
- Windows and Linux release builds before GPUI platform support is verified; screen-reader parity there.

---

## Open Questions

| ID | Question | Owner / how to settle |
|---|---|---|
| OQ-01 | **GPUI source choice spike**: crates.io `gpui` 0.2.2 (no AccessKit), `gpui-unofficial`, or Zed git at a pinned rev with copied patches? Measure build time, binary size, 10k-message scroll, VoiceOver pass. | Clement; 2-day spike in M2 (research G1) |
| OQ-02 | **Inter font licence**: can Inter be bundled and redistributed in the app? twake-mui does not ship the font file. | Clement, Twake design team; check the licence terms and what the web app serves |
| OQ-03 | **Twake team consultation**: the React app owners are not yet consulted. Confirm scope, DECISION.md as the behaviour source, side-service and ToM contracts, and who owns native distribution. | Clement; before architecture sign-off |
| OQ-04 | **E2EE assistant verification**: the web app trusts the assistant device on the word of the harness, only if the attested key signed the device (D212). How is that done with matrix-sdk-crypto (local trust of a device, cross-signing check), and does it ship in v1 given the assistant is Won't Have? | Clement with the harness owner; spike on the SDK API |
| OQ-05 | Final targets for NFR-001, NFR-002, NFR-004 (120 Hz), NFR-012, NFR-024, NFR-025 (all proposed). | Clement; after the first benchmark on the same account |
| OQ-06 | Minimum OS versions (macOS 14, Windows 10 22H2, Ubuntu 24.04 are proposed). | Clement |
| OQ-07 | Do `SyncService` and `RoomListService` give unread and notification counts good enough that the client need not recompute read state (FR-022)? | Spike (research 8, Q2) |
| OQ-08 | Updater: Sparkle from Rust, or `cargo-packager-updater` / Velopack? `cargo-packager` last release 2025-11-27: does it notarize? | Spike (research 8, Q4) |
| OQ-09 | Push with the app closed needs a gateway (Sygnal / UnifiedPush) the Twake backend may not have. Who owns it? | Twake backend team |
| OQ-10 | Does `livekit` 0.10 build with rustls only; cost of the prebuilt libwebrtc in CI minutes and bundle size? | Calls spike (research 8, Q5) |
| OQ-11 | Keyring store crates per OS for `keyring-core`; keychain prompt after each rebuild (dev signing identity). | Spike (research 8, Q3) |
| OQ-12 | Tray and native menu next to the GPUI run loop on Windows and Linux. | Spike (research 8, Q6) |
| OQ-13 | Sign-in redirect on desktop: `ASWebAuthenticationSession`, system browser with custom scheme, or loopback? Does the Synapse SSO allow a non-https redirect? | Clement with the Synapse admin |
| OQ-14 | i18n library: `rust-i18n` (simple) or Fluent (plurals, Russian)? | Clement; decide at architecture |

---

## Approval & Sign-off

### Stakeholders

- **Clement** (owner, developer) - high influence; scope and design parity.
- **Twake Chat team** (React app owners) - medium influence; source of truth for behaviour; not yet consulted (OQ-03).
- **Twake design team** - Figma Library Twake, twake-mui, Inter licence.
- **Twake backend owners** (ToM, side service, Synapse) - service contracts and push gateway.

### Approval Status

- [ ] Product Owner
- [ ] Engineering Lead
- [ ] Design Lead
- [ ] QA Lead

---

## Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-11-12 | clement | Initial PRD |

---

## Next Steps

### Phase 3: Architecture

Run `/architecture` to create system architecture based on these requirements.

The architecture will address:
- All functional requirements (FRs)
- All non-functional requirements (NFRs)
- Technical stack decisions (OQ-01, OQ-08, OQ-13, OQ-14)
- Data models and APIs
- System components

### Phase 4: Sprint Planning

After architecture is complete, run `/sprint-planning` to:
- Break epics into detailed user stories
- Estimate story complexity
- Plan sprint iterations
- Begin implementation

---

**This document was created using BMAD Method v6 - Phase 2 (Planning)**

*To continue: Run `/workflow-status` to see your progress and next recommended workflow.*

---

## Appendix A: Requirements Traceability Matrix

| Epic ID | Epic Name | Functional Requirements | Story Count (Est.) |
|---------|-----------|-------------------------|-------------------|
| EPIC-01 | Foundation, design tokens and shell | FR-001 to FR-011; NFR-014, NFR-022 to NFR-031 | 9 |
| EPIC-02 | Sign-in and session | FR-012 to FR-018; NFR-015, NFR-019 | 6 |
| EPIC-03 | Room list, search and invites | FR-019 to FR-034; NFR-003, NFR-007, NFR-008 | 10 |
| EPIC-04 | Timeline | FR-035 to FR-049; NFR-004 to NFR-006, NFR-009 | 9 |
| EPIC-05 | Composer and sending | FR-050 to FR-063; NFR-012 | 9 |
| EPIC-06 | Encryption and recovery | FR-064 to FR-074; NFR-011, NFR-015, NFR-020 | 7 |
| EPIC-07 | Message actions, pins and threads | FR-075 to FR-085 | 8 |
| EPIC-08 | Media and attachments | FR-086 to FR-094; NFR-009 | 7 |
| EPIC-09 | Room creation, info and management | FR-095 to FR-102 | 7 |
| EPIC-10 | Search | FR-103, FR-104; NFR-010, NFR-013 | 3 |
| EPIC-11 | Notifications and badges | FR-105 to FR-109 | 4 |
| EPIC-12 | Calls | FR-110 to FR-117; NFR-031 | 8 |
| EPIC-13 | Multi-account | FR-118 to FR-121 | 4 |
| EPIC-14 | Contacts, activity, polls and presence | FR-122 to FR-126 | 5 |
| EPIC-15 | Settings, personalization and accessibility | FR-127 to FR-134; NFR-032 to NFR-040 | 8 |
| EPIC-16 | Distribution and cross-platform | FR-135 to FR-140; NFR-016 to NFR-018, NFR-021, NFR-029, NFR-030 | 6 |

Inventory IDs not given their own FR: A8 (offline banner and new-version dialog: folded into FR-136), T28 and B1-B5 (folded into FR-140), I11 (folded into FR-045), I12 (server side only), M12 (FR-094, Won't).

---

## Appendix B: Prioritization Details

Mapping used: P0 = Must Have, P1 = Should Have, P2 = Could Have, P3 and web-only = Won't Have (this release). Release 1 = Must plus Should (brief: "Everything in P0 and P1 of the feature inventory, on macOS").

Inventory P0 areas mapped to Must: A1-A3, A6, A9, A10 (FR-001, 002, 012-015); R1, R2, R14, R21 (FR-019 to 022); T1-T5, T7, T9, T14, T20, T21, T23 (FR-035 to 045, 051); C1 (FR-050); M9, M13 (FR-086, 069); E1-E3, E7, E10 (FR-064 to 068); N1 (FR-105); i18n en/fr and theming tokens (FR-003, 004, 009).

Deviations from the inventory priorities, with reason:
- FR-005, FR-006, FR-008, FR-011: shell, splash and single instance are Must because nothing else runs without them (A7 is P1 in the inventory; the shell has no row).
- FR-007: Inter is Should (licence pending).
- FR-135 (signed, notarized build) is Must: the brief's success criterion and a security NFR; no inventory row.
- A4 (UIA re-auth) P1 kept Should (FR-016); A5 P2 kept Could (FR-017).
- A8 (new version dialog) is web-only P2; replaced by auto-update (FR-136, Should).

Release slicing:
- Must (M3 to M4, plus FR-105 and FR-135): the Must rows above.
- Should (M4 to M7): all Should rows.
- Could: polish and advanced items; promoted only when a milestone has slack.
- Won't: FR-062, 063, 094, 102, 109, 117, 139, 140.

Counts: 140 FRs: Must 38, Should 68, Could 26, Won't 8; 40 NFRs; 16 epics.
