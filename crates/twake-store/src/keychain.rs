//! The OS keychain behind a trait (P2: the OS crate lives only in this crate).

use twake_domain::session::Secret;

/// The keychain cannot be reached or refused. Carries no text: nothing secret can leak from it.
#[derive(Debug, Default, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the keychain is unavailable")]
pub struct KeychainError {
    _private: (),
}

/// Named secrets in the keychain. Calls are synchronous, as the OS calls are.
pub trait Keychain: Send + Sync {
    /// The secret of `entry`, or `None` when there is none.
    fn get(&self, entry: &str) -> Result<Option<Secret>, KeychainError>;

    /// Creates or replaces `entry`.
    fn set(&self, entry: &str, value: &Secret) -> Result<(), KeychainError>;

    /// Removes `entry`; a missing entry is fine.
    fn delete(&self, entry: &str) -> Result<(), KeychainError>;
}
