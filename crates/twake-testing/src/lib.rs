//! Fakes of every port, fixtures, builders and contract suites; dev-dependency only.

pub mod contract;
mod draft_store_contract;
mod fake_draft_store;
mod fake_room_list;
mod fake_session_store;
mod fake_timeline;
mod message_actions_contract;
mod session_store_contract;
mod subscribers;

pub use fake_draft_store::FakeDraftStore;
pub use fake_room_list::FakeRoomList;
pub use fake_session_store::FakeSessionStore;
pub use fake_timeline::FakeTimeline;
