//! ED25519 public and private key implementation.

// cspell: words outpubkey genpkey

use std::path::Path;

use ed25519_dalek::{
    ed25519::signature::Signer,
    pkcs8::{DecodePrivateKey, DecodePublicKey},
    SigningKey, VerifyingKey,
};

use crate::packaging::FileError;

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
        let str = std::fs::read_to_string(&path).map_err(|_| FileError::from_path(&path, None))?;
        Ok(Self::from_str(&str).map_err(|err| FileError::from_path(&path, Some(err)))?)
    }

    /// Create new private key from string decoded in PEM format
    pub(crate) fn from_str(str: &str) -> anyhow::Result<Self> {
        let key = SigningKey::from_pkcs8_pem(str).map_err(|_| KeyPemDecodingError)?;
        Ok(Self(key))
    }

    /// Get associated public key.
    pub(crate) fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key())
    }

    /// Sign the message with the current private key.
    /// Returns the signature bytes.
    pub(crate) fn sign(&self, msg: &[u8]) -> Vec<u8> {
        self.0.sign(msg).to_vec()
    }
}

/// Ed25519 public key instance.
/// Wrapper over `ed25519_dalek::VerifyingKey`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PublicKey(VerifyingKey);

impl PublicKey {
    /// Create new public key from file decoded in PEM format.
    #[allow(dead_code)]
    pub(crate) fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let str = std::fs::read_to_string(&path).map_err(|_| FileError::from_path(&path, None))?;
        Ok(Self::from_str(&str).map_err(|err| FileError::from_path(&path, Some(err)))?)
    }

    /// Create new public key from string decoded in PEM format.
    #[allow(dead_code)]
    pub(crate) fn from_str(str: &str) -> anyhow::Result<Self> {
        let key = VerifyingKey::from_public_key_pem(str).map_err(|_| KeyPemDecodingError)?;
        Ok(Self(key))
    }

    /// Create new public key from raw bytes.
    pub(crate) fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let key = VerifyingKey::from_bytes(bytes.try_into()?)?;
        Ok(Self(key))
    }

    /// Verify signature of the message with the current public key.
    /// Returns `Ok(())` if the signature is valid, `Err` otherwise.
    #[allow(dead_code)]
    pub(crate) fn verify(&self, msg: &[u8], signature_bytes: &[u8]) -> anyhow::Result<()> {
        let signature_bytes = signature_bytes.try_into().map_err(|_| {
            anyhow::anyhow!(
                "Invalid signature bytes size: expected {}, provided {}.",
                ed25519_dalek::Signature::BYTE_SIZE,
                signature_bytes.len()
            )
        })?;
        let signature = ed25519_dalek::Signature::from_bytes(signature_bytes);
        self.0.verify_strict(msg, &signature)?;
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use temp_dir::TempDir;

    use super::*;

    /// An Ed25519 private key in PEM format.
    /// Generated with `openssl` tool:
    /// ```shell
    /// openssl genpkey -algorithm=ED25519 -out=private.pem -outpubkey=public.pem
    /// ```
    pub(crate) fn private_key_str() -> String {
        format!(
            "{}\n{}\n{}",
            "-----BEGIN PRIVATE KEY-----",
            "MC4CAQAwBQYDK2VwBCIEIP1iI3LF7h89yY6QZmhDp4Y5FmTQ4oasbz2lEiaqqTzV",
            "-----END PRIVATE KEY-----"
        )
    }

    /// An Ed25519 public key in PEM format.
    /// This public key is corresponding to the `private_key_str()` private key.
    /// Generated with `openssl` tool:
    /// ```shell
    /// openssl genpkey -algorithm=ED25519 -out=private.pem -outpubkey=public.pem
    /// ```
    pub(crate) fn public_key_str() -> String {
        format!(
            "{}\n{}\n{}",
            "-----BEGIN PUBLIC KEY-----",
            "MCowBQYDK2VwAyEAtFuCleJwHS28jUCT+ulLl5c1+MXhehhDz2SimOhmWaI=",
            "-----END PUBLIC KEY-----"
        )
    }

    #[test]
    fn private_key_from_file_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let private_key_path = dir.path().join("private.pem");
        std::fs::write(&private_key_path, private_key_str())
            .expect("Cannot create private.pem file");

        let _key =
            PrivateKey::from_file(private_key_path).expect("Cannot create private key from file");
    }

    #[test]
    fn public_key_from_file_test() {
        let dir = TempDir::new().expect("cannot create temp dir");

        let public_key_path = dir.path().join("public.pem");
        std::fs::write(&public_key_path, public_key_str()).expect("Cannot create public.pem file");

        let _key =
            PublicKey::from_file(public_key_path).expect("Cannot create public key from file");
    }

    #[test]
    fn public_private_key_test() {
        let private_key =
            PrivateKey::from_str(&private_key_str()).expect("Cannot create private key");
        let public_key = PublicKey::from_str(&public_key_str()).expect("Cannot create public key");

        assert_eq!(private_key.public_key(), public_key);
    }

    #[test]
    fn sign_test() {
        let private_key =
            PrivateKey::from_str(&private_key_str()).expect("Cannot create private key");
        let public_key = PublicKey::from_str(&public_key_str()).expect("Cannot create public key");

        let msg = b"test";

        let signature = private_key.sign(msg);
        assert!(public_key.verify(msg, &signature).is_ok());
    }
}
