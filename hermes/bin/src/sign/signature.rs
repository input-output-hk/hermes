//! Hermes COSE signature implementation.

// cspell: words Coap

use std::io::Read;

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

impl<T: SignaturePayloadEncoding> Signature<T> {
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
        let json = self.payload.to_json();
        let payload_bytes = serde_json::to_vec(&json)?;
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

    /// Convert `Signature` object to CBOR decoded bytes.
    pub(crate) fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        self.build_cose_sign()?
            .to_vec()
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    /// Create new `Signature` object from CBOR encoded bytes reader.
    pub(crate) fn from_reader(mut reader: impl Read) -> anyhow::Result<Self> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Self::from_bytes(&bytes)
    }

    /// Create new `Signature` object from CBOR encoded bytes.
    pub(crate) fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let cose_sign = CoseSign::from_slice(bytes).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let payload_bytes = cose_sign
            .payload
            .ok_or(anyhow::anyhow!("Missing signature payload."))?;

        let json = serde_json::from_slice(payload_bytes.as_slice())?;
        let payload = T::from_json(json)?;

        Ok(Self {
            cose_signatures: cose_sign.signatures,
            payload,
        })
    }
}

/// Signature payload encoding trait.
/// Defines how to encode and decode signature payload object from JSON.
pub(crate) trait SignaturePayloadEncoding {
    /// Encode signature payload object to bytes.
    fn to_json(&self) -> serde_json::Value;

    /// Decode signature payload object from bytes.
    fn from_json(json: serde_json::Value) -> anyhow::Result<Self>
    where Self: Sized;
}

impl SignaturePayloadEncoding for serde_json::Value {
    fn to_json(&self) -> serde_json::Value {
        self.clone()
    }

    fn from_json(json: serde_json::Value) -> anyhow::Result<Self>
    where Self: Sized {
        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sign::{certificate::tests::certificate_str, keys::tests::private_key_str};

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

    #[test]
    fn signature_format_test() {
        let payload = serde_json::json!({ "key": "value" });
        let mut signature = Signature::new(payload.clone());

        let private_key =
            PrivateKey::from_str(&private_key_str()).expect("Cannot create private key");
        let certificate = Certificate::from_str(&certificate_str()).expect("Cannot create cert");

        signature
            .add_sign(&private_key, &certificate)
            .expect("Failed to add signature.");

        let bytes = signature
            .to_bytes()
            .expect("Failed to serialize signature.");
        let cose_sign = CoseSign::from_slice(bytes.as_slice()).expect("cannot decode CoseSign");

        assert_eq!(
            cose_sign.protected.header.alg,
            Some(coset::RegisteredLabelWithPrivate::Assigned(
                iana::Algorithm::EdDSA
            ))
        );
        assert_eq!(
            cose_sign.protected.header.content_type,
            Some(coset::RegisteredLabel::Assigned(
                iana::CoapContentFormat::Json
            ))
        );
        assert_eq!(cose_sign.payload, Some(payload.to_string().into_bytes()));

        let first_signature = cose_sign
            .signatures
            .first()
            .expect("cannot get first signature");
        assert_eq!(
            first_signature.protected.header.key_id,
            certificate
                .hash()
                .expect("cannot get certificate hash")
                .to_bytes()
                .to_vec()
        );
    }
}
