//! Hermes COSE signature implementation.

use coset::{
    iana, CborSerializable, CoseSign, CoseSignBuilder, CoseSignature, CoseSignatureBuilder, Header,
    HeaderBuilder,
};

use super::{certificate::Certificate, keys::PrivateKey};

/// COSE signature object.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Signature<T> {
    /// Signatures collection.
    cose_signatures: Vec<CoseSignature>,
    /// Signature payload object.
    payload: T,
}

impl<T: serde::Serialize + serde::de::DeserializeOwned> Signature<T> {
    /// Return a protected header for `CoseSignBuilder`.
    fn build_cose_protected_header() -> Header {
        HeaderBuilder::new()
            .algorithm(iana::Algorithm::EdDSA)
            .content_format(iana::CoapContentFormat::Json)
            .build()
    }

    /// Return a protected header for `CoseSignatureBuilder`.
    fn build_cose_sign_protected_header(certificate: &Certificate) -> anyhow::Result<Header> {
        let certificate_hash_bytes = certificate.hash()?.to_bytes().to_vec();
        let header = HeaderBuilder::new().key_id(certificate_hash_bytes).build();
        Ok(header)
    }

    /// Build empty `CoseSignature` object.
    fn build_empty_cose_signature(certificate: &Certificate) -> anyhow::Result<CoseSignature> {
        let protected_header = Self::build_cose_sign_protected_header(certificate)?;
        let res = CoseSignatureBuilder::new()
            .protected(protected_header)
            .build();
        Ok(res)
    }

    /// Prepare `CoseSignBuilder` object with necessary data.
    fn prepare_cose_sign_builder(&self) -> anyhow::Result<CoseSignBuilder> {
        let payload_bytes = serde_json::to_vec(&self.payload)?;
        let protected_header = Self::build_cose_protected_header();

        let builder = CoseSignBuilder::new()
            .protected(protected_header)
            .payload(payload_bytes);
        Ok(builder)
    }

    /// Build a `CoseSign` object.
    fn build_cose_sign(&self) -> anyhow::Result<CoseSign> {
        let builder = self.prepare_cose_sign_builder()?;

        let res = self
            .cose_signatures
            .iter()
            .fold(builder, |builder, signature| {
                builder.add_signature(signature.clone())
            })
            .build();
        Ok(res)
    }

    /// Create new `Signature` object.
    pub(crate) fn new(payload: T) -> Self {
        Self {
            cose_signatures: Vec::new(),
            payload,
        }
    }

    /// Add a new signature to the `CoseSign` object.
    pub(crate) fn add_sign(
        &mut self, private_key: &PrivateKey, certificate: &Certificate,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            private_key.public_key() == certificate.subject_public_key()?,
            "Certificate subject public key doesn't associated with the signing private key."
        );

        self.add_cose_signature(private_key, certificate)?;

        Ok(())
    }

    /// Add a new signature to the `Self::cose_signatures` field.
    fn add_cose_signature(
        &mut self, private_key: &PrivateKey, certificate: &Certificate,
    ) -> anyhow::Result<()> {
        let empty_signature = Self::build_empty_cose_signature(certificate)?;

        // sign with private key
        let signature = self
            .prepare_cose_sign_builder()?
            .add_created_signature(empty_signature, &[], |data| private_key.sign(data))
            .build()
            .signatures;

        self.cose_signatures.extend(signature);
        Ok(())
    }

    /// Get signature payload JSON object.
    pub(crate) fn payload(&self) -> &T {
        &self.payload
    }

    /// Convert `Signature` object to CBOR decoded bytes.
    pub(crate) fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        self.build_cose_sign()?
            .to_vec()
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    /// Create new `Signature` object from CBOR decoded bytes.
    pub(crate) fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let cose_sign = CoseSign::from_slice(bytes).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let payload_bytes = cose_sign
            .payload
            .ok_or(anyhow::anyhow!("Missing signature payload."))?;

        let payload = serde_json::from_slice(payload_bytes.as_slice())?;

        Ok(Self {
            cose_signatures: cose_sign.signatures,
            payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_serde_test() {
        let payload = serde_json::json!({ "key": "value" });
        let signature = Signature::new(payload);

        let bytes = signature
            .to_bytes()
            .expect("Failed to serialize signature.");

        let decoded_signature = Signature::<serde_json::Value>::from_bytes(&bytes)
            .expect("Failed to deserialize signature.");

        assert_eq!(signature, decoded_signature);
    }
}
