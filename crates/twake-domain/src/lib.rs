//! Pure domain types and rules: ids, room summaries, message view models, read state, power levels, errors.

mod ids;
pub mod read_state;

pub use ids::{EventId, IdError, RoomAlias, RoomId, UserId};
