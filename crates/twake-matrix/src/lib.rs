//! Adapter over matrix-sdk: sync, room list, timeline, crypto, store, SSO login.
#![recursion_limit = "256"] // matrix-sdk futures overflow the default layout depth of 128

mod client;
mod login;
mod restore;
mod sign_out;

pub use client::{ClientError, account_data_dir, build_client, install_tls_provider};
pub use login::{LoginError, login_with_token};
pub use restore::{RestoreError, restore_session};
pub use sign_out::{Expiry, SignOutError, expiry, sign_out};
