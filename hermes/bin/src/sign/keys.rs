//! ED25519 public and private key implementation.

use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use ed25519::pkcs8::DecodePublicKey;
use ed25519_dalek::{pkcs8::DecodePrivateKey, SigningKey, VerifyingKey};

/// Public or private key file open and read error.
#[derive(thiserror::Error, Debug)]
pub(crate) struct KeyFileError(PathBuf, Option<anyhow::Error>);
impl Display for KeyFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = format!("Cannot open and read key file at {0}.", self.0.display());
        let err = self
            .1
            .as_ref()
            .map(|msg| format!("{msg}"))
            .unwrap_or_default();
        writeln!(f, "{msg}\n{err}",)
    }
}

/// Public or private key decoding from string error.
#[derive(thiserror::Error, Debug)]
#[error("Cannot decode key from string. Invalid PEM format.")]
pub(crate) struct KeyPemDecodingError;

/// Ed25519 private key instance.
/// Wrapper over `ed25519_dalek::SigningKey`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PrivateKey(SigningKey);

impl PrivateKey {
    /// Create new private key from file decoded in PEM format
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let str =
            std::fs::read_to_string(&path).map_err(|_| KeyFileError(path.as_ref().into(), None))?;
        Ok(Self::from_str(&str).map_err(|err| KeyFileError(path.as_ref().into(), Some(err)))?)
    }

    /// Create new private key from string decoded in PEM format
    pub(crate) fn from_str(str: &str) -> anyhow::Result<Self> {
        let key = SigningKey::from_pkcs8_pem(str).map_err(|_| KeyPemDecodingError)?;
        Ok(Self(key))
    }
}

/// Ed25519 public key instance.
/// Wrapper over `ed25519_dalek::VerifyingKey`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PublicKey(VerifyingKey);

impl PublicKey {
    /// Create new public key from file decoded in PEM format.
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let str =
            std::fs::read_to_string(&path).map_err(|_| KeyFileError(path.as_ref().into(), None))?;
        Ok(Self::from_str(&str).map_err(|err| KeyFileError(path.as_ref().into(), Some(err)))?)
    }

    /// Create new public key from string decoded in PEM format.
    pub(crate) fn from_str(str: &str) -> anyhow::Result<Self> {
        let key = VerifyingKey::from_public_key_pem(str).map_err(|_| KeyPemDecodingError)?;
        Ok(Self(key))
    }

    /// Create new public key from raw bytes.
    pub(crate) fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let key = VerifyingKey::from_bytes(bytes.try_into()?)?;
        Ok(Self(key))
    }
}

#[cfg(test)]
mod tests {
    use temp_dir::TempDir;

    use super::*;

    #[test]
    fn private_key_from_file_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let private_key_path = dir.path().join("private.pem");
        let private_key = format!(
            "{}\n{}\n{}",
            "-----BEGIN PRIVATE KEY-----",
            "MC4CAQAwBQYDK2VwBCIEIP1iI3LF7h89yY6QZmhDp4Y5FmTQ4oasbz2lEiaqqTzV",
            "-----END PRIVATE KEY-----"
        );
        std::fs::write(&private_key_path, private_key).expect("Cannot create private.pem file");

        let _key =
            PrivateKey::from_file(private_key_path).expect("Cannot create private key from file");
    }

    #[test]
    fn public_key_from_file_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let private_key_path = dir.path().join("public.pem");
        let private_key = format!(
            "{}\n{}\n{}",
            "-----BEGIN PUBLIC KEY-----",
            "MCowBQYDK2VwAyEAtFuCleJwHS28jUCT+ulLl5c1+MXhehhDz2SimOhmWaI=",
            "-----END PUBLIC KEY-----"
        );
        std::fs::write(&private_key_path, private_key).expect("Cannot create public.pem file");

        let _key =
            PublicKey::from_file(private_key_path).expect("Cannot create private key from file");
    }
}
