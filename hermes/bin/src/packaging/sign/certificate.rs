//! x.509 certificate implementation.

use std::path::Path;

use x509_cert::der::{DecodePem, Encode};

use super::{hash::Blake2b256, keys::PublicKey};
use crate::packaging::FileError;

/// Certificate decoding from string error.
#[derive(thiserror::Error, Debug)]
#[error("Cannot decode certificate from string. Invalid PEM format.")]
pub(crate) struct CertificateDecodingError;

/// x.509 cert instance.
/// Wrapper over `x509_cert::Certificate`
pub(crate) struct Certificate(x509_cert::Certificate);

impl Certificate {
    /// Create new certificate from file decoded in PEM format
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let str = std::fs::read_to_string(&path).map_err(|_| FileError::from_path(&path, None))?;

        Ok(Self::from_str(&str).map_err(|err| FileError::from_path(&path, Some(err)))?)
    }

    /// Create new certificate from string decoded in PEM format
    pub(crate) fn from_str(str: &str) -> anyhow::Result<Self> {
        let cert = x509_cert::Certificate::from_pem(str.as_bytes())
            .map_err(|_| CertificateDecodingError)?;
        Ok(Self(cert))
    }

    /// Get certificate's subject public key.
    pub(crate) fn subject_public_key(&self) -> anyhow::Result<PublicKey> {
        let subject_public_key = &self
            .0
            .tbs_certificate
            .subject_public_key_info
            .subject_public_key;

        PublicKey::from_bytes(subject_public_key.raw_bytes())
    }

    /// `Blake2b256` hash of the certificate DER encoded bytes.
    pub(crate) fn hash(&self) -> anyhow::Result<Blake2b256> {
        let der_bytes = self.0.to_der()?;
        Ok(Blake2b256::hash(&der_bytes))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use temp_dir::TempDir;

    use super::*;
    use crate::packaging::sign::keys::tests::public_key_str;

    pub(crate) fn certificate_str() -> String {
        format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            "-----BEGIN CERTIFICATE-----",
            "MIICCTCCAbugAwIBAgIUfZ0PWPMb4DDteQDZagWn2x+ognEwBQYDK2VwMIGSMQsw",
            "CQYDVQQGEwJBQjELMAkGA1UECAwCQ0QxEDAOBgNVBAcMB0VGR19ISUoxDjAMBgNV",
            "BAoMBU15T3JnMRIwEAYDVQQLDAlNeU9yZ1VuaXQxFzAVBgNVBAMMDm15Y29tbW5h",
            "bWUuY29tMScwJQYJKoZIhvcNAQkBFhhlbWFpbGFkZHJlc3NAbXllbWFpbC5jb20w",
            "HhcNMjQwNjEzMDY0OTU2WhcNMjQwNzEzMDY0OTU2WjCBkjELMAkGA1UEBhMCQUIx",
            "CzAJBgNVBAgMAkNEMRAwDgYDVQQHDAdFRkdfSElKMQ4wDAYDVQQKDAVNeU9yZzES",
            "MBAGA1UECwwJTXlPcmdVbml0MRcwFQYDVQQDDA5teWNvbW1uYW1lLmNvbTEnMCUG",
            "CSqGSIb3DQEJARYYZW1haWxhZGRyZXNzQG15ZW1haWwuY29tMCowBQYDK2VwAyEA",
            "tFuCleJwHS28jUCT+ulLl5c1+MXhehhDz2SimOhmWaKjITAfMB0GA1UdDgQWBBRg",
            "MBXdOUfcxUmKk9wvcbxYCM8CoTAFBgMrZXADQQBUM4ZxsCuGwPKRrICvlPYBEhtv",
            "h6dzbzu7+YbpdIPV5jS1tufBSyhxRK9YPaXNYeKeNqKQURWDNLiZXJLZq3QL",
            "-----END CERTIFICATE-----",
        )
    }

    #[test]
    fn certificate_from_file_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let certificate_path = dir.path().join("cert.pem");
        std::fs::write(&certificate_path, certificate_str()).expect("Cannot create cert.pem file");

        let cert =
            Certificate::from_file(certificate_path).expect("Cannot create certificate from file");

        let cert_public_key = cert.subject_public_key().expect("Cannot get public key");

        let expected_public_key =
            PublicKey::from_str(&public_key_str()).expect("Cannot parse public key");

        assert_eq!(cert_public_key, expected_public_key);
    }
}
