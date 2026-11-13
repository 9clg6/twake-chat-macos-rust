//! One row of the room list, read from the SDK room (A031).

use matrix_sdk::deserialized_responses::{SyncOrStrippedState, TimelineEvent};
use matrix_sdk::notification_settings::NotificationSettings;
use matrix_sdk::ruma::MilliSecondsSinceUnixEpoch;
use matrix_sdk::ruma::OwnedUserId;
use matrix_sdk::ruma::events::room::create::RoomCreateEventContent;
use matrix_sdk::ruma::events::room::message::{MessageType, Relation};
use matrix_sdk::ruma::events::space::parent::SpaceParentEventContent;
use matrix_sdk::ruma::events::{
    AnySyncMessageLikeEvent, AnySyncTimelineEvent, SyncMessageLikeEvent, SyncStateEvent,
};
use matrix_sdk::{Client, Room};
use twake_domain::room_list::RoomSummary;
use twake_domain::{MxcUri, RoomId, Timestamp, UserId};

use crate::map;

/// What a summary needs besides the room: who the user is, and their notification settings.
pub(crate) struct Ctx {
    me: Option<UserId>,
    settings: NotificationSettings,
}

impl Ctx {
    pub(crate) async fn new(client: &Client) -> Self {
        Self {
            me: client
                .user_id()
                .and_then(|u| UserId::parse(u.as_str()).ok()),
            settings: client.notification_settings().await,
        }
    }

    async fn is_muted(&self, room: &Room) -> bool {
        let mode = self
            .settings
            .get_user_defined_room_notification_mode(room.room_id())
            .await;
        mode.is_some_and(map::is_muted)
    }
}

/// The row of `room`; none for a room whose id the domain does not accept.
pub(crate) async fn summarise(ctx: &Ctx, room: &Room) -> Option<RoomSummary> {
    let id = RoomId::parse(room.room_id().as_str()).ok()?;
    let is_direct = room.is_direct().await.unwrap_or(false);
    let name = display_name(room).await;
    let mut row = RoomSummary::new(id, name, map::room_kind(room.is_space(), is_direct));
    row.avatar = room
        .avatar_url()
        .and_then(|u| MxcUri::parse(u.as_str()).ok());
    row.unread = unread(room);
    row.is_favourite = room.is_favourite();
    row.is_low_priority = room.is_low_priority();
    row.is_muted = ctx.is_muted(room).await;
    row.space = parent_space(room).await;
    last_message(ctx, room, &mut row).await;
    Some(row)
}

/// The name the SDK computes (heroes for a room with no name), else the cached one.
async fn display_name(room: &Room) -> String {
    match room.display_name().await {
        Ok(name) => name.to_string(),
        Err(_) => room
            .cached_display_name()
            .map(|n| n.to_string())
            .unwrap_or_default(),
    }
}

fn unread(room: &Room) -> twake_domain::read_state::UnreadCount {
    let counts = room.unread_notification_counts();
    map::unread_count(
        counts.notification_count,
        counts.highlight_count,
        room.is_marked_unread(),
    )
}

/// The space the room says it sits in, the canonical one first.
/// ponytail: the claim is not checked against an `m.space.child` of the parent (the spec asks
/// clients to), so anyone who can set the state of a room can place it under a space.
async fn parent_space(room: &Room) -> Option<RoomId> {
    let events = room
        .get_state_events_static::<SpaceParentEventContent>()
        .await
        .ok()?;
    let mut parents: Vec<(bool, String)> = events
        .iter()
        .filter_map(|raw| match raw.deserialize().ok()? {
            SyncOrStrippedState::Sync(SyncStateEvent::Original(ev)) => {
                Some((ev.content.canonical, ev.state_key.to_string()))
            }
            _ => None,
        })
        .collect();
    parents.sort_by_key(|(canonical, _)| !canonical);
    parents.iter().find_map(|(_, id)| RoomId::parse(id).ok())
}

/// A message of the room, as the row needs it.
struct Last {
    sender: OwnedUserId,
    at: MilliSecondsSinceUnixEpoch,
    content: MessageType,
}

/// Sets the preview, the author and the date: the newest message of the event cache, else the
/// creation of the room (D101).
async fn last_message(ctx: &Ctx, room: &Room, row: &mut RoomSummary) {
    let Some(last) = newest_message(room).await else {
        row.last_activity = creation(room).await;
        return;
    };
    let display = member_name(room, &last.sender).await;
    let sender = UserId::parse(last.sender.as_str()).ok();
    let name = sender
        .as_ref()
        .map(|s| map::name_of(s, display.as_deref()))
        .unwrap_or_default();
    row.last_activity = map::timestamp(last.at);
    row.last_message_preview = Some(map::preview(&last.content, &name));
    row.last_sender = match (&ctx.me, sender) {
        (Some(me), Some(sender)) => Some(map::last_sender(me, &sender, display.as_deref())),
        _ => None,
    };
}

async fn member_name(room: &Room, user: &OwnedUserId) -> Option<String> {
    let member = room.get_member_no_sync(user).await.ok()??;
    member.display_name().map(str::to_string)
}

/// Whether the event cache of the room holds a message already.
pub(crate) async fn has_message(room: &Room) -> bool {
    newest_message(room).await.is_some()
}

async fn newest_message(room: &Room) -> Option<Last> {
    let (cache, _keep) = room.event_cache().await.ok()?;
    cache.rfind_map_event_in_memory_by(as_last).await.ok()?
}

/// A shown message (not an edit, not a deleted one).
fn as_last(event: &TimelineEvent) -> Option<Last> {
    let AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(
        SyncMessageLikeEvent::Original(message),
    )) = event.raw().deserialize().ok()?
    else {
        return None;
    };
    if matches!(message.content.relates_to, Some(Relation::Replacement(_))) {
        return None;
    }
    Some(Last {
        sender: message.sender,
        at: message.origin_server_ts,
        content: message.content.msgtype,
    })
}

async fn creation(room: &Room) -> Timestamp {
    let create = room
        .get_state_event_static::<RoomCreateEventContent>()
        .await
        .ok()
        .flatten()
        .and_then(|raw| raw.deserialize().ok());
    match create {
        Some(SyncOrStrippedState::Sync(SyncStateEvent::Original(ev))) => {
            map::timestamp(ev.origin_server_ts)
        }
        _ => Timestamp::from_millis(0),
    }
}
