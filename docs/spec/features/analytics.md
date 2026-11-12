# Analytics (from docs/*.md of the React repo)

Part of [feature-inventory.md](../feature-inventory.md).

---------------------------------------------------------------------------

### D.1 Path, transport
- Chat -> twake-chat-side-service `POST {side_service.base_url}/analytics/events` -> RabbitMQ `user-activity` -> twake-user-activity bridge -> PostHog. App NEVER talks to PostHog (D404; no posthog-js import, ESLint). Base from well-known `app.twake.chat.side_service.base_url` (or former `assistants.base_url`, D405); `ANALYTICS_URL` in `/.env.js` overrides (`POST {ANALYTICS_URL}/events`). Neither configured: nothing sent.
- Auth: `Authorization: Bearer <Matrix OpenID token>` + `x-matrix-server-name` (never access token). No `identify()`: server knows user by token.
- Body: `{ app:"twake-chat", version, events:[{name, properties, view?, at(ISO)}] }`. Answer 202, no body, NO retry.
- Batching: events wait max 5 s then leave together; page hidden -> flush at once (`fetch` keepalive; native: flush on app resign/terminate). Failure logged and dropped, never affects the action.
- Server side: checks token, skips bots (`@bot_`, `@hermes_`), looks up user's mail+org in LDAP, publishes CloudEvent `com.twake.chat.user.active.v1` per event (`data.action`=name, `data.properties`=props + `view` + `appVersion`); bad event dropped alone; strings cut to 200 chars. Bridge pseudonymizes address (keyed hash).
- Every event defined in `src/application/analytics/events.ts`, documented in docs/analytics.md (test fails if missing). Every user action is an event. Autocapture: every click/key activation on button, link, tab, menuitem, option, switch, checkbox, radio, summary, textbox sends `ui_clicked {target: static data-testid or 'unknown', role}` only; screens + visible time (`page_viewed`, `page_left`), session (`session_started`, `session_ended`).
- Common properties: `input` (pointer|keyboard|touch|auto), `roomId` (opaque Matrix id), `errorCode` (app/server code e.g. request_failed, M_FORBIDDEN; never message). Server-calling action emits `<action>` then `<action>_succeeded` / `_failed` (with errorCode).

### D.2 Consent
- Asked once by a bottom bar after first sync (account settings known); refuse = one click as accept. Also shown if only Sentry configured (`SENTRY_DSN` without analytics endpoint): same choice covers error reports and text says so; with both, one choice covers both (D40). Changeable anytime in Settings ("Usage measurement", "Error reports", or both).
- Stored in account data `app.twake.chat.analytics_consent` (follows user across devices; ToM reads it; `granted: true`). Checked by the app only.
- Before yes: nothing sent nor kept; later yes sends nothing older. After no: queued events dropped; refusal sends nothing, not even itself. Event `analytics_consent_given` on yes.
- Never sent: message text, file name, room/space name, topic, search query, e-mail, phone, display name, Matrix user id, token, URL with id, page content; no session recording. Page views carry route pattern only. Not tracked: reads, typing, automatic unlock after sign-in, image loads, refusal of consent.

### D.3 Catalogue by area (each `X` implies `X_succeeded`/`X_failed`/`X_*_failed` with errorCode where a server call; listed separately only when notable)

**Views / navigation / UI chrome**
- `page_viewed` {view: route pattern `/`, `/rooms/:roomId`, `/embed/rooms/:roomId`, `/activity`, `/contacts`, `/settings`, `/settings/:section`, `/new`, `/new/:kind`, `/folders/...`, `*`} - screen shown
- `page_left` {view, durationMs} - route change or page hidden (hidden time excluded)
- `ui_clicked` {target, role}; `session_started` (once per load, after consent); `session_ended` {activeMs} (page hidden)
- `performance_measured` {metric INP|LCP|CLS|FCP|TTFB|room_list|room_open, value ms (CLS x1000), sampleRate 1 or 10 for room_open}
- `room_opened`; `more_rooms_requested`; `voice_room_toggled` {isJoining}; `room_list_filtered` (first char of a search); `room_list_view_chosen` {all|unread|favourites|spaces|directs|broadcasts}; `space_folded`; `room_section_folded` {invites|spaces|channels|directs|low_priority}; `room_list_folded`; `info_panel_toggled`; `latest_messages_shown` {count}; `shared_tab_opened` {members|media|links|files}; `shared_media_opened`
- Call windows: `call_window_changed` {detached|expanded|moved|resized|popped_out|popped_in}; `call_channel_shown`; `thread_window_minimized`; `thread_window_expanded`
- Settings/display: `settings_opened` {what: screen|account|accounts|notifications|appearance|archive|folders|assistant|analytics|security|about}; `display_setting_changed` {setting: clock|density|room_order|text_size|bubble_align|language|do_not_disturb|wallpaper|seen_style|grayscale|high_contrast, value}; `theme_changed` {light|dark|system}; `personalize_dialog_closed` {done|more_options}; `help_opened`; `help_menu_opened`; `shortcuts_opened` {help_menu|quick_search}; `support_opened`/`support_open_failed`; `common_settings_opened`; `app_frame_used` {discussions|contacts|activity|account}
- Accounts: `account_added`, `account_switched`, `account_signed_out`
- Sharing/copy: `user_id_copied`; `invite_link_copied`; `invite_link_shared`; `page_link_copied`; `room_link_copied`; `contact_field_copied` {username|phone|email}; `contact_shared` {share|copy}; `message_link_copied`/`message_link_copy_failed`; `message_copied`; `phone_number_used` {call|copy}
- Integrations: `drive_picker_opened`; `drive_files_picked` {count, as attachment|link}; `scribe_used` {prompt: correct-grammar, make-shorter, expand-context, emojify, transform-to-bullets, change-tone-professional|casual|polite, translate-french|english|russian|vietnamese, summarize}; `message_auto_translated` (once/message, max 20/min/tab, clear rooms only); `message_translated`; `profile_email_opened` {hasTwakeMail, isInWindow}; `profile_email_closed` {sent|draft|cancelled}
- Search / navigation: `search_result_opened`; `search_match_moved` {older|newer}; `quick_switcher_opened` (Ctrl/Cmd+K); `quick_switcher_create_chosen`; `quick_switcher_room_opened` {isFromDirectory, isMessage}; `room_list_message_opened`; `room_mention_inserted` (# room); `replacement_room_opened`; `inbox_room_opened`; `saved_message_opened`; `pinned_message_opened`; `quoted_message_opened`; `room_date_jumped`; `room_start_jumped`
- Activity: `activity_missed_call_opened`; `activity_missed_call_back`; `activity_reaction_opened`; `activity_filter_changed`
- Misc: `text_formatted` {bold|italic|underline|strike|link|ordered_list|bullet_list|quote|code|code_block}; `room_list_bulk_action` {mark_read|mute|favourite|leave, count}; `message_bulk_action` {copy|forward|download|delete, count}; `profile_opened` {author|mention|warning|account_menu|header_menu|member}; `attachment_viewed`; `attachment_stepped` {previous|next}; `attachment_discarded`; `assistant_privacy_opened`; `analytics_consent_given`

**Messages**
- Send: `message_sent` {isReply, mentionCount, isBotCommand} -> `message_send_succeeded|failed`; `message_retried`; `message_discarded`/`message_discard_failed`; `message_edited` -> `_edit_succeeded|failed`; `message_deleted` {own} -> `_delete_succeeded|failed`
- Moderation/personal: `person_ignored` {isIgnored}; `message_reported` {hasReason}; `message_forwarded` {toRoomId}; `message_asked_assistant` {task email|task|meeting}; `message_pinned` {isPinned, count}; `message_saved` {isSaved}; `message_history_opened` {versionCount}; `message_readers_opened` {readerCount}
- `reaction_toggled` {isAdded} (+succeeded/failed); `older_messages_requested|loaded|failed`
- Attachments: `attachment_sent` {kind image|video|audio|file, hasCaption} -> `attachment_send_succeeded|failed` (errorCode empty_file, too_large); `attachment_opened`/`_open_failed`
- Voice/audio: `voice_recording_started`; `voice_recording_cancelled`; `voice_message_sent` {durationMs, isAtLimit (30-min limit)}; `voice_message_played`; `audio_played`
- Polls: `poll_created` {answerCount}; `poll_voted`; `poll_ended`; `poll_edited` (each with `_failed`)
- GIFs: `gif_picker_opened`; `gif_sent`; `gif_download_failed` {expired|download_failed}
- Threads: `thread_opened`; `thread_reply_sent` -> `thread_reply_succeeded|failed`; `thread_resolved_changed` {isResolved}; `thread_follow_changed` {follow|unfollow|mute}
- Room search: `room_search_submitted` {hasAuthorFilter, hasDateFilter, hasFileFilter} -> `room_search_completed` {resultCount, isComplete}|`_failed`; `room_search_older_requested`; `room_search_older_ended` {pages, resultCount, stop found|start|cap|stopped|failed}; `room_search_advanced_toggled`

**Read state / notifications**
- `room_marked_read` -> `_succeeded {receiptCount}`|`_failed`; `room_marked_unread`; `room_tag_changed` {favourite|low_priority|none}; `room_folder_changed` {isIn}; `folders_saved` {count}; `space_marked_read` {roomCount}; `room_notifications_changed` {all|mentions|none}; `chat_deleted`/`chat_delete_undone` (D186); `notification_settings_changed` {directMessages, groups, mentionsOfMe, invitations, memberEvents, notices}; `private_receipts_changed` {isPrivate}; `autoplay_changed`; `auto_translate_changed` {prompt|null}; `notification_device_changed` {browser, sound, showText, permission default|granted|denied|unsupported}

**Rooms and people**
- Status/profile: `status_set` {isCleared, preset}; `status_preset_set`; `own_profile_saved` {hasName, picture kept|changed|removed}; `contacts_visibility_changed` {private|contacts|public}
- Create/join: `room_created` {kind discussion|broadcast|space|voice|page, isInSpace, isPublic, isEncrypted, access open|invite|approval}; `direct_room_created`; `invite_answered` {accept}; `room_joined`; `room_left`; `room_forgotten`; `member_invited`; `member_removed`; `member_banned`/`member_unbanned`; `member_read_only_changed`; `member_role_changed` {member|moderator|admin}; `ownership_transferred`; `room_settings_saved` {name|topic|avatar|access}; `room_reported`; `room_encryption_enabled`
- Channel switches: `channel_suggestions_enabled|disabled`; `channel_gifs_enabled|disabled`; `channel_scribe_enabled|disabled`; `channel_public_page_enabled|disabled`
- Broadcasts: `page_followed` {open|approval}; `follow_request_answered`; `page_style_saved` {cover|accent_color}
- Directory/contacts: `public_rooms_searched`; `public_rooms_browsed`; `picked_person_removed`; `contacts_searched`; `invitation_sent` {email|msisdn|unknown} -> `invitation_succeeded|failed`; `contact_added|updated|removed`
- Assistant: `assistant_opened`; `assistant_activated`; `assistant_invited`; `assistant_requested`; `assistant_request_answered` {accepted}; `assistant_removed`; `assistant_suggestions_changed`; `assistant_recovery_asked`; question card: `assistant_question_shown|answered {yes|no}|opened|dismissed|text_toggled|popups_changed|dismiss_failed|popups_failed`

**Calls**
- `call_started` {isVideo}; `call_joined`; `call_failed` {no_focus|no_permission|room_not_found|token_failed|media_failed|connection_failed}; `call_left`; `call_declined` (page only, not system notification button); `call_media_toggled` {microphone|camera|screen, isOn}; `call_device_switched` {audioinput|audiooutput|videoinput} (device never sent); `call_recording_toggled`; `call_subtitles_started`; `call_transcription_toggled`; `call_hand_toggled`; `call_meet_action_failed` {action, errorCode meet_failed|meet_forbidden}

**Security / encryption**
- `device_verification_submitted|succeeded|failed` {method recovery_key}; `history_restore_requested|succeeded|failed`; `recovery_setup_requested|succeeded|failed`; `verification_requested` {this_device|other_device|person}, `verification_accepted`, `verification_compared` {isSame}, `verification_emojis_chosen`, `verification_scan_confirmed` {isOk}, `verification_qr_scanned`, `verification_cancelled`, `verification_failed`; `identity_change_approved`; `identity_reset_requested|succeeded {keySaved}|failed`; `device_removed` {count}; `device_renamed`; `device_block_changed` {isBlocked}; `new_device_answered` {wasMe}; `recovery_key_copied`; `room_session_discarded` (/discardsession)

Note: docs/analytics.md lists `display_setting_changed` twice (identical); catalogue has well over 300 event names including `_failed` siblings (not counted exactly).
