//! The client factory: one `Client` per account, on an encrypted SQLite store (A019, A025).

use std::path::{Path, PathBuf};

use matrix_sdk::{Client, SqliteStoreConfig};
use twake_domain::UserId;
use twake_domain::session::{HomeserverUrl, Secret};

/// Why a client could not be built. Never carries the key.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The key is not 64 hexadecimal characters (32 bytes).
    #[error("the store key is not 32 bytes in hexadecimal")]
    InvalidStoreKey,
    /// The store cannot be created or opened (wrong key, unreadable folder, bad homeserver).
    #[error("the account store is unavailable")]
    StoreUnavailable,
}

/// Installs the aws-lc-rs TLS provider once: `ring` (gpui) is in the binary too (A019).
/// Calling it again is harmless.
pub fn install_tls_provider() {
    // Err means a provider is already installed, which is what we want.
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
}

/// The folder of one account under `root`: its id with every character but `[A-Za-z0-9._-]`
/// written `%XX`, so it is one plain segment, whatever the id (it starts with `%40`).
pub fn account_data_dir(root: &Path, user: &UserId) -> PathBuf {
    let name: String = user
        .as_str()
        .bytes()
        .map(|b| match b {
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'.' | b'_' | b'-' => {
                char::from(b).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    root.join(name)
}

/// A client of `homeserver` with its store in `dir`, signed out. No network call.
pub async fn build_client(
    homeserver: &HomeserverUrl,
    dir: &Path,
    key: &Secret,
) -> Result<Client, ClientError> {
    let key = decode_key(key.expose()).ok_or(ClientError::InvalidStoreKey)?;
    std::fs::create_dir_all(dir).map_err(|_| ClientError::StoreUnavailable)?;
    Client::builder()
        .homeserver_url(homeserver.as_str())
        .sqlite_store_with_config_and_cache_path(
            SqliteStoreConfig::new(dir).key(Some(&key)),
            None::<&Path>,
        )
        .build()
        .await
        .map_err(|_| ClientError::StoreUnavailable)
}

fn decode_key(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 || !hex.is_ascii() {
        return None;
    }
    let mut key = [0u8; 32];
    for (byte, pair) in key.iter_mut().zip(hex.as_bytes().chunks(2)) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
    }
    Some(key)
}
