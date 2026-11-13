//! The key of the SQLite store of a Matrix account (A027 decision 1, A030): 32 bytes of the OS
//! CSPRNG as 64 lowercase hexadecimals, one per account, kept in the keychain.

use std::fmt::Write;

use twake_domain::UserId;
use twake_domain::session::Secret;

use crate::{Keychain, KeychainError};

const KEY_BYTES: usize = 32;

/// The key from the bytes `fill` writes (the CSPRNG, injected).
pub fn generate_store_key_with(fill: impl FnOnce(&mut [u8; KEY_BYTES])) -> Secret {
    let mut bytes = [0_u8; KEY_BYTES];
    fill(&mut bytes);
    let mut hex = String::with_capacity(KEY_BYTES * 2);
    for byte in bytes {
        let _ = write!(hex, "{byte:02x}");
    }
    Secret::new(hex)
}

/// A key from the CSPRNG of the OS. Panics if the OS has none: a weak key must never be made.
#[allow(clippy::expect_used)]
pub fn generate_store_key() -> Secret {
    generate_store_key_with(|bytes| getrandom::fill(bytes).expect("the OS random generator"))
}

fn entry_name(user: &UserId) -> String {
    format!("store-key:{user}")
}

fn is_valid(key: &str) -> bool {
    key.len() == KEY_BYTES * 2 && key.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The key of `user`; made with `generate` and kept when there is none. A kept key that is not
/// 64 hexadecimals is an error, never replaced: a new key would orphan the encrypted store.
pub fn store_key_for(
    keychain: &dyn Keychain,
    user: &UserId,
    generate: impl FnOnce() -> Secret,
) -> Result<Secret, KeychainError> {
    let name = entry_name(user);
    match keychain.get(&name)? {
        Some(key) if is_valid(key.expose()) => Ok(key),
        Some(_) => Err(KeychainError::default()),
        None => {
            let key = generate();
            keychain.set(&name, &key)?;
            Ok(key)
        }
    }
}

/// Removes the key of `user` (sign-out); a missing key is fine.
pub fn forget_store_key(keychain: &dyn Keychain, user: &UserId) -> Result<(), KeychainError> {
    keychain.delete(&entry_name(user))
}
