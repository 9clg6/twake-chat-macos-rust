//! Adapter over matrix-sdk: sync, room list, timeline, crypto, store, SSO login.
#![recursion_limit = "256"] // matrix-sdk futures overflow the default layout depth of 128

mod client;
mod login;
pub mod map;
mod message;
mod restore;
mod room_list;
mod sign_out;
mod summary;
mod sync;
mod timeline;
mod view;

pub use client::{ClientError, account_data_dir, build_client, install_tls_provider};
pub use login::{LoginError, login_with_token};
pub use restore::{RestoreError, restore_session};
pub use room_list::MatrixRoomList;
pub use sign_out::{Expiry, SignOutError, expiry, sign_out};
pub use timeline::MatrixTimeline;
