//! The ports of the room list and of the conversation (A009: boxed `Send` futures, so the UI
//! holds them as `Arc<dyn Port>`).

use crate::composer::Draft;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use twake_domain::room_list::RoomListDiff;
use twake_domain::session::Session;
use twake_domain::timeline::TimelineDiff;
use twake_domain::{EventId, RoomId, UserId};

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

/// Accounts signed in at once (D373).
pub const MAX_ACCOUNTS: usize = 5;

/// Why the session store failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SessionStoreError {
    /// The keychain cannot be reached (A005: no plaintext fallback).
    #[error("the session store is unavailable")]
    Unavailable,
    /// A sixth distinct account.
    #[error("too many accounts")]
    TooManyAccounts,
}

/// The sessions of the signed-in accounts, in the OS keychain (A005, NFR-015): one per user id.
pub trait SessionStorePort: Send + Sync {
    /// Inserts, or replaces the session of the same user id in place (its place is kept).
    /// A sixth distinct user id is refused and nothing changes.
    fn save(&self, session: Session) -> BoxFuture<'_, Result<(), SessionStoreError>>;

    /// The session of `user`, if any.
    fn load(&self, user: UserId) -> BoxFuture<'_, Result<Option<Session>, SessionStoreError>>;

    /// In the order the accounts were first saved (the first is the default one, D373).
    fn list(&self) -> BoxFuture<'_, Result<Vec<Session>, SessionStoreError>>;

    /// Forgets `user`, freeing a place; an unknown user is fine.
    fn clear(&self, user: UserId) -> BoxFuture<'_, Result<(), SessionStoreError>>;
}

/// Why the drafts cannot be kept or read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DraftStoreError {
    /// The store cannot be reached.
    #[error("the draft store is unavailable")]
    Unavailable,
}

/// The draft of each room, kept across restarts (FR-055).
pub trait DraftStorePort: Send + Sync {
    /// Replaces the draft of `room`. A draft whose text is empty is a `clear`.
    fn save(&self, room: RoomId, draft: Draft) -> BoxFuture<'_, Result<(), DraftStoreError>>;

    /// The draft of `room`, if any.
    fn load(&self, room: RoomId) -> BoxFuture<'_, Result<Option<Draft>, DraftStoreError>>;

    /// Forgets the draft of `room`; an unknown room is fine.
    fn clear(&self, room: RoomId) -> BoxFuture<'_, Result<(), DraftStoreError>>;
}

/// Why an action on a message failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ActionError {
    /// The user is not in this room.
    #[error("room not found")]
    RoomNotFound,
    /// The room does not let the user do this.
    #[error("not allowed")]
    Forbidden,
    /// The server cannot be reached.
    #[error("the server is unavailable")]
    Unavailable,
}

/// Reactions, redactions, pins and reports of a room.
pub trait MessageActionsPort: Send + Sync {
    /// Sends an `m.reaction` on `target`; returns the id of the reaction event.
    fn send_reaction(
        &self,
        room: RoomId,
        target: EventId,
        key: String,
    ) -> BoxFuture<'_, Result<EventId, ActionError>>;

    /// Redacts an event (a reaction, a message); twice, or an unknown event, is fine.
    fn redact(&self, room: RoomId, event: EventId) -> BoxFuture<'_, Result<(), ActionError>>;

    /// Adds to `m.room.pinned_events`; already pinned is fine; the others stay, in order.
    fn pin(&self, room: RoomId, event: EventId) -> BoxFuture<'_, Result<(), ActionError>>;

    /// Takes out of `m.room.pinned_events`; the others stay.
    fn unpin(&self, room: RoomId, event: EventId) -> BoxFuture<'_, Result<(), ActionError>>;

    /// The pinned events now, in state order.
    fn pinned(&self, room: RoomId) -> BoxFuture<'_, Result<Vec<EventId>, ActionError>>;

    /// Reports a message to the server administrators, with an optional reason (D255).
    fn report(
        &self,
        room: RoomId,
        event: EventId,
        reason: Option<String>,
    ) -> BoxFuture<'_, Result<(), ActionError>>;
}
