//! A key type for role data.

use serde::Serialize;

/// A key type for role data.
#[derive(Debug, Clone, Serialize)]

pub(crate) enum KeyType {
    /// A public key.
    Pubkey,
    /// A X509 certificate.
    X509,
    /// A C509 certificate.
    C509,
}
