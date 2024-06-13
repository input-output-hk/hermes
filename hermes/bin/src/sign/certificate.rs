//! x.509 certificate implementation.

use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use x509_cert::der::DecodePem;

use super::keys::PublicKey;

/// Certificate file open and read error.
#[derive(thiserror::Error, Debug)]
pub(crate) struct CertificateFileError(PathBuf, Option<anyhow::Error>);
impl Display for CertificateFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = format!(
            "Cannot open and read certificate file at {0}.",
            self.0.display()
        );
        let err = self
            .1
            .as_ref()
            .map(|msg| format!("{msg}"))
            .unwrap_or_default();
        writeln!(f, "{msg}\n{err}",)
    }
}

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
        let str = std::fs::read_to_string(&path)
            .map_err(|_| CertificateFileError(path.as_ref().into(), None))?;
        Ok(Self::from_str(&str)
            .map_err(|err| CertificateFileError(path.as_ref().into(), Some(err)))?)
    }

    /// Create new certificate from string decoded in PEM format
    pub(crate) fn from_str(str: &str) -> anyhow::Result<Self> {
        let cert = x509_cert::Certificate::from_pem(str.as_bytes())?;
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
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn certificate_from_file_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let private_key_path = dir.path().join("cert.pem");
        let private_key = format!(
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
        );
        std::fs::write(&private_key_path, private_key).expect("Cannot create cert.pem file");

        let cert =
            Certificate::from_file(private_key_path).expect("Cannot create certificate from file");

        let cert_public_key = cert.subject_public_key().expect("Cannot get public key");

        let expected_public_key = PublicKey::from_str(&format!(
            "{}\n{}\n{}",
            "-----BEGIN PUBLIC KEY-----",
            "MCowBQYDK2VwAyEAtFuCleJwHS28jUCT+ulLl5c1+MXhehhDz2SimOhmWaI=",
            "-----END PUBLIC KEY-----"
        ))
        .expect("Cannot parse public key");

        assert_eq!(cert_public_key, expected_public_key);
    }
}
