//! Hermes COSE signature implementation.

// cspell: words Coap

use std::io::Read;

use coset::{
    iana, CborSerializable, CoseSign, CoseSignBuilder, CoseSignature, CoseSignatureBuilder, Header,
    HeaderBuilder,
};

use super::{
    super::hash::Blake2b256,
    certificate::{self, Certificate},
    keys::PrivateKey,
};
use crate::errors::Errors;

/// COSE signature object.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Signature<T> {
    /// Signatures collection.
    cose_signatures: Vec<CoseSignature>,
    /// Signature payload object.
    payload: T,
}

impl<T: SignaturePayloadEncoding> Signature<T> {
    /// Create new `Signature` object.
    pub(crate) fn new(payload: T) -> Self {
        Self {
            cose_signatures: Vec::new(),
            payload,
        }
    }

    /// Get the payload object.
    pub(crate) fn payload(&self) -> &T {
        &self.payload
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

    /// Verify the `Signature` object.
    /// Validate all signatures, how they correspond to the certificate and public key.
    /// Returns `Ok(())` if signature is valid.
    ///
    /// # Note:
    /// Before verifying, all related to the added signatures should be added to the
    /// certificate storage `certificate::storage::add_certificate()`.
    pub(crate) fn verify(&self) -> anyhow::Result<()> {
        let cose_sign = self.build_cose_sign()?;
        anyhow::ensure!(!cose_sign.signatures.is_empty(), "Empty signatures list.");

        let mut errors = Errors::new();
        for (i, cose_signature) in cose_sign.signatures.iter().enumerate() {
            Self::verify_cose_sign(&cose_sign, i, cose_signature)
                .unwrap_or_else(errors.get_add_err_fn());
        }
        errors.return_result(())
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
        Self::validate_cose_protected_header(&cose_sign)?;

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

impl<T: SignaturePayloadEncoding> Signature<T> {
    /// Verify the `CoseSignature` object.
    fn verify_cose_sign(
        cose_sign: &CoseSign, i: usize, cose_signature: &CoseSignature,
    ) -> anyhow::Result<()> {
        let kid = &cose_signature.protected.header.key_id;

        let cert_hash = Blake2b256::from_bytes(kid).map_err(|err| {
            anyhow::anyhow!("Failed to decode signature `kid` value to `Blake2b256` hash. {err}",)
        })?;
        let cert = certificate::storage::get_certificate(&cert_hash).ok_or(anyhow::anyhow!(
            "Cannot find certificate in the storage, cert hash `{}`.",
            cert_hash.to_hex()
        ))?;
        let public_key = cert.subject_public_key()?;

        cose_sign.verify_signature(i, &[], |signature_bytes, msg| {
            public_key.verify(msg, signature_bytes)
        })
    }

    /// Validate `CoseSign` protected header.
    fn validate_cose_protected_header(cose_sign: &CoseSign) -> anyhow::Result<()> {
        let header = &cose_sign.protected.header;
        let expected_header = Self::build_cose_protected_header();

        anyhow::ensure!(
            header.alg == expected_header.alg,
            "Invalid COSE signature protected header `alg` value."
        );
        anyhow::ensure!(
            header.content_type == expected_header.content_type,
            "Invalid COSE signature protected header `content_type` value."
        );

        Ok(())
    }

    /// Add a new signature to the `Self::cose_signatures` field.
    fn add_cose_signature(
        &mut self, private_key: &PrivateKey, certificate: &Certificate,
    ) -> anyhow::Result<()> {
        let empty_signature = Self::build_empty_cose_signature(certificate)?;

        // check for duplicate
        if self
            .cose_signatures
            .iter()
            .any(|sign| sign.protected.header.key_id == empty_signature.protected.header.key_id)
        {
            return Ok(());
        }

        // sign with private key
        let cose_sign = self
            .prepare_cose_sign_builder()?
            .add_created_signature(empty_signature, &[], |data| private_key.sign(data))
            .build();
        if let Some(new_signature) = cose_sign.signatures.into_iter().next() {
            self.cose_signatures.push(new_signature);
        }

        Ok(())
    }

    /// Build empty `CoseSignature` object.
    fn build_empty_cose_signature(certificate: &Certificate) -> anyhow::Result<CoseSignature> {
        let protected_header = Self::build_cose_sign_protected_header(certificate)?;
        let res = CoseSignatureBuilder::new()
            .protected(protected_header)
            .build();
        Ok(res)
    }

    /// Return a protected header for `CoseSignatureBuilder`.
    fn build_cose_sign_protected_header(certificate: &Certificate) -> anyhow::Result<Header> {
        let certificate_hash_bytes = certificate.hash()?.to_bytes().to_vec();
        let header = HeaderBuilder::new().key_id(certificate_hash_bytes).build();
        Ok(header)
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

    /// Return a protected header for `CoseSignBuilder`.
    fn build_cose_protected_header() -> Header {
        HeaderBuilder::new()
            .algorithm(iana::Algorithm::EdDSA)
            .content_format(iana::CoapContentFormat::Json)
            .build()
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
    use crate::packaging::sign::{
        certificate::tests::certificate_str, keys::tests::private_key_str,
    };

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

        let cose_sign = CoseSign::from_slice(bytes.as_slice()).expect("Failed to decode CoseSign.");
        assert!(Signature::<serde_json::Value>::from_bytes(
            &cose_sign
                .clone()
                .to_vec()
                .expect("Failed to encode CoseSign.")
        )
        .is_ok());

        let mut cose_sign_modified_alg = cose_sign.clone();
        cose_sign_modified_alg.protected.original_data = None;
        cose_sign_modified_alg.protected.header.alg = None;
        assert!(Signature::<serde_json::Value>::from_bytes(
            &cose_sign_modified_alg
                .clone()
                .to_vec()
                .expect("Failed to encode CoseSign.")
        )
        .is_err());

        cose_sign_modified_alg.protected.original_data = None;
        cose_sign_modified_alg.protected.header.alg = Some(
            coset::RegisteredLabelWithPrivate::Assigned(iana::Algorithm::ES256),
        );
        assert!(Signature::<serde_json::Value>::from_bytes(
            &cose_sign_modified_alg
                .to_vec()
                .expect("Failed to encode CoseSign.")
        )
        .is_err());

        let mut cose_sign_modified_content_type = cose_sign.clone();
        cose_sign_modified_content_type.protected.original_data = None;
        cose_sign_modified_content_type
            .protected
            .header
            .content_type = None;
        assert!(Signature::<serde_json::Value>::from_bytes(
            &cose_sign_modified_content_type
                .clone()
                .to_vec()
                .expect("Failed to encode CoseSign.")
        )
        .is_err());

        cose_sign_modified_content_type.protected.original_data = None;
        cose_sign_modified_content_type
            .protected
            .header
            .content_type = Some(coset::RegisteredLabel::Assigned(
            iana::CoapContentFormat::Cbor,
        ));
        assert!(Signature::<serde_json::Value>::from_bytes(
            &cose_sign_modified_content_type
                .clone()
                .to_vec()
                .expect("Failed to encode CoseSign.")
        )
        .is_err());
    }

    #[test]
    fn signature_add_sign_test() {
        let payload = serde_json::json!({ "key": "value" });
        let mut signature = Signature::new(payload);

        let private_key =
            PrivateKey::from_str(&private_key_str()).expect("Cannot create private key");
        let certificate = Certificate::from_str(&certificate_str()).expect("Cannot create cert");

        signature
            .add_sign(&private_key, &certificate)
            .expect("Failed to add signature.");
        assert_eq!(signature.cose_signatures.len(), 1);
        signature
            .add_sign(&private_key, &certificate)
            .expect("Failed to add signature twice with the same private key.");
        assert_eq!(signature.cose_signatures.len(), 1);

        let another_private_key = PrivateKey::from_str(&format!(
            "{}\n{}\n{}",
            "-----BEGIN PRIVATE KEY-----",
            "MC4CAQAwBQYDK2VwBCIEIP1iI3LF7h89yY6QZmhDp4Y5FmTQ4oasbz2lEiaqqTz5",
            "-----END PRIVATE KEY-----"
        ))
        .expect("Failed to create private key.");
        assert_ne!(private_key, another_private_key);

        assert!(
            signature
                .add_sign(&another_private_key, &certificate)
                .is_err(),
            "Private key must be associated with the certificate."
        );
    }

    #[test]
    fn signature_verify_test() {
        let payload = serde_json::json!({ "key": "value" });
        let mut signature = Signature::new(payload);

        let private_key =
            PrivateKey::from_str(&private_key_str()).expect("Cannot create private key");
        let certificate = Certificate::from_str(&certificate_str()).expect("Cannot create cert");

        assert!(
            signature.verify().is_err(),
            "Empty signature must be invalid."
        );

        signature
            .add_sign(&private_key, &certificate)
            .expect("Failed to add signature.");

        assert!(
            signature.verify().is_err(),
            "Missing certificate in the storage."
        );

        certificate::storage::add_certificate(certificate)
            .expect("Failed to add certificate to the storage.");
        signature.verify().expect("Failed to verify signature.");

        // corrupt signature
        let bytes = signature
            .to_bytes()
            .expect("Failed to serialize signature.");
        let mut cose_sign =
            CoseSign::from_slice(bytes.as_slice()).expect("Failed to decode CoseSign.");
        // change payload
        cose_sign.payload = Some(
            serde_json::json!("corrupted")
                .to_string()
                .as_bytes()
                .to_vec(),
        );
        let signature = Signature::<serde_json::Value>::from_bytes(
            &cose_sign.to_vec().expect("Failed to encode CoseSign."),
        )
        .expect("Failed to decode signature.");

        assert!(signature.verify().is_err(), "Corrupted signature.");
    }

    #[test]
    fn signature_format_test() {
        let payload = serde_json::json!({ "key": "value" });
        let mut signature = Signature::new(payload.clone());

        let private_key =
            PrivateKey::from_str(&private_key_str()).expect("Failed to create private key.");
        let certificate =
            Certificate::from_str(&certificate_str()).expect("Failed to create cert.");

        signature
            .add_sign(&private_key, &certificate)
            .expect("Failed to add signature.");

        let bytes = signature
            .to_bytes()
            .expect("Failed to serialize signature.");
        let cose_sign = CoseSign::from_slice(bytes.as_slice()).expect("Failed to decode CoseSign.");

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
            .expect("Failed to get first signature.");
        assert_eq!(
            first_signature.protected.header.key_id,
            certificate
                .hash()
                .expect("Failed to get certificate hash.")
                .to_bytes()
                .to_vec()
        );
    }
}
