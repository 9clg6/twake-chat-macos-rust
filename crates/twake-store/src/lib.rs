//! Secret store over the OS keychain, preferences file, store passphrase.

mod keychain;
mod session_store;

pub use keychain::{Keychain, KeychainError};
pub use session_store::KeychainSessionStore;
