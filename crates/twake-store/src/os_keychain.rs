//! `OsKeychain`: the `Keychain` trait over `keyring-core` (spike S3, A030).

use std::sync::Arc;

use keyring_core::{CredentialStore, Entry, Error as CoreError};
use twake_domain::session::Secret;

use crate::{Keychain, KeychainError};

/// The service name every entry of the app is filed under.
pub const KEYCHAIN_SERVICE: &str = "twake-chat";

/// The keychain of the OS (or any `keyring-core` store handed to it).
pub struct OsKeychain {
    store: Arc<CredentialStore>,
    service: String,
}

impl OsKeychain {
    /// Entries filed under [`KEYCHAIN_SERVICE`] in `store`.
    pub fn new(store: Arc<CredentialStore>) -> Self {
        Self::with_service(store, KEYCHAIN_SERVICE)
    }

    /// Entries filed under `service` (tests, parallel installs).
    pub fn with_service(store: Arc<CredentialStore>, service: impl Into<String>) -> Self {
        Self {
            store,
            service: service.into(),
        }
    }

    fn entry(&self, name: &str) -> Result<Entry, KeychainError> {
        self.store
            .build(&self.service, name, None)
            .map_err(|_| KeychainError::default())
    }
}

impl Keychain for OsKeychain {
    fn get(&self, entry: &str) -> Result<Option<Secret>, KeychainError> {
        match self.entry(entry)?.get_password() {
            Ok(text) => Ok(Some(Secret::new(text))),
            Err(CoreError::NoEntry) => Ok(None),
            Err(_) => Err(KeychainError::default()),
        }
    }

    // A value over the limit of the OS (2560 bytes on Windows) comes back as an error from the
    // store: never cut here.
    fn set(&self, entry: &str, value: &Secret) -> Result<(), KeychainError> {
        self.entry(entry)?
            .set_password(value.expose())
            .map_err(|_| KeychainError::default())
    }

    fn delete(&self, entry: &str) -> Result<(), KeychainError> {
        match self.entry(entry)?.delete_credential() {
            Ok(()) | Err(CoreError::NoEntry) => Ok(()),
            Err(_) => Err(KeychainError::default()),
        }
    }
}

impl std::fmt::Debug for OsKeychain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OsKeychain")
            .field("service", &self.service)
            .finish_non_exhaustive()
    }
}

/// The store of the OS the build targets: macOS Keychain, Windows Credential Manager, Secret
/// Service on Linux. Any other target has none.
pub fn platform_store() -> Result<Arc<CredentialStore>, KeychainError> {
    #[cfg(target_os = "macos")]
    let store =
        apple_native_keyring_store::keychain::Store::new().map(|s| s as Arc<CredentialStore>);
    #[cfg(target_os = "windows")]
    let store = windows_native_keyring_store::Store::new().map(|s| s as Arc<CredentialStore>);
    #[cfg(target_os = "linux")]
    let store = zbus_secret_service_keyring_store::Store::new().map(|s| s as Arc<CredentialStore>);
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    let store: Result<Arc<CredentialStore>, CoreError> = Err(CoreError::NotSupportedByStore(
        "no keychain on this target".into(),
    ));
    store.map_err(|_| KeychainError::default())
}
