//! A concurrent certificates storage.

use dashmap::DashMap;

use crate::packaging::{hash::Blake2b256, sign::certificate::Certificate};

/// `Certificate`'s storage, implemented as a key-value store.
/// Key is a `Blake2b256` hash of the certificate DER bytes.
/// Value is a `Certificate` itself.
#[allow(dead_code)]
pub(crate) struct CertificateStorage(DashMap<Blake2b256, Certificate>);

#[allow(dead_code)]
impl CertificateStorage {
    /// Create new `CertificateStorage` instance.
    fn new() -> Self {
        Self(DashMap::new())
    }

    /// Insert new `Certificate` into the storage.
    pub(crate) fn insert(&self, certificate: Certificate) -> anyhow::Result<()> {
        self.0.insert(certificate.hash()?, certificate);
        Ok(())
    }

    /// Get `Certificate` from the storage.
    pub(crate) fn get(&self, hash: &Blake2b256) -> Option<Certificate> {
        self.0.get(hash).map(|val| val.value().clone())
    }
}
