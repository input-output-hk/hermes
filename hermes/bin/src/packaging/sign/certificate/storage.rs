//! A concurrent certificates storage.

use dashmap::DashMap;
use once_cell::sync::Lazy;

use crate::packaging::{hash::Blake2b256, sign::certificate::Certificate};

/// Singleton `CertificateStorage` instance.
static STORAGE: Lazy<CertificateStorage> = Lazy::new(CertificateStorage::new);

/// Add new `Certificate` to the storage.
#[allow(dead_code)]
pub(crate) fn add_certificate(certificate: Certificate) -> anyhow::Result<()> {
    STORAGE.insert(certificate)
}

/// Get `Certificate` from the storage.
pub(crate) fn get_certificate(hash: &Blake2b256) -> Option<Certificate> {
    STORAGE.get(hash)
}

/// `Certificate`'s storage, implemented as a key-value store.
/// Key is a `Blake2b256` hash of the certificate DER bytes.
/// Value is a `Certificate` itself.
struct CertificateStorage(DashMap<Blake2b256, Certificate>);

impl CertificateStorage {
    /// Create new `CertificateStorage` instance.
    fn new() -> Self {
        Self(DashMap::new())
    }

    /// Insert new `Certificate` into the storage.
    fn insert(&self, certificate: Certificate) -> anyhow::Result<()> {
        self.0.insert(certificate.hash()?, certificate);
        Ok(())
    }

    /// Get `Certificate` from the storage.
    fn get(&self, hash: &Blake2b256) -> Option<Certificate> {
        self.0.get(hash).map(|val| val.value().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packaging::sign::certificate::tests::certificate_str;

    #[test]
    fn storage_test() {
        let cert = Certificate::from_str(&certificate_str()).expect("Cannot create cert");
        let cert_hash = cert.hash().expect("Failed to get certificate hash.");

        assert!(get_certificate(&cert_hash).is_none());

        add_certificate(cert).expect("Failed to add certificate.");

        assert!(get_certificate(&cert_hash).is_some());
    }
}
