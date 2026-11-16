# Accessibility (Acceptance Criteria) (from docs/*.md of the React repo)

Part of [feature-inventory.md](../feature-inventory.md).

---------------------------------------------------------------------------
Standard: RGAA 4.1 / WCAG 2.1 AA + WCAG 2.2 AA target size. State: not audited, no compliance rate claimed. Native macOS equivalent: NSAccessibility / VoiceOver; map web roles to AX roles.

### B.1 Automated gates (web) -> native equivalents
- B1. axe zero violations at end of every e2e test; WCAG 2.1 A/AA + 2.2 target >=24 px. Colour-contrast rule is OFF (brand colours of twake-mui used untouched, D245 reverses D23; high-contrast palette exists as a setting). Native: Accessibility Inspector audit clean on every screen.
- B2. Walk every screen in light AND dark: sign-in, room list, room + details, thread, person profile, activity, dialogs for settings, notifications, creation, security.
- B3. Keyboard-only flow: reach a room, read messages, react, cross regions, write; icon-button tooltip shows on focus and closes on Escape.
- B4. Layout at 320 px and 640 px (200% zoom of 1280): no horizontal scroll. Native: window min-width/zoom/text-size scaling without clipping.

### B.2 Landmarks / regions
- B5. One banner (app bar). Named regions: rail "Sections", list "Conversations", conversation, details. F6 / Shift+F6 cycle regions in that order, skipping hidden ones (on phone the rail shows with the list only).
- B6. First tab stop of framed screen = skip link (shown on focus) that skips the room list to messages (WCAG 2.4.1).
- B7. On phone-width single pane, the shown pane is the main landmark. One `h1` per screen, headings in order (manual check 9.1).
- B8. A frame button without an action is disabled, never hidden.

### B.3 Keyboard
- B9. Conversation = ONE tab stop; Arrow Up/Down, Home, End move message-to-message. Actions of a message = one stop, arrows inside. "..." menu named, opens on its first action. Emoji picker opens in its search field.
- B10. Focus ring on every keyboard-reached control: 2 px, primary colour (`focusVisible`), visible in both themes. Text field edge contrast 3:1. A button that waits keeps focus (`aria-disabled`, not `disabled`).
- B11. Opening a view focuses its title (`PaneHeader focusKey`); tab/window title names the view (room, Activity, settings section, Create).
- B12. Conversation or thread window opened by MOUSE focuses its message field (Slack-like); by keyboard or touch, stays on title. Room picked in Spotchat (Ctrl+K) focuses the message field, keyboard too (D509).
- B13. Closing a thread window returns focus to the thread root in the conversation. Deleting a message focuses the next one (else the previous).
- B14. Dialogs trap focus and return it on close. Exception: attachment dialog (opener menu item is gone) focuses message field, or the previously focused element if field wasn't focused (touch).
- B15. People picker: cross of a picked person named "Remove <name>"; picking/removing returns focus to search field.
- B16. No keyboard trap; every action reachable without mouse in sensible order (room list, foldable sections, message toolbar, "Start a thread", composer, emoji picker, call controls).

### B.4 Names, roles, states
- B17. Every icon button has an accessible name AND shows it as a tooltip on hover and keyboard focus (`ds/IconAction`, never native `title`). Tooltip persists under pointer, closes on Escape (WCAG 1.4.13).
- B18. Toggles keep name, expose state (`aria-pressed`): Microphone, Camera. Menu choices expose checked (`menuitemradio`). Section toggles expose expanded; tabs Media/Links/Files expose selected + controls panel; notification level checked; call controls pressed.
- B19. Lists contain list items only. Avatar initials hidden from AT.
- B20. A message is read as: author, text (or file name, or notice), time, read state, and the quoted message for replies. Image without caption: alt = file name (open question, manual check 1.2/1.3).
- B21. Unread badges read with meaning ("3 unread messages").
- B22. Composer: mention (@) and command (/) suggestion lists and the active item announced.

### B.5 Announcements (live regions)
- B23. Timeline is `role="log"` and SILENT (a loaded page of older messages is not read). Loading older messages silent.
- B24. New message of someone else announced in a hidden status ("bob: hello").
- B25. Polite statuses (`ds/LiveStatus`): "is typing", "Link copied", search count, file sending, search/filter results ("3 conversations found" in room list, "2 people found" in Contacts, items shown or "Nothing matches" in Activity; said only after a search or chip is set), empty list.
- B26. Errors: `role="alert"`; a field error linked to its field. Save buttons never disabled for a missing field: stay enabled, show helperText ("Give it a name.") and focus the field (room edit page, Create form). Person name field has `autoComplete="name"`; radio group named by its own label/legend ("Who can post" is a group with legend).
- B27. Notice (toast) stays 10 s, pauses under pointer, closes with a button. Errors of actions = toast, not dialog alert (memory note).

### B.6 Visual
- B28. Nothing conveyed by colour alone: unread = dot + bold; mention = "@"; muted = crossed speaker; selected row = bar; read ticks = shape; call speaker not only by outline (captions when Meet sends them).
- B29. Contrast of states and edges in both themes checked by eye: focus ring, field edges, selected bar, active tile. Light + dark themes mandatory.
- B30. Text at 200% zoom: nothing clipped. Text spacing (line 1.5, paragraph 2, letter 0.12, word 0.16): nothing cut; rail labels wrap. Target size >=24 px (WCAG 2.2).
- B31. "Reduce motion" setting removes transitions and animations (also honour OS setting). Display settings: text size small/normal/large, grayscale, high contrast.
- B32. Button labels: 1-2 words, one line, never truncated (memory note).
- B33. Seven languages; page/app language follows system. (de, en, es, fr, it, ru, vi.)

### B.7 Forms
- B34. Every field has a visible label (create room, recovery key, search, composer).

### B.8 Manual screen-reader pass (VoiceOver + Safari macOS/iOS, NVDA+Firefox, TalkBack) - criteria list above covers: 1.2/1.3, 3.1, 3.3, 9.1, 12.6, 12.7, 7.3/12.8, 10.7, 12.11, 13.x tooltips, 7.1, 7.5, 11.1/11.2/11.5/11.10, 10.4, 10.11, 10.12, 13.8, 13.1, 4.x. Native: VoiceOver pass.

### B.9 Known gaps (acknowledged)
No audit with real screen readers; HTML from other clients sanitised but unchecked structurally; image alt = file name; emoji picker (statement says 64 emojis no search - outdated, picker has search per B9); QR scan needs camera (emoji comparison fallback offered).

---------------------------------------------------------------------------
