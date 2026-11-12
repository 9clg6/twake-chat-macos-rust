# Twake Chat: feature inventory for a native Rust/GPUI port

Source: `~/Developer/twake-space-chat` (React, read-only), snapshot 2026-11-12, branch `fix/mention-list-25`.
Goal: copy features and design 1:1. `Dnn` = entry of `DECISION.md` (631 headings, D1 to D627). Paths are relative to the React repo.

Priority legend: **P0** usable chat (login, room list, timeline, composer, send/receive, E2EE basics). **P1** expected daily features. **P2** advanced / polish. **P3** skip on desktop (web-only or embed-only).

Organisation (2nd pass, 2026-11-12): this file is the index plus the cross-cutting parts (screens, summary feature table, ChatApp/ports surface, priorities, web-only mapping, glossary, mismatches). The **final behaviour of every decision** (all 615 `## Dnn` headings of DECISION.md read; superseded chains resolved) and the **e2e acceptance criteria** live in `features/<area>.md`:

| Area file | Content |
|---|---|
| [auth](features/auth.md), [accounts](features/accounts.md), [multi-account](features/multi-account.md) | sign-in, session, startup; multi-account rules and lifecycle (D373) |
| [roomlist](features/roomlist.md) | room list, rows, filters, folders, spaces, create flows, search, invites |
| [timeline](features/timeline.md) | header, bubbles, menu, reactions, receipts, pins, scroll, read state |
| [composer](features/composer.md), [media](features/media.md), [threads](features/threads.md), [polls-status](features/polls-status.md) | composing, formatting, drafts, voice; attachments, video, viewer; thread windows; polls, presence, status |
| [info](features/info.md), [contacts](features/contacts.md) | room info, members, roles, settings; Contacts and Activity |
| [calls](features/calls.md), [notifications](features/notifications.md), [encryption](features/encryption.md) | MatrixRTC/LiveKit, floating window, rings; notifications/push; E2EE, recovery, verification |
| [assistant](features/assistant.md), [settings](features/settings.md), [theming](features/theming.md) | assistant/bots/Scribe (P3); settings and personalisation; theme, layout, motion, keyboard |
| [performance](features/performance.md), [accessibility](features/accessibility.md), [analytics](features/analytics.md) | budgets and figures; a11y acceptance criteria; event catalogue and consent |
| [embed](features/embed.md), [infra](features/infra.md) | Twake Space frame (P3, skip); infra-visible rules |

Reading rule: in an area file, an entry with `modified by: Dx` is changed by Dx; the latest entry wins. The section 2 tables below are the original summary rows (source file, priority); where a row and an area file disagree, the area file wins.

---

## 1. Screens and navigation

### 1.1 Routes (`src/ui/RoomsScreen.tsx` l.943-998)

| Route | Screen | Notes |
|---|---|---|
| `/rooms/:roomId/*` | Room list (left) + conversation (centre) + optional side pane (right) | Query: `?event=` jump to message (D605), `?thread=` thread window (D3), `?user=` profile in side pane, `?via=` servers for a room not joined (D581), `?q=` search kept (D523), `?call=start` (D91, embed) |
| `/new`, `/new/:kind` | Create hub / create group, feed, space, broadcast | Replaces the room list; open room stays centre (D187, D208, D232, D314, D326, D340) |
| `/rooms/:id/new` | Create from a room | same |
| `/folders/new`, `/folders/edit/:name`, `/folders/builtin/:builtin` | Folder pages | D237, D308, D348, D354 |
| `/activity` | Activity (bell in the chat list) | D57, D58, D185, D224, D254, D318, D568, D577 |
| `/contacts` | Contacts | D185, D205, D254, D365, D376 |
| `/settings`, `/settings/:section` | Settings | D32, D41, D303, D304 |
| `/embed/rooms/:roomId`, `/embed/new/:kind`, `/embed/overlay.html` | Frame inside Twake Space | **P3** (D39, D144, D158) |
| `/login`, `/accounts/callback`, `/oidc/callback` | Sign-in, add account popup, Twake SSO | see 2.1 |
| `*` | `NoRoomScreen` | empty card (D294) |

### 1.2 Layout (`src/ds/ChatShell/ChatShell.tsx`, `layout.ts`, `src/ui/AppFrame.tsx`)

- Frame: app bar on top (logo, help menu, Twake apps, account menu; D29, D64, D379), bottom bar (Contacts, Chat, Activity, Settings; D182, D221, D615 filled icon for current tab, unread count on Chat, mentions on Activity). `ds/AppBar`, `ds/BottomNav`, `ds/ChatShell`. Chat always draws its own frame (D29). A button without action is shown disabled.
- Panes are square, no border, 4 px apart and from the app bar, flush with page edges (D194). Theme palette = twake-mui, brand blue `#0A84FF` (D245; D23 reversed).
- `PANE_WIDTH` = 360 px for the room list column and the side pane (D220). `MIN_CONVERSATION_WIDTH` = 360. `PANE_GAPS` = 24. Thread window 420 open / 320 folded, bottom offset 88. Call conversation column 380.
- **769 px rule**: from 769 px up the side pane (room info, profile) is a fixed 360 px column beside the conversation. Between 769 px and 1104 px (list 360 + conversation 360 + pane 360 + gaps) the pane floats over the conversation from the right at 360 px with a shadow; Escape closes it (D220). Below 769 px (`breakpoints.down('md')`): one pane at a time (phone layout: list, conversation, info stacked; `BackToList` arrow; `useSidePaneLevel` returns 1 narrow / 2 wide). A call folds the list.
- Threads: floating windows over the panes, several at once (D3, D173, D309); expand button moves a thread into the middle column (D406); pinned bar under an expanded thread (D438).
- Call: calls open in the room; leaving the room moves the call to a floating window (`ds/CallWindow`, D571), picture-in-picture (D618).
- Density: Comfortable / Compact (D328, D432, D464, D590), applies on screens wide enough (`ds/Density`).
- Toasts: twake-mui Snackbar centred in the main pane (D393, D488); errors of actions are toasts, not in dialogs (D591); composer error is a banner on top.
- Banners: one bar for all notices, paged by arrows (D356, `ds/BannerStack`): safety first, then what reads the room, then info.
- Keyboard: F6 regions, one tab stop per conversation, quick switcher (`ui/QuickSwitcher.tsx`), shortcuts dialog (`ui/ShortcutsDialog.tsx`), focus on view change (`PaneHeader focusKey`).

---

## 2. Feature list by area

Columns: **ID**, feature and behaviour, Dnn, React source, priority.

### 2.1 Auth / SSO / session

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| A1 | Matrix login via Synapse SSO: `buildSsoLoginUrl` then `loginWithToken` on `loginToken` of callback. Homeserver discovery (`.well-known`, SSO popup or password) | D4, D16 | `adapters/matrix/matrixLogin.ts`, `discoverHomeserver.ts`, `ui/LoginScreen.tsx`, `app/session.tsx` | P0 |
| A2 | Session persists across restarts, sealed at rest; device id kept so next sign-in reuses device and E2EE keys | D72, D218 | `adapters/matrix/persistentSession.ts`, `app/sessionMachine.ts` (pure state machine D442) | P0 |
| A3 | Sign out (soft/hard), session lifecycle machine | D442 | `app/sessionMachine.ts`, `ui/SettingsScreen.tsx` | P0 |
| A4 | Re-authentication (UIA) for cross-signing reset: SSO popup opened from the click | D20, D280, D425 | `app/ssoFallbackWindow.ts`, `app/ssoState.ts` | P1 |
| A5 | Twake SSO (OIDC PKCE) for Twake services identity (analytics sub, Drive, Scribe, platform bar) | D4, D469, D511 | `adapters/oidc/TwakeOidcSession`, `app/twakeSso.ts` | P2 |
| A6 | Sign-in link only accepted by the tab that asked (state) | D217 | `app/ssoState.ts` | P0 |
| A7 | Loading/splash screen (Flutter Lottie look), skeleton shimmer until first sync | D278, D428, D465, D518 | `ds/SplashLoader`, `ds/MessagesSkeleton` | P1 |
| A8 | Offline banner; new-version dialog (tab on older build offers reload) | D595 | `ui/OfflineBanner.tsx`, `ui/NewVersionDialog.tsx` | P2 (web) |
| A9 | Runtime config: `config.json` (client config) + `/.env.js` | D18 | `domain/clientConfig.ts`, `app/config.ts` | P0 |
| A10 | Tokens only to https services; error reports carry no Matrix id | D210, D214 | `adapters/http.ts` | P0 |

### 2.2 Accounts (multi)

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| AC1 | Up to 5 Matrix accounts at once, any homeserver, keyed by user id; one on screen, others in background (sync, invites, notifications, rings) | D373, D396 | `app/session.tsx`, `ui/BackgroundAccount.tsx`, `ui/accounts.ts`, `domain` `docs/multi-account-plan.md` | P1 |
| AC2 | Add account: server entry, SSO popup or password; refuses server part not matching homeserver; deployment token never handed to another server | D381, D395, D425 | `ui/AddAccountDialog.tsx`, `ui/AccountsSettings.tsx`, `app/accountRules.ts` | P1 |
| AC3 | Account switcher in the account menu; changing account ends a call | D373 | `ui/AppFrame.tsx` | P1 |
| AC4 | Only first account of the configured homeserver gets deployment services (push, side service) | D373 | `app/wireChatApp.ts` | P1 |
| AC5 | Per-account unread/badges for the others | D373 | `ui/BackgroundAccount.tsx` | P2 |

### 2.3 Room list, filters, folders, spaces

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| R1 | One list, no titles: pinned (favourites) first, then rooms by last message (not last event), spaces where their latest room would be, low priority at end. Optional "Group by kind" sections (Appearance) | D190, D101, D37, D310, D337 | `ui/RoomList.tsx`, `ui/rankRooms.ts`, `domain/roomListView.ts` | P0 |
| R2 | Row: avatar 32 px (20 px under a space, 24 px compact), name 14 px medium, last message 13 px, time 12 px, "You:" prefix, sender, media icon, tick, typing, last reaction, pin 16 px, unread count 16 px round (blue; red if mention; grey if muted), `@` for mention, kind badge (cube space, people channel, globe public, megaphone broadcast, mic voice, key encrypted) | D31, D86, D191, D219, D246, D275, D293, D325, D402, D589, D598, D601, D608, D615 | `ui/RoomListRow.tsx`, `ds/RoomRow`, `ds/RoomLine`, `domain/roomLine.ts`, `ui/roomRowText.ts` | P0 |
| R3 | Selected row reaches column edges; hover; text colours of NavItem twake-mui; AA contrast | D230, D241, D603 | `ds/RoomRow` | P1 |
| R4 | Filter chips: All, Unread, Broadcast, Direct, Spaces, Favourites + user folders (32 px pills, 12 px medium, chosen = light blue); fade shows they scroll | D191, D221, D375 | `ds/FilterChips`, `ui/ChatRows.tsx` | P1 |
| R5 | Folders (Telegram style): `u.<name>` tags, kinds of chats, exclusions, colour dot; built-in filters are folders (order, rename, delete; All first); chip "⋮" menu edits that folder; "New folder" page with chats ticked, toast on create | D237, D306, D308, D313, D348, D354, D454 | `ui/FolderPage.tsx`, `ui/FolderSettings.tsx`, `ui/useFolders.ts`, `domain/folders.ts` | P2 |
| R6 | Sections fold (Spaces, Direct messages, Channels), each takes a fifth of the screen, folded shows unread count; shows first rooms then pages | D37, D471, D483, D538, D545 | `ds/SectionTitle`, `ds/CappedList`, `ui/foldedSpaceUnread.ts` | P2 |
| R7 | Space row opens a card with its channels (not a conversation in Chat alone); new space comes with "General"; sub-spaces the member may join show before joining; children cached | D170, D172, D202, D277, D286, D542 | `ui/SpaceLanding.tsx`, `ui/SpaceChannels.tsx`, `ds/SpaceCard`, `domain/spaceTree.ts` | P1 |
| R8 | Room row menu (right click / "…" / long press on touch): Mark read/unread, Mute, Pin, Favourite, folder, Leave/Delete chat, Report | D107, D150, D231, D247, D478 | `ui/RoomRowMenu.tsx` | P1 |
| R9 | Selection mode (cross), bulk mute/pin/read | D247 | `ui/RoomSelectionBar.tsx`, `ds/SelectionBar` | P2 |
| R10 | Search of the list: local index of all rooms, sections Groups / Direct messages / Messages / Global; finds contacts and public directory; words marked; chips filter results; says when only part of history read; can search older | D147, D199, D228, D238, D392, D484, D503, D504, D547 | `ui/RoomSearch.tsx`, `domain/search.ts`, `ui/roomSearchMatches.ts`, `adapters/matrix/roomIndex.ts` | P1 |
| R11 | Invitations: own-server invites joined at once (not from ignored); other-server invite waits in "Invites" section with Join / Decline and inviter id | D235, D248, D556 | `ui/InvitesSection.tsx`, `ds/InviteRow`, `ui/LiveUpdates.tsx` | P1 |
| R12 | Browse public groups (planet icon in search field); public directory not yet joined | D202, D206, D416, D450 | `ui/PublicRoomsDialog.tsx` | P2 |
| R13 | Create page: group chat, feed, space, broadcast, then contacts A to Z; two-step group ("Add people", "Create a group"), avatar strip of picked people; encryption switch (off by default) | D187, D203, D208, D256, D314, D321, D407, D479, D480 | `ui/CreatePage.tsx`, `ui/CreateRoomScreen.tsx`, `ui/CreateGroupPeople.tsx`, `ui/PeopleSelect.tsx` | P1 |
| R14 | Direct chat creation: never twice, never becomes group; says whether encrypted and offers to encrypt | D291, D397 | `application/useCases/roomActions.ts` | P0 |
| R15 | Delete chat hides history for the user alone (account data `app.twake.chat.deleted_chats`), undo; direct is deleted not left | D186, D225, D374 | `domain/deletedChats.ts`, `ui/DeletedChatNotice.tsx` | P1 |
| R16 | Archive of left rooms (account data), Rejoin / forget | D76, D301, D458 | `ui/ArchiveSettings.tsx`, `domain/roomArchive.ts` | P2 |
| R17 | Room list cache for reloads; room index; skeleton until first sync read whole; skip rows far from view; first sync 30 rooms | D19, D503, D518, D533, D559 | `adapters/browser/StorageRoomListCache.ts`, `src/adapters/matrix/roomIndex.ts` | P1 |
| R18 | Sidebar header: title, count badge, search, bell (Activity), compose pen, planet | D246, D318, D343 | `ds/SidebarHeader` | P1 |
| R19 | Voice rooms listed with who is in the call | D61 | `ui/VoiceRooms.tsx` | P2 |
| R20 | Upgraded (tombstoned) room leads to old history; replaced room notice | D338 | `ui/RoomReplaced.tsx` | P2 |
| R21 | Unread computed by client: main timeline + followed threads; mark as read sends receipts for main + unread threads; "Mark as unread" shows dot | D615 | `domain/readState.ts`, `application/useCases/markRoomAsRead.ts` | P0 |

### 2.4 Conversation: header, timeline, messages

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| T1 | Header: avatar, name, subtitle (members or `status · presence` for DM), video call button (left of search), search, "…" menu (Report, Room info, Delete chat, Leave; DM: Contact info, Report, Block, Delete chat), phone: search+assistant move into "…" | D186, D315, D466, D473 | `ui/ChatPaneHeader.tsx`, `ui/RoomHeaderMenu.tsx` | P0 |
| T2 | Timeline list with bubbles: author, avatar (32 px at last message of a run, with tail), time + ticks inside bubble bottom right, own messages right, rounded 16 px, wallpaper light-blue paper (not inside Space) | D183, D194, D497, D500, D519, D99 | `ds/Timeline`, `ds/MessageBubble`, `ui/MessageItem.tsx`, `ui/MessageRun.tsx`, `ds/wallpaper.ts` | P0 |
| T3 | Day dividers, "New messages" divider, system lines (created, added, joined, left, identity changed, missed call) | D292, D380, D577 | `ui/TimelineHead.tsx`, `domain/conversationParts.ts` | P0 |
| T4 | Scroll rules: follows new messages only at bottom; opens where user left (per tab), else at first unread, else bottom; stays put as images load | D83, D523 | `ui/scrollMemory.ts`, `ui/useTimelineHistory.ts`, `ui/ChatPane.tsx` | P0 |
| T5 | History paging: 30 per page back; "Load earlier" only when messages wait; `loadNewerMessages` after a jump; no receipt while window not live; jump to date, jump to latest (round), jump to beginning | D290, D319, D532, D605 | `application/useCases/loadOlderMessages.ts`, `loadNewerMessages.ts`, `ui/JumpToDate.tsx` | P0 |
| T6 | Message anchor via `?event=` (pin, search result, link) with one `/context` call | D605 | `application/useCases/fetchChatView.ts`, `domain/conversationView.ts` | P1 |
| T7 | Send states: sent shows at once (local echo), clock while sending (replaces tick), failed with retry/discard; ticks: ✓ server, ✓✓ blue read | D494, D495, D522 | `ds/ReadTicks`, `adapters/matrix/localEcho.ts` | P0 |
| T8 | Read receipts: private receipts option; "Seen by" avatars under message (submenu of menu, 300 ms hover rest); no Seen in DM menu | D55, D327, D470, D472, D486, D501, D520 | `ui/MessageItem.tsx`, `ds/SeenBy`, `domain/message.ts` `lastReadBy` | P2 |
| T9 | Read on screen: a timeline reads up to last fully visible message, only while page visible, used in last 30 s and focused | D558, D575 | `ui/useReadOnScreen.ts`, `ui/useReadOnOpen.ts` | P0 |
| T10 | Jumbo emoji: a single emoji alone drawn big (1 emoji only since D499) | D226, D499 | `domain/jumboEmoji.ts` | P2 |
| T11 | Hover on a message: quick reactions above (D489, D493), Reply, "…"; right click opens the same menu at pointer; hover row hidden while menu open; text never moves on hover | D169, D234, D489, D492 | `ds/MessageToolbar`, `ds/HoverActionItem`, `ui/MessageItem.tsx`, `ui/useMessageActions.tsx` | P1 |
| T12 | Message menu (sections, Delete red last): quick reactions card + chevron to full picker; Forward, Copy, Report, Ask my assistant (never in Space), Translate (Scribe), Pin/Unpin, Select, Reply, Start thread, Edit, Copy link, Save, Edit history, Seen by, Download, Delete | D163, D171, D183, D274, D391, D130, D587 (menu), D588 | `ui/useMessageActions.tsx` (22 KB), `ui/MessageDialogs.tsx` | P1 |
| T13 | Reactions: chips under the bubble; tooltip = emoji + three names; faces of who gave it; quick set ❤️ 👍 👎 😂 😮 😢 (recent first) | D118, D168, D524, D583 | `ds/ReactionChips`, `domain/message.ts` Reaction | P1 |
| T14 | Replies: quote above bubble; click goes to original; reply inside composer; attachment sent during a reply answers it | D88, D138, D276 | `ui/MessageItem.tsx`, `ds/Composer` | P0 |
| T15 | Edit own message (shows "edited"), edit history; others' edits shown; deletion shows "deleted" line; moderators delete others' | D77 | `domain/message.ts`, `application/useCases/messageActions.ts` | P1 |
| T16 | Multi-select: dark bar on top ("N selected" / "N files"), Forward, Download, Copy, Delete, pin/unpin all, cross; Escape leaves | D183, D260 | `ui/MessageSelectionBar.tsx`, `ds/SelectionBar` | P2 |
| T17 | Forward to several rooms; list names space of a channel | D234, D444, D585 | `ui/ForwardRoomRow.tsx` | P1 |
| T18 | Pins: several pinned; pinned bar (opens all in side pane when >1); pin icon before time; pinned reply opens its thread; far-back pins jump | D183, D229, D345, D461 | `ui/PinnedBar.tsx`, `ui/PinnedView.tsx`, `ui/PinnedMessages.tsx`, `ds/PinnedBanner` | P1 |
| T19 | Saved messages (account data), "Save" in menu | D59 | `domain/savedMessages.ts` | P2 |
| T20 | Message kinds: text, emote `/me`, formatted HTML (sanitised), location, poll, voice, image, video, audio, file, GIF, album, code, quote, assistant question, missed call, legacy call, unsupported, undecryptable (reason shown), call-thread root | D105, D403 | `domain/message.ts`, `ds/RichText` | P0 |
| T21 | Undecryptable shows why (SDK failure code + device state) | D403 | `domain/encryption.ts` | P0 |
| T22 | Link previews under message (not on hover), none in encrypted room; wide image on top else small square; bare domain is a link; phone links | D211, D336, D347, D508, D362 | `ui/LinkPreview.tsx`, `ds/LinkPreviewCard`, `domain/webLinks.ts` | P2 |
| T23 | Mentions rendering: user pills, `@room` pill, room links `#name:server` previews+joins; name given to a contact shown | D358, D401, D487, D491 | `domain/mention.ts`, `ds/RichText` | P0 |
| T24 | Message author shown as they are now (current name/avatar); no sender name/avatar in 1-1; names cut at 100 chars | D297, D351, D596 | `ui/MessageRun.tsx` | P1 |
| T25 | Typing indicator | – | `application/useCases/typing.ts` | P1 |
| T26 | Presence dot on avatar, status line | D466, D468 | `ds/PresenceBadge`, `ds/StatusLine` | P2 |
| T27 | Messages read aloud: live announcements; keyboard position kept; Tab-reached message soft tint | D378, D433, D597 | `ds/LiveAnnouncement`, `ui/useAnnouncement.ts` | P2 |
| T28 | Auto-translate received messages with Scribe (off by default); per-message "Translate" for reader alone | D588, D617 | `ui/MessageTranslation.tsx`, `ui/autoTranslate.ts` | P3 |

### 2.5 Threads

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| TH1 | Discord-style: root message gets a card (count, last author/text/time, unread, resolved; no title) opening a floating thread window (420 px; folded 320); several windows at once; window expands to centre column | D3, D173, D309, D406, D485 | `ui/ThreadPanel.tsx`, `ui/ThreadConversation.tsx`, `ds/ThreadCard` | P1 |
| TH2 | Thread reply in thread; typing; composer in thread keeps send button free | D372, D509 | `ui/ThreadConversation.tsx` | P1 |
| TH3 | Resolve / reopen thread (resolved mark) | D56 | `ds/ResolvedMark`, `application/useCases/threads.ts` | P2 |
| TH4 | Follow / mute thread; unread thread banner; unread includes followed threads | D60 | `domain/threadFollow.ts`, `ui/UnreadThreadBanner.tsx` | P2 |
| TH5 | Channel threads list (threads of a space calls) | D172 | `application/useCases/fetchChannelThreads.ts` | P2 |
| TH6 | Call thread: chat of a call in a channel is a marked root thread (`app.twake.chat.call_thread`) | D105 | `adapters/matrix/callThread.ts` | P2 |

### 2.6 Composer

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| C1 | One pill: "+" menu (Add from device, Twake Drive, poll), placeholder "Message", GIF+emoji at right; round button = microphone when empty, Send when text (16 px icon, mic 24 px, white) | D181, D167, D451, D463 | `ds/Composer/Composer.tsx`, `ui/ConversationComposer.tsx`, `ui/useRoomComposer.tsx` | P0 |
| C2 | Formatting (Slack-like): bold, italic, strike, link (text + address), lists, quote, inline code chip, code block (typed ``` opens card at closing fence; fences on one line), always-shown bar (hidden on phone) | D35, D97, D331, D332, D490, D506, D564, D584, D602, D614 | `ds/Composer`, `adapters/matrix/markdown.ts`, `ui/composerFormatting.ts` | P1 |
| C3 | Suggestions float in own layer: `@` people (offers `@room` last), `#` rooms/groups/spaces, `:` emoji shortcodes, `/` bot commands; arrows keep highlight in view; Escape closes only them; mention list up to 25 | D27, D431, D502, D607, D621 | `ui/useMemberSuggestions.tsx`, `ui/useCommandSuggestions.tsx`, `domain/mention.ts`, `domain/emojiShortcode.ts` | P1 |
| C4 | Slash commands: `/me`, `/shrug`, `//` sends one "/", others refused; known bot command sent as `!command` (MSC4332); `/discardsession` local | D27 | `domain/botCommand.ts`, `application/useCases/botCommands.ts` | P2 |
| C5 | Drafts: per room, same across tabs, survive reload | D382, D512 | `ui/drafts.ts` | P1 |
| C6 | Reply / edit banner inside composer; edit with caption keeps mentions; placeholder stays on one line | D276, D448, D606 | `ds/Composer` | P1 |
| C7 | Disabled composer says why ("Only moderators can post...", read-only) | D194 | `ui/ChatPaneFooter.tsx` | P1 |
| C8 | Typing notifications sent | – | `application/useCases/typing.ts` | P1 |
| C9 | Paste/drag-drop files ("Drop to send"), paste link | – | `ds/DropZone`, `ui/useConversationUploads.tsx` | P1 |
| C10 | Emoji picker (only emojis the device can draw, versions, recents; opens inside screen; tab with GIFs) | D276, D481, D514, D561 | `ds/EmojiPicker`, `domain/emojiCatalog.ts`, `domain/emojiSupport.ts`, `ui/recentEmojis.ts` | P1 |
| C11 | GIFs via twake-common-proxy (OpenID token), re-uploaded as `m.image`; owner can disable per channel | D63, D146, D276 | `ui/useGifPicker.tsx`, `ds/GifPicker`, `application/ports/Gifs.ts` | P3 (needs proxy) |
| C12 | Voice message recording: `m.audio` + MSC3245 waveform, recording bar | D54 | `ui/useVoiceMessage.tsx`, `ds/VoicePlayer` | P1 |
| C13 | Scribe (Twake Assistant rewrite) chip, off in encrypted room unless owner turns on | D53, D73, D215, D626 | `ui/useScribe.tsx`, `domain/scribe.ts` | P3 |
| C14 | Mail compose window from a profile e-mail | D65 | `ui/MailCompose.tsx` | P3 |
| C15 | After send/cancel focus returns to field | D477 | | P1 |

### 2.7 Attachments, media, gallery

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| M1 | Attachment dialog: tile per file (with video frame), caption centred, round field | D264, D342, D390 | `ui/useAttachmentSender.tsx`, `ui/useAttachmentPreview.ts` | P1 |
| M2 | Upload shown in timeline with progress, stop, retry; waits for one upload; hides empty-chat card | D322, D513 | `ds/OutgoingAttachment` | P1 |
| M3 | Images sent together = one album (grid); each send own album; right-click on tile opens menu; hover row on tile edge | D89, D335, D414, D622, D627 | `ui/ImageAlbum.tsx`, `ds/AlbumGrid` | P2 |
| M4 | Image viewer full screen, dark, never enlarges; loading shows still; SVG drawn as PNG; media only under safe types | D213, D227, D334, D389, D613 | `ds/ImageViewer`, `ui/RoomImageViewer.tsx` | P1 |
| M5 | Video: plays only on user start, muted autoplay option, three sizes + banner slot, first frame, `.mov` and `file/video` handled, downloaded when played, opens in viewer; folded video stays in corner | D227, D259, D300, D312, D341, D366, D394, D525, D557, D587 | `ds/VideoPreview`, `ui/MessageItem.tsx`, `ui/useAutoplay.ts` | P1 |
| M6 | Thumbnail upload for large image/video | D429 | `adapters/matrix/matrixAttachments.ts` | P1 |
| M7 | Audio file player (one sound at a time); voice player | D258 | `ds/AudioPlayer`, `ds/VoicePlayer` | P1 |
| M8 | PDF opens in viewer | D364 | `ui/RoomPdfViewer.tsx` | P2 |
| M9 | File chip (name · size), download, caption editable and grows | D93, D136, D330 | `ds/FileChip` | P0 |
| M10 | Twake Drive pick: attach as file or link; shortcut never sent as file | D36, D200, D339 | `ui/useDrivePicker.tsx` | P3 |
| M11 | Media streaming through service worker for unencrypted; authenticated media (`/_matrix/client/v1/media`); thumbnails cached; media leave memory off screen | D525, D526, D536 | `ui/useMediaUrl.ts`, `domain/mediaAuth` | P1 (native: HTTP cache + decrypt) |
| M12 | Locations (`m.location`) shown | – | `domain/location.ts` | P3 |
| M13 | Encrypted attachments (AES-CTR + SHA256 v2 files) | D8 | `adapters/matrix/matrixAttachments.ts` | P0 |

### 2.8 Polls, voice, status, presence

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| PO1 | Polls MSC3381 (read stable+unstable, send unstable), several answers, voters shown unless anonymous, edit until a vote, only author ends, text ≤150 chars; vote encrypted where user may post; answer white on blue | D54, D207, D344, D414, D422, D455, D456, D460 | `ui/PollDialog.tsx`, `ui/PollMessage.tsx`, `ds/PollCard`, `adapters/matrix/polls.ts` | P2 |
| PR1 | Presence dot; status = `status_msg` of presence, presets (4 + holiday beach), 60 chars, "What are you up to?"; refreshed every 15 s focused / 1 min unfocused only for people on screen; visible tab keeps user online; PUT must always resend status_msg | D54, D80, D439, D462, D466, D467, D474, D475, D624 | `ui/StatusDialog.tsx`, `ui/useStatus.ts`, `adapters/matrix/MatrixPresence.ts` | P2 |
| PR2 | Voice room (`app.twake.chat.voice_room`): row joins/leaves call, shows participants, never rings | D61 | `ui/VoiceRooms.tsx` | P3 |

### 2.9 Search

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| S1 | In-room search: local encrypted index (device-sealed, worker), server search for rooms in clear, match bar (older/newer), search older; closing search does not move conversation | D21, D560, D611 | `ui/RoomSearch.tsx`, `ui/RoomSearchBand.tsx`, `ds/SearchMatchBar`, `adapters/indexeddb/` | P1 |
| S2 | Global (room list) search – see R10 | | | P1 |
| S3 | Spotchat / quick switcher: find people by name, rooms; Ctrl/Cmd+K | D42, D515 | `ui/QuickSwitcher.tsx`, `ds/Spotlight` | P2 |

### 2.10 Room info, settings, roles, members

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| I1 | Info pane (360 px) on "Contact info" template: back arrow, title, Edit; avatar, name, member count / last activity; tiles Message (not in room info D121) / Video / Search / More; description; share link w/ copy; tabs Members / Media / Links / Files (no counts; no Members in DM); card with Matrix id, phone, e-mail with copy, Leave chat, Block user | D75, D103, D161, D188, D189, D192, D233, D249, D562 | `ui/InfoPanel.tsx`, `ui/ProfilePanel.tsx`, `ds/PanelHero`, `ds/PanelTile`, `ds/PanelCard`, `ds/ProfileCard` | P1 |
| I2 | "Edit" = view of the panel closed by "Done": picture (visible pen), name, description, encryption switch (on for good), per-room switches (GIFs, Scribe, suggestions, public page), who can join / directory, public page cover+colour, roles via "Assign roles" › "Permissions" | D104, D203, D236, D267, D350, D516 | `ui/RoomEditPanel.tsx`, `ui/RoomSettings.tsx`, `ui/RoomPermissions.tsx`, `ui/RoomAccessSettings.tsx` | P1 |
| I3 | Members: list, invite, remove, ban (only behind `enable_bans`; no reason), unban, role set (Owner/Admin/Moderator/Member; "Member" = room default level), read-only member, transfer ownership (below room v12) / add owner (v12+) | D95, D248, D252, D346, D421, D593 | `ui/RoomMembers.tsx`, `ui/RoomOwnershipDialog.tsx`, `ui/AddMembersDialog.tsx`, `domain/roomManagement.ts` | P1 |
| I4 | Shared content: Media / Links / Files tabs; file saved by a press with sender+date | D165 | `ui/SharedContent.tsx`, `domain/sharedContent.ts` | P2 |
| I5 | Notification levels per room (all, mentions, none) in push rules; mute | D22, D162 | `ui/NotificationSettings.tsx`, `application/useCases/notifications.ts` | P1 |
| I6 | Contact card: Message, Video, Search, More; Share contact, chats in common (no Delete chat); e-mail icon; encryption switch on contact card; edit name of a contact (when in address book); add to contacts toast | D299, D302, D317, D355 | `ui/ContactCard.tsx`, `ui/EditContactDialog.tsx` | P2 |
| I7 | Block user (ignore), ignored list | D186 | `ui/BlockAction.tsx`, `application/useCases/roomActions.ts` | P1 |
| I8 | Report room / message (failed report keeps dialog) | D591 | `ui/ReasonDialog.tsx` | P2 |
| I9 | Room kinds: `discussion`, `broadcast`, `direct`; broadcast = `app.twake.chat.page`; follow requests; public web page opt-in (`app.twake.chat.public_page`) | D82, D90, D100, D109, D178, D388 | `ui/PageSettings.tsx`, `ui/RoomPublicPageSection.tsx`, `ui/BroadcastActions.tsx` | P3 (public page) / P2 (broadcast kind) |
| I10 | Join by link (`matrix.to`, `?via=`): preview via MSC3266 summary, join, read history | D581 | `domain/roomPreview.ts`, `ui/RoomUnavailable.tsx` | P1 |
| I11 | Address `#name:server` is a link that previews and joins | D491 | `domain/permalink.ts` | P2 |
| I12 | Public rooms stay on their server (`m.room.server_acl`) | D543 | synapse-module (server) | n/a |
| I13 | Support/feedback room, feedback button, Sentry reports | D40 | `ui/FeedbackButton.tsx`, `adapters/sentry` | P3 |

### 2.11 Contacts and Activity

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| CT1 | Contacts list A–Z, each opening its direct room; directory search (ToM), address book, create contact (first, last, Matrix ID), invite by e-mail/phone, invite friend with link and QR code; contact visibility (who sees e-mail/phone) | D78, D79, D185, D205, D257, D289, D365, D376 | `ui/ContactsScreen.tsx`, `ui/NewContactDialog.tsx`, `ui/ContactsVisibilitySetting.tsx`, `ds/QrCode` | P2 |
| CT2 | Names of contacts everywhere; own-contacts names cached, list shows without waiting | D302, D358, D527, D546 | `application/useCases/contactNames.ts`, `adapters/browser/StorageContactNamesCache.ts` | P2 |
| AV1 | Activity: feed of mentions, replies, reactions, inbox, missed calls; filters; rows like Contacts rows; counts what the bell counts | D57, D58, D224, D254, D568, D577 | `ui/ActivityScreen.tsx`, `domain/activityFeed.ts`, `ds/ActivityItem` | P2 |

### 2.12 Calls (MatrixRTC + LiveKit)

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| CL1 | MatrixRTC signalling (`MatrixCalls`), token service = side service `/sfu/get` (formerly ToM), LiveKit media (`LiveKitCallMedia`), media connected before user announced. No Element Call | D25, D30, D179, D387 | `adapters/matrix/MatrixCalls.ts`, `adapters/livekit/LiveKitCallMedia.ts`, `application/ports/Calls.ts`, `CallMedia.ts` | P1 |
| CL2 | Call UI: tiles wrap on rows, shared screen takes stage/whole width, own camera mirrored (selfie only), conversation column beside call (380 px), devices menu (arrows by mic/camera) chosen during call and remembered | D123, D133, D195, D309, D320, D570, D573, D574 | `ui/calls/CallView.tsx`, `ds/CallStage`, `ds/CallLayout`, `ui/calls/CallDeviceMenu.tsx` | P1 |
| CL3 | Ring: incoming call banner/notification "X is calling you in General", rings 60 s, once, decline tells caller, answered/declined quiet on all pages; ring event MSC4075 (`m.rtc.invite`, unstable `org.matrix.msc4075.rtc.notification`); ending a call never rings the ender; missed call inferred client-side | D106, D108, D127, D134, D324, D577, D578, D579 | `ui/calls/IncomingCallBanner.tsx`, `ui/calls/useIncomingRing.ts`, `adapters/matrix/missedCalls.ts`, `callRing.ts` | P1 |
| CL4 | Call encrypted when its room is (E2EE keys via media key to-device) | D430, D216 | `adapters/matrix/callEncryption.ts` | P2 |
| CL5 | Floating call window: drag/resize freely to screen edges, corner menu, sizes, "Full screen", "Detach"; band on phone; over thread windows; sizes persisted | D571, D576 | `ui/calls/FloatingCall.tsx`, `ds/CallWindow`, `domain/callWindow.ts`, `ui/calls/callWindowStorage.ts` | P1 |
| CL6 | Pop-out to system window (Document PiP) on tab switch; "Pop out of the tab" menu | D618, D619 | `ui/calls/PipCall.tsx`, `pipDocument.ts`, `popOut.ts` | P3 (native window instead) |
| CL7 | Meet features through token: recording, captions, transcription (transcript mode), raised hand (encrypted call keeps raised hand only) | D30, D572 | `adapters/meet/`, `application/ports/MeetingActions.ts` | P3 |
| CL8 | One page holds the call; vanished page leaves; call with nobody ends when last membership expires | D131, D609 | `app/*`, `ui/calls/useCall.ts` | P1 |
| CL9 | Voice/video buttons: DM header, each room header; calm call buttons; camera/mic permission failures reported | D184, D315, D375 | `ui/ChatPaneCall.tsx` | P1 |

### 2.13 Notifications / push

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| N1 | Local notifications for live events (decrypted, once per event, sound on by default, plays in background); click opens room/message | D22, D71, D137, D240 | `adapters/browser/BrowserNotifier.ts`, `ui/NotificationsWatcher.tsx` | P0 → macOS UserNotifications |
| N2 | Push with app closed: Matrix pusher (`append` for multi-account), named sender+room; encrypted push says so; ring notification answers from system | D62, D70, D71, D578 | `adapters/matrix/MatrixPushPusher.ts`, `public/` worker | P2 (APNs / UnifiedPush gateway) |
| N3 | Settings: default levels, switches for mentions, invitations, member events, notices, `@room` | D85, D401 | `ui/NotificationSettings.tsx`, `adapters/matrix/MatrixNotificationSettings.ts` | P1 |
| N4 | Notification center in app bar / apps first | D126 | `ui/AppFrame.tsx` | P3 |
| N5 | Dock/tab badge (unread count) | D66 | `ui/EmbedBadges.tsx` | P1 (Dock badge) |

### 2.14 Encryption, recovery, verification

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| E1 | Rust crypto (matrix-sdk-crypto) in IndexedDB, sealed with a key beside session; keys store wrongly diverged → new device | D8, D218, D282 | `adapters/matrix/startMatrixChat.ts`, `MatrixEncryption.ts` | P0 |
| E2 | Recovery key held by ToM (`TomRecoveryVault`) and unlocked after sign-in (`unlockDevice`); new account reserves key then builds secret storage; typed recovery key; held in memory only | D14, D279, D281 | `application/useCases/unlockDevice.ts`, `adapters/tom/` | P0 |
| E3 | `setUpRecovery`: cross-signing, recovery key, key backup for new account; refuses to replace existing | D269, D271, D282 | `application/useCases/deviceSecurity.ts` | P0 |
| E4 | Device verification: by recovery key, QR code (show/scan), emoji comparison; user-to-user emoji | D28, D427 | `ui/DeviceVerificationDialog.tsx`, `ui/VerifyDeviceDialog.tsx`, `ui/VerifyAction.tsx`, `ds/CameraScanner`, `ds/EmojiComparison` | P1 |
| E5 | Devices list: rename, remove, block (patched SDK), new-device notice names device; announced on others | D87, D426, D457, D600 | `ui/SecuritySettings.tsx`, `ui/NewDeviceNotice.tsx`, `application/useCases/devices.ts` | P1 |
| E6 | Identity reset: nothing destroyed before server accepts; device nothing can unlock is led to reset; identity changed alert once, accept in one press, line in conversation | D269, D271, D284, D292 | `ui/SecuritySettings.tsx`, `ui/SecurityBanner.tsx` | P1 |
| E7 | **Settled**: DM starts in the clear (D203 reverses D140); its empty card says "This conversation is not encrypted." with "Encrypt this chat" (D397); groups clear unless the "End-to-end encryption" switch is on, never encrypted and public; assistant room encrypted from creation; can be turned on later, never off | D203, D397 (D140 reversed) | `application/useCases/roomManagement.ts` | P0 |
| E8 | `/discardsession`: next message uses new session | D27 | `application/useCases/discardRoomSession.ts` | P2 |
| E9 | Key backup restore history | D281 | `deviceSecurity.ts` `restoreHistory` | P1 |
| E10 | Own-device trust only for signed assistant device; never show "verified" the crypto did not confirm | D212 | `domain/encryption.ts` | P0 |

### 2.15 Bots / Assistant / Scribe

| ID | Feature | Dnn | Source | P |
|---|---|---|---|---|
| B1 | Bot commands (MSC4332) in composer after `/`; bots `@bot_` / `@hermes_` prefixed; creator's commands for everyone | D27, D151 | `adapters/matrix/botCommands.ts` | P3 |
| B2 | User assistant provisioned by twake-harness via side service (`GET/PUT /assistants/me`), "My assistant" encrypted DM, trust/approval of its device, 503/429 retry-after | D98, D152, D212, D386 | `ui/MyAssistantSetting.tsx`, `ui/useMyBot.ts`, `adapters/sideService/` | P3 |
| B3 | Yes/no question card (Oui/Non under message, floating card, popups setting) | D96, D143, D193, D482 | `ui/AssistantQuestionHost.tsx`, `domain/assistantQuestion.ts` | P3 |
| B4 | Assistant suggestions (listener user invited, per channel switch, per-user setting), "Ask my assistant" quote | D74, D113, D139, D164 | `ui/RoomSuggestionsSection.tsx`, `ui/AssistantSuggestionsSetting.tsx` | P3 |
| B5 | Scribe rewrite / translate | D53, D588 | see C13 | P3 |

### 2.16 Settings (D32, D41, D303, D304; blocks 610 px wide; mobile: list → page)

`ui/SettingsScreen.tsx` sections: Profile/Account (D295, D323: Edit goes to common settings), Appearance (theme light/dark/system D38, density, text size D272, room order, wallpaper, high contrast D437, language), Personalize dialog (cards with preview, step by step; D261, D272, D316, D349), Notifications, Security (devices, recovery, identity), Privacy (private receipts D55, autoplay D259, contacts visibility D78), Folders (D348), Archive (D76), Accounts (D373), Assistant (D98), Ignored users, Help/Shortcuts, Analytics consent, Sign out. Source files: `AppearanceSettings.tsx`, `NotificationSettings.tsx`, `SecuritySettings.tsx`, `ProfileSettings.tsx`, `ArchiveSettings.tsx`, `AccountsSettings.tsx`, `PersonalizeDialog.tsx`, `QuestionPopupsSetting.tsx`, `AutoTranslateSetting.tsx`.

### 2.17 Analytics / consent (D5, D24, D26, D404)

User actions = events in `src/application/analytics/events.ts`, documented in `docs/analytics.md`; `HttpAnalytics` batches to `POST {side service}/analytics/events` with OpenID token; every click/screen/session captured (`ui/analyticsCapture.ts`); consent in account data `app.twake.chat.analytics_consent`, consent bar along the bottom (D383, D476). **P3** for the native port at start (keep consent plumbing; skip PostHog). Port: `Analytics`, `ConsentStore`.

### 2.18 Accessibility (docs/accessibility.md)

axe clean on every screen; targets ≥ 24 px, icon buttons hover 48 px (D436); tooltips (`ds/IconAction`) never native `title`; lists hold list items only; new view focuses title; high contrast mode (D437, `app/theme.ts` HIGH_CONTRAST_*), reduced motion (D34); screen reader announcements. Map to macOS AX roles/labels in GPUI. **P1**.

### 2.19 i18n

Locales: `en, fr, de, es, it, ru, vi` (`src/locales/*.json`, 1277 leaf strings in en.json across 56 groups: app, common, login, rooms, chat, info, security, emoji, create, people, invites, members, settings, space, session, verification, profile, devices, notifications, activity, theme, display, frame, analytics, call, assistant, switcher, format, voice, audio, voiceRoom, poll, status, drive, scribe, contacts, gif, recoveryKey, folders, personalize, accounts, help, roomLine, newVersion, ...). Language follows OS/common settings (D67, D68); default en when unsupported (`app/languages.ts`). Names start with capital (D209, D383). Buttons 1-2 words one line. **P0** (en + fr first).

### 2.20 Theming (`app/theme.ts`, twake-mui)

Light, dark, system; brand `#0A84FF`; colours from `theme.vars` so they follow dark (D245, D311); wallpaper = Figma light-blue paper (D183; none in embed D99); high contrast palette (light `primary #0055B8`, dark `primary #7CC0FF`); text size scale (`app/textScale.tsx`); density. Figma map of `ds` components in `src/ds/figma-map.json` (D196). **P0** tokens.

---

## 3. Domain and API surface to port

### 3.1 ChatApp use cases (`src/app/createChatApp.ts`, implementations `src/application/useCases/*.ts`)

Screens call these only; groups and functions keep names.

- **rooms**: createRoom, finishNewRoom, channelSuggestions, setChannelGifs, setChannelScribe, encryptRoom, setChannelPublicPage, startDirectChat, fetchInvites, acceptInvites, answerInvite, joinRoom, leaveRoom, fetchArchive, forgetRoom, reportRoom, fetchPermissions, fetchActionLevels, saveRoomSettings, fetchRoomAccess, saveRoomAccess, saveRoomAvatar, fetchSpaceChildren, fetchPublicRooms, fetchRoomPreview, fetchAllPublicRooms, markSpaceAsRead, fetchRoomList, fetchSyncState, openRoom, loadMoreRooms, fetchRoomListProgress, searchAllRooms, fetchSpaceRoomSummaries, markRoomAsRead, readOnScreen, fetchTimelineUnread, markRoomAsUnread, setNotificationLevel, setRoomTag, setRoomFolder, fetchFolders, saveFolders, deleteChat, undeleteChat
- **members**: inviteMember, removeMember, banMember, unbanMember, transferOwnership, setMemberReadOnly, fetchBannedMembers, setMemberRole, fetchMembers, ignorePerson, fetchIgnoredUsers
- **pages** (broadcast / public pages): fetchPublicPages, followPage, fetchFollowRequests, answerFollowRequest, fetchPageStyle, savePageStyle
- **messages**: fetchChatView, sendMessage, editMessage, deleteMessage, retryMessage, discardMessage, toggleReaction, copyMessageLink, fetchTypingNames, fetchBotCommands, notifyTyping, loadOlderMessages, loadNewerMessages, reportMessage, forwardMessage, fetchPins, setPinned, fetchSavedMessages, setSaved, fetchMessageVersions, fetchMessageReaders, searchMessages, searchOlderMessages, searchAllMessages, createPoll, votePoll, endPoll, editPoll, sendGif
- **attachments**: fetchMedia, fetchLinkPreview, sendAttachment, cancelAttachment, openAttachment, fetchAttachmentPreview, fetchAttachmentStreamUrl, fetchMediaAuth, fetchSharedContent
- **threads**: fetchThreadMessages, trackThreadOpened, replyInThread, fetchThreadResolution, saveThreadResolved, fetchThreadFollow, saveThreadPref, fetchChannelThreads
- **calls**: callControls (join, leave, mic, camera, screen share, devices, ring, decline, recording, captions, hand)
- **contacts**: fetchContactNames, searchContacts, fetchAddressBook, fetchPerson, addContact, updateContact, removeContact, fetchSentInvitations, inviteContact, fetchContactsVisibility, setContactsVisibility
- **assistant**: fetchMyBot, fetchMyBotAgain, recoverMyAssistant, fetchMyAssistantRoomId, openMyAssistant, askAssistant, watchMyAssistantKeys, inviteMyAssistant, requestMyAssistant, answerAssistantRequest, removeAssistant, fetchSuggestions, setSuggestions
- **encryption**: fetchIdentityNeedsApproval, approveIdentity, subscribeIdentityChange, fetchIdentityWarnings, discardRoomSession, fetchDeviceSecurity, verification (request, accept, emoji, QR, scan, cancel), fetchDevices, removeDevice, renameDevice, blockDevice, watchNewDevices, answerNewDevices, fetchOwnDeviceKey, resetIdentity, fetchRecoveryKey, verifyDevice, restoreHistory, setUpRecovery, unlockDevice
- **notifications**: watchNotifications, fetchActivity, tellIncomingCall, saveCallRingRule, saveAssistantRequestRule, fetchNotificationDevice, saveNotificationDevice, syncWebPush, fetchNotificationSettings, saveNotificationSettings, fetchPrivateReceipts, savePrivateReceipts, fetchAutoplay, saveAutoplay, fetchAutoTranslate, saveAutoTranslate
- **profile**: trackProfileOpened, fetchStatus, fetchPresence, subscribeStatuses, watchPresence, setStatus, setPreset, fetchOwnProfile, fetchOwnProfileEditable, saveOwnProfile
- **settings**: analyticsConsent, fetchSharedSettings, commonSettingsUrl, openSupport
- **tracking**: trackPage, trackUiAction
- top level: `subscribe(listener: (changes: Change[]) => void)`; plus bare ports `drivePicker`, `mailComposer`, `gifs`, `scribe` (guarded by `makeGuardedScribe`/`offersScribe`, state `app.twake.chat.scribe`).

### 3.2 Ports (`src/application/ports/*.ts`; contract suites in `ports/contracts/`, D446)

| Port | Purpose / methods |
|---|---|
| ChatRepository | `fetchSyncState`, `saveOpenRoom`, `saveRoomSearch`, `subscribe(changes)`, `subscribeToIncoming(notice)`, `fetchMedia`, `fetchLinkPreview`, `fetchMaxUploadSize`, `fetchAttachmentContent`, `fetchAttachmentStreamUrl`, `fetchMediaAuth`, `fetchPins`, `savePinned`, `fetchRoomThreads`, `fetchActivity`, `saveReadReceipts` |
| RoomList | `fetchRoomList`, `fetchRoomListProgress`, `saveMoreRooms`, `fetchSpaceRooms`, `fetchRoomFlags`, `saveRoomTag`, `saveRoomFolder`, `saveMarkedUnread`, `fetchRoomReadState` |
| RoomListCache | cached list rows for reload |
| Timelines | `fetchRoomMessages/Events`, `fetchHasOlder/NewerMessages`, `saveOlder/NewerMessages`, `fetchRoomInfo`, `fetchTypingPeople`, `saveTyping`, `fetchIgnoredUsers`, `saveIgnoredUser`, `fetchThreadResolution`, `saveThreadResolved`, `fetchMessageVersions`, `fetchMessageReaders`, `fetchThreadMessages`, `fetchBotCommands` |
| MessageSender | `saveMessage`, `saveThreadReply`, `saveAttachment`, `cancelAttachment`, `saveResend`, `saveDiscard`, `saveEdit`, `saveReaction`, `saveRedaction`, `saveReport`, `saveForward` |
| RoomManagement | rooms create/join/leave/forget/invite/answer/preview/archive, members invite/remove/ban/unban/level/ownership/list, permissions, action levels, settings/avatar, access, encryption, GIFs/Scribe/public page switches, space children, public rooms/pages, knock, follow requests, page style, assistant request/answer, report |
| Encryption | device security, own device key, device verification, recovery key creation/setup, history restore, identity approval/reset/change subscription, devices (list/remove/rename/block), trusted device, session discard |
| Verification | current verification, subscribe, request (device/user), user trust, accept, emoji start/compare, scan confirm/code, cancel |
| RecoveryVault | `fetchRecoveryKey`, `saveRecoveryKey`, `deleteRecoveryKey` (ToM) |
| Calls | focus URL, call server/token, join/leave, members, active rooms, rings, decline, thread root, left-at, media-key subscription, `subscribe` |
| CallMedia | `connect/disconnect`, E2EE key, mic/camera/screen share, devices list/switch, tiles, recording mode, captions, audio blocked/start, `attach` video element, `subscribe` |
| MeetingActions | start/stop recording, subtitles, transcription, hand raised |
| Presence | status/presence fetch, subscribe, subscribeShown/Focus, save status/preset |
| Polls | start, edit, response, end |
| NotificationRules | level per room, global settings, call-ring rule, assistant-request rule |
| Notifier | permission, preferences, show, open subscription, close, sound, ring |
| PushPusher / PushSubscriber | register/remove Matrix pusher; web push subscription |
| AccountPrefs | account-data store: thread follow, saved messages, private receipts, autoplay, auto-translate, dismissed questions, question popups, deleted chats, folders |
| OwnProfile | fetch/edit own name+avatar |
| Directory | ToM contacts: search, address book, own contacts, save/update/delete, invitations, person |
| ContactsVisibility, ContactNamesCache, UserSettings (common settings), ConsentStore, Analytics, Diagnostics (error report) | as named |
| SearchIndex / ServerSearch | local encrypted index save/delete/match; server-side search |
| BotProvisioning | assistant (side service): fetch/ensure bot, home room, recovery |
| AssistantQuestions / AssistantSuggestions | question stream; suggestions switch |
| Scribe, Gifs, DrivePicker, MailComposer | external Twake services (bare ports) |
| OpenId | OpenID token for side services |

Domain (`src/domain/*.ts`) pure logic to port with tests: `message`, `attachment`, `room`, `readState`, `conversationView`, `conversationParts`, `roomLine`, `roomListView`, `folders`, `spaceTree`, `mention`, `botCommand`, `poll`, `search`, `call`, `callWindow`, `callWatch`, `activityFeed`, `permalink`, `emojiShortcode/Catalog/Support`, `jumboEmoji`, `roomManagement` (roles/power levels), `encryption` (undecryptable reasons), `notifications`, `deletedChats`, `roomArchive`, `savedMessages`, `threadFollow`, `twakeServices` (well-known), `clientConfig`, `mediaUrl`, `mediaAuth`, `languageGuess`, `text`, `format`.

### 3.3 Adapters (replace by native)

`adapters/matrix/*` (all protocol; port to `matrix-sdk` Rust with sliding sync MSC4186 and `matrix-sdk-crypto`), `adapters/livekit` (livekit Rust SDK), `adapters/tom`, `adapters/meet`, `adapters/sideService`, `adapters/proxy` (GIFs), `adapters/drive`, `adapters/mail`, `adapters/oidc`, `adapters/browser`, `adapters/indexeddb`, `adapters/sentry`, `adapters/analytics`.

---

## 4. Suggested priority summary

- **P0**: A1 A2 A3 A6 A9 A10, R1 R2 R14 R21, T1-T5 T7 T9 T14 T20 T21 T23, C1, M9 M13, E1 E2 E3 E7 E10, N1, i18n en/fr, theming tokens (2.20).
- **P1**: accounts AC1-AC4, R3-R4 R7 R8 R10 R11 R13 R15 R17-R18, T6 T11-T13 T15 T17-T18 T24 T25, TH1-TH2, C2-C3 C5-C10 C12, M1 M2 M4 M5 M6 M7 M11, S1, I1-I3 I5 I7 I10, calls CL1-CL3 CL5 CL8 CL9, N3 N5, E4-E6 E9, a11y.
- **P2**: R5 R6 R9 R12 R16 R19 R20, T8 T10 T16 T19 T22 T26 T27, TH3-TH6, C4, M3 M8, PO1 PR1, S3, I4 I6 I8 I9, CT1 CT2 AV1, CL4, N2, E8, A4 A5 A8.
- **P3** (skip on desktop at first): embed/Twake Space frame (all `/embed/*`, D39, D43, D47, D64, D66, D94, D99, D103-D111, D114, D116, D119, D129, D135, D144, D149, D158, D164, D195, D594), GIFs, Drive picker, mail compose, Scribe/auto-translate, assistant/bots/suggestions/question cards, Meet extras (CL7), voice rooms, public pages, Sentry feedback, platform bar, notification center, PiP (CL6).

---

## 5. Web-only parts and native equivalents

| Web mechanism | Where | Native equivalent |
|---|---|---|
| IndexedDB session + sealed key ("key the browser cannot export") | D72, D218, `persistentSession.ts`, `storageKey` | macOS Keychain for access token + store key (`security-framework`); matrix-sdk SQLite store with passphrase from Keychain |
| Rust crypto in IndexedDB | `startMatrixChat.ts` | matrix-sdk-crypto SQLite store (same Rust core) |
| Shared worker (one client for all tabs, rpc.ts, port targets per account) D19, D373 | `app/matrixWorker.ts`, `app/rpc.ts` | Single process: Matrix client(s) in a tokio runtime, UI observes via channels; one client per account |
| Web Locks (one display of question card, store lock, one call page) | `app/storeLock.ts`, D131, D193 | Single-instance lock (flock on app-support dir) / process model; no per-tab logic needed |
| Service worker media streaming/thumbnails cache D526, D536 | `ui/mediaWorker.ts`, `app/thumbnailCache.ts` | On-disk media cache + range requests; decrypt encrypted media on a background task |
| Web Push + VAPID + `call-watch.js` | D22, D62, D70, `adapters/browser/BrowserPushSubscriber.ts` | APNs with a Sygnal/UnifiedPush gateway pusher (needs backend), or background sync + local `UNUserNotificationCenter` while running; ring notification action button |
| Notification API + sound | `BrowserNotifier.ts` | `UNUserNotificationCenter`, `NSSound`, Dock badge |
| Document Picture-in-Picture (D618, D619) | `ui/calls/PipCall.tsx` | Separate always-on-top `NSWindow` for the call; no tab-switch trigger; "Pop out" button only |
| Floating call window (CSS) D571 | `ds/CallWindow` | Secondary borderless GPUI window or in-window overlay |
| CSP, `connect-src`, frame headers, COOP (D210, D385, `docker/nginx/*`) | | Not needed; keep https-only rule for tokens (D210, D395) |
| OIDC/SSO popups, `/oidc/callback`, `/accounts/callback`, hidden-frame sign in (D511) | `app/embedSignIn.ts`, `ssoFallbackWindow.ts` | `ASWebAuthenticationSession` or system browser + custom URL scheme / localhost redirect |
| `getUserMedia` mic/camera/screen share, `MediaRecorder` voice messages | `useVoiceMessage.tsx` | AVFoundation + ScreenCaptureKit; Opus encoding to `m.audio` |
| `<video>`/`<audio>` playback | `ds/VideoPreview`, `AudioPlayer` | AVPlayer / gstreamer; first frame via AVAssetImageGenerator |
| Clipboard, drag and drop, paste files/links | `ds/DropZone` | NSPasteboard, drag destination |
| Browser BroadcastChannel / localStorage drafts, scroll memory, display prefs | `ui/drafts.ts`, `scrollMemory.ts`, `displayPreferences.ts` | Local SQLite / plist in app support |
| Window title, tab titles, `visibilityState` + focus | `routeTitle.ts`, D558, D575 | NSWindow key/main state, occlusion state |
| Service-worker/new-build reload prompt D594/D595 | `NewVersionDialog.tsx` | Auto-update (Sparkle) |
| `@linagora/twake-mui` (MUI theme) and Figma Library Twake | all UI | GPUI components with the same tokens; extract values from `twake-mui` and `src/ds/figma-map.json`; fonts (woff2) bundled |
| Emoji rendering support detection D481, D514 | `domain/emojiSupport.ts` | macOS renders all via Apple Color Emoji; keep version filtering optional |
| Web `Intl`, `Date` formatting | `ui/format.ts` | `chrono` + locale from system |
| Embed contract with Twake Space (`@linagora/twake-embed`) | D43, D47 | Skip |

---

## 6. Glossary

| Term | Meaning |
|---|---|
| Space (Twake Space) | The host product (Feed, Chat, Files, Tasks, Agenda, Mail, Contact tabs). Chat is its Chat tab; in Chat alone a "space" is a Matrix space shown as a card of channels (D170) |
| Matrix space / `Room.isSpace` | Room that contains rooms (`m.space.child`); has a "General" room created with it (D542) |
| Channel | A room inside a space or outside any; `Room.kind` = `discussion`, `broadcast`, `direct` |
| Discussion / Broadcast / Direct | Everyone can post / only moderators post (posting needs more than default power level; `app.twake.chat.page` for page rooms) / 1-1 room (`m.direct`) |
| Broadcast page | Room typed `app.twake.chat.page`, followable, optional public web page `/b/<slug>` (D90, D178) |
| Voice room | Room typed `app.twake.chat.voice_room`; row joins/leaves the call (D61) |
| Feed | A Create option: a room used as a feed (D187) |
| Folder | User chat group stored as tag `u.<name>`; built-in filters are folders too (D348, D354) |
| Favourite / Pinned room | `m.favourite` tag; pinned rows first in the list |
| Low priority | `m.lowpriority` tag, rows at the end |
| Pin (message) | `m.room.pinned_events`; several allowed (D229) |
| Saved message | Bookmark kept in account data (D59) |
| Thread window | Floating Discord-like thread view (D3, D173) |
| Call thread | Marked root `app.twake.chat.call_thread` in a channel; the chat beside a call (D105) |
| Ring / missed call | MSC4075 `m.rtc.invite` notification; missed call is inferred client-side (D577) |
| MatrixRTC / LiveKit / Meet | Call signalling (memberships) / media SFU / Twake Meet backend (recording, captions, transcription, hand) |
| ToM | Twake on Matrix server: contacts directory, recovery key vault, historically call token; `tom` adapters |
| Side service | `twake-chat-side-service` (`side_service.base_url` of the well-known): call tokens `/sfu/get`, assistants, analytics, public pages (D386-D388, D398, D405) |
| Harness (twake-harness) | Provisions each user's assistant (D386) |
| Assistant / Scribe / Suggestions listener | Per-user bot / text-rewrite service of Twake Assistant (D53) / bot invited to rooms in the clear that proposes events (D113) |
| Sliding sync (MSC4186) | Simplified Sliding Sync used instead of `/sync`; window of rooms, `REQUIRED_STATE` |
| Account data keys | `app.twake.chat.*` custom data (deleted_chats, analytics_consent, scribe, public_page, gifs, ...) |
| Recovery key | Secret-storage key; kept by ToM (`TomRecoveryVault`), held in memory only to verify (D14) |
| Cross-signing identity / device verification | Matrix E2EE trust; "Reset your identity" (D269), QR/emoji verification (D28, D427) |
| Bubble tail / run | Last bubble of a sender's consecutive messages carries avatar and tail (D500, D519) |
| Library Twake / twake-mui | Figma component library and MUI theme the UI follows (D196, D230, D245) |
| Compact / Comfortable | Density modes (D328) |
| Embed / frame | Chat inside Twake Space via `/embed/*` (P3 here) |
| D-number (Dnn) | Numbered product decision in the React repo's DECISION.md; latest wins |
| Spotchat | Quick-switcher style search of people and rooms (D42, D515) |
| Archive | List of rooms the user left, kept in account data (D76) |
| Deleted chat | History hidden for the user alone (`app.twake.chat.deleted_chats`, D186) |

## 7. AGENTS.md versus code (checked 2026-11-12, branch `fix/mention-list-25`)

Method: about 90 named symbols, numbers, routes, account-data keys and doc files grepped in `src/`. Only 3 mismatches; all in AGENTS.md (the code wins, D620).

| Claim in AGENTS.md | What the code shows | Severity |
|---|---|---|
| "History and recovery": the SSO re-authentication page of Synapse "not written yet" | Written: `makeSsoReauthenticate` in `src/adapters/matrix/ssoReauthenticate.ts:41`, wired as `reauthenticate` in `src/app/matrixSession.ts:172` | Medium (stale) |
| "Create": new group "with `NewRoom.isEncrypted`" | No such field: the port takes `options?: { isEncrypted?: boolean }` (`src/application/ports/RoomManagement.ts:44-48`); behaviour matches | Low (name) |
| `src/adapters/matrix/` is the only folder that may import matrix-js-sdk | Two integration specs in `src/adapters/tom/` import it (exempted in `eslint.config.js` ~l.411); production code clean | Low |
| Bottom bar "Contacts, Chat, Activity, Settings, with the mentions on Activity" (D182) | `app-frame.e2e.ts` l.178-201 says the bar has Contacts, Chat, Settings and Activity is the bell of the chat list, "no tab of the bottom bar" (D318 reverses D182 on this point; `FrameBottomNav` in `src/ui/AppFrame.tsx:95`). Use the D318 layout | Medium (stale) |

No DECISION.md decision was found contradicted by the code. Verified OK: ESLint complexity 10 and max-lines 300; 769/360 px; 30 messages a page; `MAX_ACCOUNTS` = 5; ring 60 s; presence 15 s / 1 min; 30 s still-there window; all cited routes and account-data keys. Not verifiable by grep: hover/menu details, Contact info tiles, PiP behaviour per browser, deployment claims.

## 8. Still unread / limits

- DECISION.md: all 615 `## Dnn` entries were read by sub-readers and distilled (one process entry, D554, skipped); `###` sub-headings were read inside their entries. Distillation is a summary: for pixel-level work reopen the cited Dnn.
- docs: read `performance.md`, `accessibility.md`, `accessibility-manual-checks.md`, `multi-account-plan.md`, `analytics.md`. Not read: `deploy.md`, `deploy-calls.md`, `services.md`, `release.md`, `dpia-assistant-suggestions.md`, `twake-mui-gaps.md`, `docs/research/*`, `implementation-plan.md` beyond its first 60 lines.
- e2e: bodies read for about 150 of 162 test files (groups A, B, C); skipped perf-*.e2e.ts, csp/analytics plumbing and the `.mjs` helper scripts.
- `src/ui/*`, `src/ds/*`, `src/domain/*` source: inspected by listing, props and doc comments, plus `message.ts`, `room.ts`, `ChatShell/layout.ts`, `theme.ts`; not read in full. Exact pixel values come from the Dnn entries and the Figma map (`src/ds/figma-map.json`), not from component code.
- `locales/en.json`: counted (1277 strings, 56 groups), not read string by string.
