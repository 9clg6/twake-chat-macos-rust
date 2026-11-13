//! Secret store over the OS keychain, preferences file, store passphrase.

mod keychain;
mod os_keychain;
mod session_store;
mod store_key;

pub use keychain::{Keychain, KeychainError};
pub use os_keychain::{KEYCHAIN_SERVICE, OsKeychain, platform_store};
pub use session_store::KeychainSessionStore;
pub use store_key::{forget_store_key, generate_store_key, generate_store_key_with, store_key_for};
