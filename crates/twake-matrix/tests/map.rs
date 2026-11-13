//! The pure mapping from matrix-sdk / ruma values to the domain (room list and conversation).
//! No server, no client: plain values in, plain values out.
//!
//! Expected public surface, `twake_matrix::map`:
//! - `room_kind(is_space: bool, is_direct: bool) -> RoomKind` (Space, else Direct, else Group)
//! - `is_muted(RoomNotificationMode) -> bool`
//! - `unread_count(messages: u64, mentions: u64, marked_unread: bool) -> UnreadCount`
//! - `last_sender(me: &UserId, sender: &UserId, display_name: Option<&str>) -> LastSender`
//! - `timestamp(MilliSecondsSinceUnixEpoch) -> Timestamp`
//! - `send_state(&EventSendState) -> SendState`
//! - `preview(&MessageType, sender_name: &str) -> String` (the row of the room list, D31, D175)
//! - `message_body(&MessageType, sender_name: &str) -> String` (the bubble)
//! - `room_list_error(&matrix_sdk::Error) -> RoomListError`,
//!   `timeline_error(&matrix_sdk::Error) -> TimelineError`
#![recursion_limit = "256"]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::sync::Arc;

use matrix_sdk::notification_settings::RoomNotificationMode;
use matrix_sdk::ruma::events::room::message::{
    AudioMessageEventContent, EmoteMessageEventContent, FileMessageEventContent,
    ImageMessageEventContent, MessageType, NoticeMessageEventContent, VideoMessageEventContent,
};
use matrix_sdk::ruma::{MilliSecondsSinceUnixEpoch, UInt, event_id, mxc_uri};
use matrix_sdk_ui::timeline::EventSendState;
use twake_app::ports::{RoomListError, TimelineError};
use twake_domain::read_state::UnreadCount;
use twake_domain::room_list::{LastSender, RoomKind};
use twake_domain::timeline::SendState;
use twake_domain::{Timestamp, UserId};
use twake_matrix::map;

fn user(id: &str) -> UserId {
    UserId::parse(id).unwrap()
}

// ------------------------------------------------------------ rooms

#[test]
fn a_space_is_a_space_even_when_it_is_flagged_direct() {
    assert_eq!(map::room_kind(true, true), RoomKind::Space);
    assert_eq!(map::room_kind(true, false), RoomKind::Space);
}

#[test]
fn a_direct_room_is_direct_and_the_others_are_groups() {
    assert_eq!(map::room_kind(false, true), RoomKind::Direct);
    assert_eq!(map::room_kind(false, false), RoomKind::Group);
}

#[test]
fn a_room_that_says_nothing_or_only_mentions_is_muted() {
    // D31: "Muted room (level mentions or none)".
    assert!(!map::is_muted(RoomNotificationMode::AllMessages));
    assert!(map::is_muted(RoomNotificationMode::MentionsAndKeywordsOnly));
    assert!(map::is_muted(RoomNotificationMode::Mute));
}

#[test]
fn unread_counts_are_kept_as_given() {
    let want = UnreadCount {
        unread: 3,
        mentions: 1,
        marked_unread: false,
    };
    assert_eq!(map::unread_count(3, 1, false), want);
}

#[test]
fn the_mark_without_a_count_is_kept() {
    // D615: a room marked unread shows without a count.
    let got = map::unread_count(0, 0, true);
    assert!(got.marked_unread && got.is_unread());
    assert!(!map::unread_count(0, 0, false).is_unread());
}

#[test]
fn a_count_beyond_what_the_domain_holds_stops_at_the_largest_one() {
    let got = map::unread_count(u64::MAX, u64::from(u32::MAX) + 1, false);
    assert_eq!((got.unread, got.mentions), (u32::MAX, u32::MAX));
}

#[test]
fn the_last_message_of_the_user_is_marked_me() {
    let me = user("@alice:example.org");
    assert_eq!(map::last_sender(&me, &me, Some("Alice")), LastSender::Me);
}

#[test]
fn the_last_message_of_another_person_carries_their_display_name() {
    let (me, bob) = (user("@alice:example.org"), user("@bob:example.org"));
    assert_eq!(
        map::last_sender(&me, &bob, Some("Bob Martin")),
        LastSender::Other("Bob Martin".into())
    );
}

#[test]
fn a_person_without_a_display_name_is_known_by_the_local_part_of_their_id() {
    let (me, bob) = (user("@alice:example.org"), user("@bob:example.org"));
    for name in [None, Some(""), Some("   ")] {
        assert_eq!(
            map::last_sender(&me, &bob, name),
            LastSender::Other("bob".into()),
            "{name:?}"
        );
    }
}

#[test]
fn a_display_name_is_shown_as_is_in_the_row_never_escaped() {
    // The row is plain text: the Markdown escape belongs to text that is rendered as Markdown.
    let (me, bob) = (user("@alice:example.org"), user("@bob:example.org"));
    assert_eq!(
        map::last_sender(&me, &bob, Some("*Bob* <b>")),
        LastSender::Other("*Bob* <b>".into())
    );
}

#[test]
fn a_matrix_time_is_the_same_instant_in_the_domain() {
    let at = MilliSecondsSinceUnixEpoch(UInt::new(1_700_000_123_456).unwrap());
    assert_eq!(
        map::timestamp(at),
        Timestamp::from_millis(1_700_000_123_456)
    );
}

// ------------------------------------------------------------ send state

#[test]
fn a_message_on_its_way_is_sending() {
    let state = EventSendState::NotSentYet { progress: None };
    assert_eq!(map::send_state(&state), SendState::Sending);
}

#[test]
fn a_message_the_server_accepted_is_sent() {
    let state = EventSendState::Sent {
        event_id: event_id!("$sent").to_owned(),
    };
    assert_eq!(map::send_state(&state), SendState::Sent);
}

#[test]
fn a_message_that_could_not_be_sent_is_failed_whether_or_not_it_can_be_tried_again() {
    for is_recoverable in [true, false] {
        let error = matrix_sdk::Error::UnknownError(Box::new(std::io::Error::other("offline")));
        let state = EventSendState::SendingFailed {
            error: Arc::new(error),
            is_recoverable,
        };
        assert_eq!(
            map::send_state(&state),
            SendState::Failed,
            "{is_recoverable}"
        );
    }
}

// ------------------------------------------------------------ text of a row and of a bubble

fn caption_image(body: &str, filename: Option<&str>) -> MessageType {
    let mut image =
        ImageMessageEventContent::plain(body.into(), mxc_uri!("mxc://example.org/i").to_owned());
    image.filename = filename.map(str::to_string);
    MessageType::Image(image)
}

#[test]
fn a_text_is_its_own_preview_and_its_own_body() {
    let text = MessageType::text_plain("see you at 5");
    assert_eq!(map::preview(&text, "Bob"), "see you at 5");
    assert_eq!(map::message_body(&text, "Bob"), "see you at 5");
}

#[test]
fn a_notice_reads_as_a_text() {
    let notice = MessageType::Notice(NoticeMessageEventContent::plain("build passed"));
    assert_eq!(map::preview(&notice, "Bot"), "build passed");
    assert_eq!(map::message_body(&notice, "Bot"), "build passed");
}

#[test]
fn a_code_block_previews_its_first_line_of_code_not_the_fence() {
    // D175.
    let code = MessageType::text_plain("```rust\nlet x = 1;\nlet y = 2;\n```");
    assert_eq!(map::preview(&code, "Bob"), "let x = 1;");
}

#[test]
fn an_emote_reads_the_name_then_the_words_in_the_row() {
    // D175: "alice waves" in the row, with no author before it.
    let emote = MessageType::Emote(EmoteMessageEventContent::plain("waves"));
    assert_eq!(map::preview(&emote, "alice"), "alice waves");
}

#[test]
fn an_emote_reads_a_star_the_name_then_the_words_in_the_bubble() {
    let emote = MessageType::Emote(EmoteMessageEventContent::plain("waves"));
    assert_eq!(map::message_body(&emote, "alice"), "* alice waves");
}

#[test]
fn a_name_that_looks_like_markdown_is_escaped_in_the_bubble_with_the_existing_escape() {
    // The bubble text is rendered as Markdown: `*alice*` must stay a name, not become emphasis.
    let emote = MessageType::Emote(EmoteMessageEventContent::plain("waves"));
    assert_eq!(map::message_body(&emote, "*alice*"), "* \\*alice\\* waves");
}

#[test]
fn a_file_without_a_caption_previews_its_kind() {
    // D31: "Image" / "Video" / "Audio" / "File"; the body of such an event is the file name.
    let url = || mxc_uri!("mxc://example.org/f").to_owned();
    let cases = [
        (caption_image("photo.png", None), "Image"),
        (caption_image("photo.png", Some("photo.png")), "Image"),
        (
            MessageType::Video(VideoMessageEventContent::plain("clip.mp4".into(), url())),
            "Video",
        ),
        (
            MessageType::Audio(AudioMessageEventContent::plain("note.ogg".into(), url())),
            "Audio",
        ),
        (
            MessageType::File(FileMessageEventContent::plain("report.pdf".into(), url())),
            "File",
        ),
    ];
    for (content, kind) in cases {
        assert_eq!(map::preview(&content, "Bob"), kind);
    }
}

#[test]
fn a_file_with_a_caption_previews_the_caption() {
    let content = caption_image("Look at this", Some("photo.png"));
    assert_eq!(map::preview(&content, "Bob"), "Look at this");
    assert_eq!(map::message_body(&content, "Bob"), "Look at this");
}

#[test]
fn a_bubble_never_comes_out_empty_for_a_file_without_a_caption() {
    // Media get their own rendering in a later lot; until then the bubble says what it is.
    let content = caption_image("photo.png", None);
    assert_eq!(map::message_body(&content, "Bob"), "Image");
}

// ------------------------------------------------------------ errors

const TOKEN: &str = "syt_secret_access_token";

fn failure_mentioning_the_token() -> matrix_sdk::Error {
    matrix_sdk::Error::UnknownError(Box::new(std::io::Error::other(format!(
        "GET /sync?access_token={TOKEN} failed"
    ))))
}

#[test]
fn a_failure_of_the_room_list_is_unavailable_and_says_nothing_of_the_token() {
    let mapped = map::room_list_error(&failure_mentioning_the_token());
    assert_eq!(mapped, RoomListError::Unavailable);
    assert!(!format!("{mapped} {mapped:?}").contains(TOKEN));
}

#[test]
fn a_failure_of_the_conversation_is_unavailable_and_says_nothing_of_the_token() {
    let mapped = map::timeline_error(&failure_mentioning_the_token());
    assert_eq!(mapped, TimelineError::Unavailable);
    assert!(!format!("{mapped} {mapped:?}").contains(TOKEN));
}

#[test]
fn a_client_with_no_session_is_not_signed_in() {
    let mapped = map::room_list_error(&matrix_sdk::Error::AuthenticationRequired);
    assert_eq!(mapped, RoomListError::NotSignedIn);
}
