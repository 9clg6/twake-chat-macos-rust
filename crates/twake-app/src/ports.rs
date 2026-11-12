//! The ports of the room list and of the conversation (A009: boxed `Send` futures, so the UI
//! holds them as `Arc<dyn Port>`).

use futures::future::BoxFuture;
use futures::stream::BoxStream;
use twake_domain::RoomId;
use twake_domain::room_list::RoomListDiff;
use twake_domain::timeline::TimelineDiff;

/// Messages in a page of history.
pub const TIMELINE_PAGE_SIZE: usize = 30;

/// Why the room list is not available.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoomListError {
    /// The server or the store cannot be reached.
    #[error("the room list is unavailable")]
    Unavailable,
    /// There is no session.
    #[error("not signed in")]
    NotSignedIn,
}

/// Why a conversation is not available.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TimelineError {
    /// The user is not in this room.
    #[error("room not found")]
    RoomNotFound,
    /// The server or the store cannot be reached.
    #[error("the conversation is unavailable")]
    Unavailable,
}

/// The rooms of the account.
pub trait RoomListPort: Send + Sync {
    /// A stream of batches; the first of every subscription is exactly `[Reset(all rooms)]`.
    fn subscribe(
        &self,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Vec<RoomListDiff>>, RoomListError>>;
}

/// The conversation of a room.
pub trait TimelinePort: Send + Sync {
    /// A stream of batches; the first is `[Reset(last TIMELINE_PAGE_SIZE messages, oldest first)]`.
    fn open(
        &self,
        room: RoomId,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Vec<TimelineDiff>>, TimelineError>>;

    /// Returns once asked; the page arrives on the open streams as `[Prepend { .. }]`.
    fn paginate_back(&self, room: RoomId) -> BoxFuture<'_, Result<(), TimelineError>>;

    /// Returns once accepted; the local echo arrives as `[Append([Message::local(..)])]`, the
    /// server event later as an `Append` carrying the same transaction id.
    fn send_text(&self, room: RoomId, body: String) -> BoxFuture<'_, Result<(), TimelineError>>;
}
