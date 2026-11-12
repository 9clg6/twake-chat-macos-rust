//! Pure domain types and rules: ids, room summaries, message view models, read state, power levels, errors.

mod ids;
pub mod read_state;
pub mod room_list;
pub mod session;
mod time;
pub mod timeline;

pub use ids::{EventId, IdError, MxcUri, RoomAlias, RoomId, TxnId, UserId};
pub use session::{DeviceId, HomeserverUrl, Secret, Session};
pub use time::{Day, Timestamp};
