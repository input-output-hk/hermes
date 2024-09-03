//! Cardano Improvement Proposal 509 (CIP-509) metadata module.
//! Doc Reference: <https://github.com/input-output-hk/catalyst-CIPs/tree/x509-envelope-metadata/CIP-XXXX>
//! CDDL Reference: <https://github.com/input-output-hk/catalyst-CIPs/blob/x509-envelope-metadata/CIP-XXXX/x509-envelope.cddl>

// cspell: words pkix
use c509_certificate::general_names::general_name::GeneralNameValue;
use decode_helper::{decode_bytes, decode_map_len, decode_u8};
use der_parser::{asn1_rs::oid, der::parse_der_sequence, Oid};
use rbac::{certs::C509Cert, role_data::RoleData};

mod decode_helper;
mod rbac;
use x509_cert::{der::Decode as _, ext::pkix::ID_CE_SUBJECT_ALT_NAME};
mod x509_chunks;

use std::sync::Arc;

use minicbor::{
    decode::{self},
    Decode, Decoder,
};
use pallas::{
    codec::{
        minicbor::{Encode, Encoder},
        utils::Bytes,
    },
    ledger::traverse::MultiEraTx,
};
use strum::FromRepr;
use x509_chunks::X509Chunks;

use super::{
    raw_aux_data::RawAuxData, DecodedMetadata, DecodedMetadataItem, DecodedMetadataValues,
    ValidationReport,
};
use crate::{
    utils::{
        blake2b_128, blake2b_256, compare_key_hash, decode_utf8, extract_cip19_hash,
        extract_key_hash, zero_out_last_n_bytes,
    },
    witness::TxWitness,
    Network,
};

/// CIP509 label.
pub const LABEL: u64 = 509;

/// Context-specific primitive type with tag number 6 (`raw_tag` 134) for
/// uniform resource identifier (URI) in the subject alternative name extension.
pub(crate) const URI: u8 = 134;

/// Subject Alternative Name OID
pub(crate) const SUBJECT_ALT_NAME_OID: Oid = oid!(2.5.29 .17);

/// CIP509 metadatum.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Cip509 {
    /// `UUIDv4` Purpose .
    pub purpose: [u8; 16], // (bytes .size 16)
    /// Transaction inputs hash.
    pub txn_inputs_hash: [u8; 16], // bytes .size 16
    /// Optional previous transaction ID.
    pub prv_tx_id: Option<[u8; 32]>, // bytes .size 32
    /// x509 chunks.
    pub x509_chunks: X509Chunks, // chunk_type => [ + x509_chunk ]
    /// Validation signature.
    pub validation_signature: Vec<u8>, // bytes size (1..64)
    /// Validation value, not a part of CIP509, justs storing validity of the data.
    pub validation: Cip509Validation,
}

/// Validation value for CIP509 metadatum.
#[allow(clippy::struct_excessive_bools, clippy::module_name_repetitions)]
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Cip509Validation {
    /// Boolean value for the validity of the transaction inputs hash.
    pub valid_txn_inputs_hash: bool,
    /// Boolean value for the validity of the auxiliary data.
    pub valid_aux: bool,
    /// Bytes of precomputed auxiliary data.
    pub precomputed_aux: Vec<u8>,
    /// Boolean value for the validity of the public key.
    pub valid_public_key: bool,
    /// Boolean value for the validity of the payment key.
    pub valid_payment_key: bool,
}

/// Enum of CIP509 metadatum with its associated unsigned integer value.
#[allow(clippy::module_name_repetitions)]
#[derive(FromRepr, Debug, PartialEq)]
#[repr(u8)]
pub(crate) enum Cip509Int {
    /// Purpose.
    Purpose = 0,
    /// Transaction inputs hash.
    TxInputsHash = 1,
    /// Previous transaction ID.
    PreviousTxId = 2,
    /// Validation signature.
    ValidationSignature = 99,
}

impl Decode<'_, ()> for Cip509 {
    fn decode(d: &mut Decoder, ctx: &mut ()) -> Result<Self, decode::Error> {
        let map_len = decode_map_len(d, "CIP509")?;
        let mut cip509_metadatum = Cip509::default();
        for _ in 0..map_len {
            // Use probe to peak
            let key = d.probe().u8()?;
            if let Some(key) = Cip509Int::from_repr(key) {
                // Consuming the int
                decode_u8(d, "CIP509")?;
                match key {
                    Cip509Int::Purpose => {
                        cip509_metadatum.purpose = decode_bytes(d, "CIP509 purpose")?
                            .try_into()
                            .map_err(|_| decode::Error::message("Invalid data size of Purpose"))?;
                    },
                    Cip509Int::TxInputsHash => {
                        cip509_metadatum.txn_inputs_hash =
                            decode_bytes(d, "CIP509 txn inputs hash")?
                                .try_into()
                                .map_err(|_| {
                                    decode::Error::message("Invalid data size of TxInputsHash")
                                })?;
                    },
                    Cip509Int::PreviousTxId => {
                        cip509_metadatum.prv_tx_id = Some(
                            decode_bytes(d, "CIP509 previous tx ID")?
                                .try_into()
                                .map_err(|_| {
                                    decode::Error::message("Invalid data size of PreviousTxId")
                                })?,
                        );
                    },
                    Cip509Int::ValidationSignature => {
                        let validation_signature = decode_bytes(d, "CIP509 validation signature")?;
                        if validation_signature.is_empty() || validation_signature.len() > 64 {
                            return Err(decode::Error::message(
                                "Invalid data size of ValidationSignature",
                            ));
                        }
                        cip509_metadatum.validation_signature = validation_signature;
                    },
                }
            } else {
                // Handle the x509 chunks 10 11 12
                let x509_chunks = X509Chunks::decode(d, ctx)?;
                cip509_metadatum.x509_chunks = x509_chunks;
            }
        }
        Ok(cip509_metadatum)
    }
}

#[allow(clippy::module_name_repetitions)]
impl Cip509 {
    /// Decode and validate CIP509 metadata.
    pub(crate) fn decode_and_validate(
        decoded_metadata: &DecodedMetadata, _slot: u64, txn: &MultiEraTx,
        raw_aux_data: &RawAuxData, _chain: Network, txn_idx: usize,
    ) {
        // Get the CIP509 metadata if possible
        let Some(k509) = raw_aux_data.get_metadata(LABEL) else {
            return;
        };

        let mut validation_report = ValidationReport::new();
        let mut decoder = Decoder::new(k509.as_slice());

        let mut cip509 = match Cip509::decode(&mut decoder, &mut ()) {
            Ok(metadata) => metadata,
            Err(e) => {
                Cip509::default().validation_failure(
                    &format!("Failed to decode CIP509 metadata: {e}"),
                    &mut validation_report,
                    decoded_metadata,
                );
                return;
            },
        };

        // Validate transaction inputs hash
        match cip509.validate_txn_inputs_hash(txn, &mut validation_report, decoded_metadata) {
            Some(b) => cip509.validation.valid_txn_inputs_hash = b,
            None => {
                cip509.validation_failure(
                    "Failed to validate transaction inputs hash",
                    &mut validation_report,
                    decoded_metadata,
                );
            },
        }

        // Validate the auxiliary data
        match cip509.validate_aux(txn, &mut validation_report, decoded_metadata) {
            Some(b) => cip509.validation.valid_aux = b,
            None => {
                cip509.validation_failure(
                    "Failed to validate auxiliary data",
                    &mut validation_report,
                    decoded_metadata,
                );
            },
        }

        // Validate the role 0
        if let Some(role_set) = &cip509.x509_chunks.0.role_set {
            // Validate only role 0
            for role in role_set {
                if role.role_number == 0 {
                    // Validate public key to in certificate to the witness set in transaction
                    match cip509.validate_public_key(
                        txn,
                        &mut validation_report,
                        decoded_metadata,
                        txn_idx,
                    ) {
                        Some(b) => cip509.validation.valid_public_key = b,
                        None => {
                            cip509.validation_failure(
                                &format!("Failed to validate public key in tx id {txn_idx}"),
                                &mut validation_report,
                                decoded_metadata,
                            );
                        },
                    }
                    // Validate payment key reference
                    match cip509.validate_payment_key(
                        txn,
                        &mut validation_report,
                        decoded_metadata,
                        txn_idx,
                        role,
                    ) {
                        Some(b) => cip509.validation.valid_payment_key = b,
                        None => {
                            cip509.validation_failure(
                                &format!("Failed to validate payment key in tx id {txn_idx}"),
                                &mut validation_report,
                                decoded_metadata,
                            );
                        },
                    }
                }
            }
        }
    }

    /// Handle validation failure.
    fn validation_failure(
        &self, reason: &str, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata,
    ) {
        validation_report.push(reason.into());
        decoded_metadata.0.insert(
            LABEL,
            Arc::new(DecodedMetadataItem {
                value: DecodedMetadataValues::Cip509(Arc::new(self.clone()).clone()),
                report: validation_report.clone(),
            }),
        );
    }

    /// Transaction inputs hash validation.
    /// Must exist and match the hash of the transaction inputs.
    fn validate_txn_inputs_hash(
        &self, txn: &MultiEraTx, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata,
    ) -> Option<bool> {
        let mut buffer = Vec::new();
        let mut e = Encoder::new(&mut buffer);
        match txn {
            MultiEraTx::AlonzoCompatible(tx, _) => {
                let inputs = tx.transaction_body.inputs.clone();
                if let Err(e) = e.array(inputs.len() as u64) {
                    self.validation_failure(
                        &format!("Failed to encode array of transaction input in validate_txn_inputs_hash: {e}"),
                        validation_report,
                        decoded_metadata,
                    );
                    return None;
                }
                for input in &inputs {
                    if let Err(e) = input.encode(&mut e, &mut ()) {
                        self.validation_failure(
                            &format!("Failed to encode transaction input in validate_txn_inputs_hash: {e}"),
                            validation_report,
                            decoded_metadata,
                        );
                        return None;
                    }
                }
            },
            MultiEraTx::Babbage(tx) => {
                let inputs = tx.transaction_body.inputs.clone();
                if let Err(e) = e.array(inputs.len() as u64) {
                    self.validation_failure(
                        &format!("Failed to encode array of transaction input in validate_txn_inputs_hash: {e}"),
                        validation_report,
                        decoded_metadata,
                    );
                    return None;
                }
                for input in &inputs {
                    if let Err(e) = input.encode(&mut e, &mut ()) {
                        self.validation_failure(
                            &format!("Failed to encode transaction input in validate_txn_inputs_hash: {e}"),
                            validation_report,
                            decoded_metadata,
                        );
                        return None;
                    }
                }
            },
            MultiEraTx::Conway(tx) => {
                let inputs = tx.transaction_body.inputs.clone();
                if let Err(e) = e.array(inputs.len() as u64) {
                    self.validation_failure(
                        &format!("Failed to encode array of transaction input in validate_txn_inputs_hash: {e}"),
                        validation_report,
                        decoded_metadata,
                    );
                    return None;
                }
                for input in &inputs {
                    match input.encode(&mut e, &mut ()) {
                        Ok(()) => {},
                        Err(e) => {
                            self.validation_failure(
                                &format!(
                                "Failed to encode transaction input in validate_txn_inputs_hash {e}"
                            ),
                                validation_report,
                                decoded_metadata,
                            );
                            return None;
                        },
                    }
                }
            },
            _ => {
                self.validation_failure(
                    "Unsupported transaction era for transaction inputs hash validation",
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        }
        let inputs_hash = match blake2b_128(&buffer) {
            Ok(hash) => hash,
            Err(e) => {
                self.validation_failure(
                    &format!("Failed to hash transaction inputs in validate_txn_inputs_hash {e}"),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        };
        Some(inputs_hash == self.txn_inputs_hash)
    }

    /// Validate the auxiliary data with the auxiliary data hash in the transaction.
    /// Also log out the pre-computed hash where the validation signature (99) set to
    /// zero.
    fn validate_aux(
        &mut self, txn: &MultiEraTx, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata,
    ) -> Option<bool> {
        match txn {
            MultiEraTx::AlonzoCompatible(tx, _) => {
                if let pallas::codec::utils::Nullable::Some(a) = &tx.auxiliary_data {
                    let original_aux = a.raw_cbor();
                    let aux_data_hash =
                        tx.transaction_body
                            .auxiliary_data_hash
                            .as_ref()
                            .or_else(|| {
                                self.validation_failure(
                                    "Auxiliary data hash not found in transaction",
                                    validation_report,
                                    decoded_metadata,
                                );
                                None
                            })?;
                    self.validate_aux_helper(
                        original_aux,
                        aux_data_hash,
                        validation_report,
                        decoded_metadata,
                    )
                } else {
                    self.validation_failure(
                        "Auxiliary data not found in transaction",
                        validation_report,
                        decoded_metadata,
                    );
                    None
                }
            },
            MultiEraTx::Babbage(tx) => {
                if let pallas::codec::utils::Nullable::Some(a) = &tx.auxiliary_data {
                    let original_aux = a.raw_cbor();
                    let aux_data_hash =
                        tx.transaction_body
                            .auxiliary_data_hash
                            .as_ref()
                            .or_else(|| {
                                self.validation_failure(
                                    "Auxiliary data hash not found in transaction",
                                    validation_report,
                                    decoded_metadata,
                                );
                                None
                            })?;
                    self.validate_aux_helper(
                        original_aux,
                        aux_data_hash,
                        validation_report,
                        decoded_metadata,
                    )
                } else {
                    self.validation_failure(
                        "Auxiliary data not found in transaction",
                        validation_report,
                        decoded_metadata,
                    );
                    None
                }
            },
            MultiEraTx::Conway(tx) => {
                if let pallas::codec::utils::Nullable::Some(a) = &tx.auxiliary_data {
                    let original_aux = a.raw_cbor();
                    let aux_data_hash =
                        tx.transaction_body
                            .auxiliary_data_hash
                            .as_ref()
                            .or_else(|| {
                                self.validation_failure(
                                    "Auxiliary data hash not found in transaction",
                                    validation_report,
                                    decoded_metadata,
                                );
                                None
                            })?;
                    self.validate_aux_helper(
                        original_aux,
                        aux_data_hash,
                        validation_report,
                        decoded_metadata,
                    )
                } else {
                    self.validation_failure(
                        "Auxiliary data not found in transaction",
                        validation_report,
                        decoded_metadata,
                    );
                    None
                }
            },
            _ => {
                self.validation_failure(
                    "Unsupported transaction era for auxillary data validation",
                    validation_report,
                    decoded_metadata,
                );
                None
            },
        }
    }

    /// Helper function for auxiliary data validation.
    fn validate_aux_helper(
        &mut self, original_aux: &[u8], aux_data_hash: &Bytes,
        validation_report: &mut ValidationReport, decoded_metadata: &DecodedMetadata,
    ) -> Option<bool> {
        let mut vec_aux = original_aux.to_vec();

        // Zero out the last 64 bytes
        zero_out_last_n_bytes(&mut vec_aux, 64);

        // Pre-computed aux with the last 64 bytes set to zero
        self.validation.precomputed_aux = vec_aux;

        // Compare the hash
        match blake2b_256(original_aux) {
            Ok(original_hash) => {
                return Some(aux_data_hash.as_ref() == original_hash);
            },
            Err(e) => {
                self.validation_failure(
                    &format!("Cannot hash auxiliary data {e}"),
                    validation_report,
                    decoded_metadata,
                );
                None
            },
        }
    }

    /// Validate the public key in the certificate with witness set in transaction.
    #[allow(clippy::too_many_lines)]
    fn validate_public_key(
        &self, txn: &MultiEraTx, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata, txn_idx: usize,
    ) -> Option<bool> {
        let mut pk_addrs = Vec::new();
        match txn {
            MultiEraTx::AlonzoCompatible(..) | MultiEraTx::Babbage(_) | MultiEraTx::Conway(_) => {
                // X509 certificate
                if let Some(x509_certs) = &self.x509_chunks.0.x509_certs {
                    for cert in x509_certs {
                        // Attempt to decode the DER certificate
                        let der_cert = match x509_cert::Certificate::from_der(&cert.0) {
                            Ok(cert) => cert,
                            Err(e) => {
                                self.validation_failure(
                                    &format!("Failed to decode x509 certificate DER: {e}"),
                                    validation_report,
                                    decoded_metadata,
                                );
                                return None;
                            },
                        };

                        // Find the Subject Alternative Name extension
                        let san_ext =
                            der_cert
                                .tbs_certificate
                                .extensions
                                .as_ref()
                                .and_then(|exts| {
                                    exts.iter()
                                        .find(|ext| ext.extn_id == ID_CE_SUBJECT_ALT_NAME)
                                });

                        // Subject Alternative Name extension if it exists
                        if let Some(san_ext) = san_ext {
                            match parse_der_sequence(san_ext.extn_value.as_bytes()) {
                                Ok((_, parsed_seq)) => {
                                    for data in parsed_seq.ref_iter() {
                                        // Check for context-specific primitive type with tag number
                                        // 6 (raw_tag 134)
                                        if data.header.raw_tag() == Some(&[URI]) {
                                            match data.content.as_slice() {
                                                Ok(content) => {
                                                    // Decode the UTF-8 string
                                                    let addr: String = match decode_utf8(content) {
                                                        Ok(addr) => addr,
                                                        Err(e) => {
                                                            self.validation_failure(
                                                                &format!(
                                                                    "Failed to decode UTF-8 string for context-specific primitive type with raw tag 134: {e}",
                                                                ),
                                                                validation_report,
                                                                decoded_metadata,
                                                            );
                                                            return None;
                                                        },
                                                    };
                                                    // Extract the CIP19 hash and push into array
                                                    if let Some(h) = extract_cip19_hash(&addr) {
                                                        pk_addrs.push(h);
                                                    }
                                                },
                                                Err(e) => {
                                                    self.validation_failure(
                                                        &format!("Failed to process content for context-specific primitive type with raw tag 134: {e}"),
                                                        validation_report,
                                                        decoded_metadata,
                                                    );
                                                    return None;
                                                },
                                            }
                                        }
                                    }
                                },
                                Err(e) => {
                                    self.validation_failure(
                                        &format!(
                                            "Failed to parse DER sequence for Subject Alternative Name extension: {e}",
                                        ),
                                        validation_report,
                                        decoded_metadata,
                                    );
                                    return None;
                                },
                            }
                        }
                    }
                }
                // C509 Certificate
                if let Some(c509_certs) = &self.x509_chunks.0.c509_certs {
                    for cert in c509_certs {
                        match cert {
                            C509Cert::C509CertInMetadatumReference(_) => {
                                self.validation_failure(
                                    "C509 metadatum reference is currently not supported",
                                    validation_report,
                                    decoded_metadata,
                                );
                            },
                            C509Cert::C509Certificate(c509) => {
                                for exts in c509.get_tbs_cert().get_extensions().get_inner() {
                                    if exts.get_registered_oid().get_c509_oid().get_oid()
                                        == SUBJECT_ALT_NAME_OID
                                    {
                                        match exts.get_value() {
                                            c509_certificate::extensions::extension::ExtensionValue::AlternativeName(alt_name) => {
                                                match alt_name.get_inner() {
                                                    c509_certificate::extensions::alt_name::GeneralNamesOrText::GeneralNames(gn) => {
                                                        for name in gn.get_inner() {
                                                            if name.get_gn_type() == &c509_certificate::general_names::general_name::GeneralNameTypeRegistry::UniformResourceIdentifier {
                                                                match name.get_gn_value() {
                                                                    GeneralNameValue::Text(s) => {
                                                                        if let Some(h) = extract_cip19_hash(s) {
                                                                            pk_addrs.push(h);
                                                                        }
                                                                    },
                                                                    _ => {
                                                                        self.validation_failure(
                                                                            "Failed to get the value of subject alternative name",
                                                                            validation_report,
                                                                            decoded_metadata,
                                                                        );
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    },
                                                    c509_certificate::extensions::alt_name::GeneralNamesOrText::Text(_) => {
                                                        self.validation_failure(
                                                            "Failed to find C509 general names in subject alternative name",
                                                            validation_report,
                                                            decoded_metadata,
                                                        );
                                                    }
                                                }
                                            },
                                            _ => {
                                                self.validation_failure(
                                                    "Failed to get C509 subject alternative name",
                                                    validation_report,
                                                    decoded_metadata,
                                                );
                                            }
                                        }
                                    }
                                }
                            },
                        }
                    }
                }
            },
            _ => {
                self.validation_failure(
                    "Unsupported transaction era for public key validation",
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        }

        // Create TxWitness
        let witnesses = match TxWitness::new(&[txn.clone()]) {
            Ok(witnesses) => witnesses,
            Err(e) => {
                self.validation_failure(
                    &format!("Failed to create TxWitness: {e}"),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        };

        let index = match u8::try_from(txn_idx) {
            Ok(value) => value,
            Err(e) => {
                self.validation_failure(
                    &format!("Failed to convert transaction index to usize: {e}"),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        };
        Some(
            compare_key_hash(&pk_addrs, &witnesses, index)
                .map_err(|e| {
                    self.validation_failure(
                        &format!("Failed to compare public keys with witnesses {e}"),
                        validation_report,
                        decoded_metadata,
                    );
                })
                .is_ok(),
        )
    }

    /// Validate the payment key
    #[allow(clippy::too_many_lines)]
    fn validate_payment_key(
        &self, txn: &MultiEraTx, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata, txn_idx: usize, role_data: &RoleData,
    ) -> Option<bool> {
        if let Some(payment_key) = role_data.payment_key {
            match txn {
                MultiEraTx::AlonzoCompatible(tx, _) => {
                    // Handle negative payment keys (reference to tx output)
                    if payment_key < 0 {
                        let witness = match TxWitness::new(&[txn.clone()]) {
                            Ok(witness) => witness,
                            Err(e) => {
                                self.validation_failure(
                                    &format!("Failed to create TxWitness: {e}"),
                                    validation_report,
                                    decoded_metadata,
                                );
                                return None;
                            },
                        };
                        let index = match usize::try_from(payment_key.abs()) {
                            Ok(value) => value,
                            Err(e) => {
                                self.validation_failure(
                                    &format!("Failed to convert payment_key to usize: {e}"),
                                    validation_report,
                                    decoded_metadata,
                                );
                                return None;
                            },
                        };
                        let outputs = tx.transaction_body.outputs.clone();
                        if let Some(output) = outputs.get(index) {
                            return self.validate_payment_output_key_helper(
                                &output.address.to_vec(),
                                validation_report,
                                decoded_metadata,
                                &witness,
                                txn_idx,
                            );
                        }
                        self.validation_failure(
                            "Role payment key reference index is not found in transaction outputs",
                            validation_report,
                            decoded_metadata,
                        );
                        return None;
                    }
                    // Handle positive payment keys (reference to tx input)
                    let inputs = &tx.transaction_body.inputs;
                    let index = match usize::try_from(payment_key) {
                        Ok(value) => value,
                        Err(e) => {
                            self.validation_failure(
                                &format!("Failed to convert payment_key to isize: {e}"),
                                validation_report,
                                decoded_metadata,
                            );
                            return None;
                        },
                    };
                    if inputs.get(index).is_none() {
                        self.validation_failure(
                            "Role payment key reference index is not found in transaction inputs",
                            validation_report,
                            decoded_metadata,
                        );
                        return None;
                    }
                    return Some(true);
                },
                MultiEraTx::Babbage(tx) => {
                    // Negative indicates reference to tx output
                    if payment_key < 0 {
                        let index = match usize::try_from(payment_key.abs()) {
                            Ok(value) => value,
                            Err(e) => {
                                self.validation_failure(
                                    &format!("Failed to convert payment_key to usize: {e}"),
                                    validation_report,
                                    decoded_metadata,
                                );
                                return None;
                            },
                        };
                        let outputs = tx.transaction_body.outputs.clone();
                        let witness = match TxWitness::new(&[txn.clone()]) {
                            Ok(witnesses) => witnesses,
                            Err(e) => {
                                self.validation_failure(
                                    &format!("Failed to create TxWitness: {e}"),
                                    validation_report,
                                    decoded_metadata,
                                );
                                return None;
                            },
                        };
                        if let Some(output) = outputs.get(index) {
                            match output {
                                pallas::ledger::primitives::babbage::PseudoTransactionOutput::Legacy(o) => {
                                    return self.validate_payment_output_key_helper(&o.address.to_vec(), validation_report, decoded_metadata, &witness, txn_idx);
                                }
                                ,
                                pallas::ledger::primitives::babbage::PseudoTransactionOutput::PostAlonzo(o) => {
                                    return self.validate_payment_output_key_helper(&o.address.to_vec(), validation_report, decoded_metadata, &witness, txn_idx)
                                }
                                ,
                            };
                        }
                        self.validation_failure(
                            "Role payment key reference index is not found in transaction outputs",
                            validation_report,
                            decoded_metadata,
                        );
                        return None;
                    }
                    // Positive indicates reference to tx input
                    let inputs = &tx.transaction_body.inputs;
                    let index = match usize::try_from(payment_key) {
                        Ok(value) => value,
                        Err(e) => {
                            self.validation_failure(
                                &format!("Failed to convert payment_key to isize: {e}"),
                                validation_report,
                                decoded_metadata,
                            );
                            return None;
                        },
                    };
                    if inputs.get(index).is_none() {
                        self.validation_failure(
                            "Role payment key reference index is not found in transaction inputs",
                            validation_report,
                            decoded_metadata,
                        );
                        return None;
                    }
                    return Some(true);
                },
                MultiEraTx::Conway(tx) => {
                    // Negative indicates reference to tx output
                    if payment_key < 0 {
                        let index = match usize::try_from(payment_key.abs()) {
                            Ok(value) => value,
                            Err(e) => {
                                self.validation_failure(
                                    &format!("Failed to convert payment_key to usize: {e}"),
                                    validation_report,
                                    decoded_metadata,
                                );
                                return None;
                            },
                        };
                        let outputs = tx.transaction_body.outputs.clone();
                        let witness = match TxWitness::new(&[txn.clone()]) {
                            Ok(witnesses) => witnesses,
                            Err(e) => {
                                self.validation_failure(
                                    &format!("Failed to create TxWitness: {e}"),
                                    validation_report,
                                    decoded_metadata,
                                );
                                return None;
                            },
                        };

                        if let Some(output) = outputs.get(index) {
                            match output {
                                 pallas::ledger::primitives::conway::PseudoTransactionOutput::Legacy(o) => {
                                     return self.validate_payment_output_key_helper(&o.address.to_vec(), validation_report, decoded_metadata, &witness, txn_idx);
                                 },
                                 pallas::ledger::primitives::conway::PseudoTransactionOutput::PostAlonzo(o) => {
                                     return self.validate_payment_output_key_helper(&o.address.to_vec(), validation_report, decoded_metadata, &witness, txn_idx);
                                 },
                             };
                        }
                        self.validation_failure(
                            "Role payment key reference index is not found in transaction outputs",
                            validation_report,
                            decoded_metadata,
                        );
                        return None;
                    }
                    // Positive indicates reference to tx input
                    let inputs = &tx.transaction_body.inputs;
                    let index = match usize::try_from(payment_key) {
                        Ok(value) => value,
                        Err(e) => {
                            self.validation_failure(
                                &format!("Failed to convert payment_key to isize: {e}"),
                                validation_report,
                                decoded_metadata,
                            );
                            return None;
                        },
                    };
                    // Check whether the index exists in transaction inputs
                    if inputs.get(index).is_none() {
                        self.validation_failure(
                            "Role payment key reference index is not found in transaction inputs",
                            validation_report,
                            decoded_metadata,
                        );
                        return None;
                    }
                    return Some(true);
                },
                _ => {
                    self.validation_failure(
                        "Unsupported transaction era for payment key validation",
                        validation_report,
                        decoded_metadata,
                    );
                    return None;
                },
            }
        }
        Some(false)
    }

    /// Helper function for validating payment output key.
    fn validate_payment_output_key_helper(
        &self, output_address: &[u8], validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata, witness: &TxWitness, txn_idx: usize,
    ) -> Option<bool> {
        let idx = match u8::try_from(txn_idx) {
            Ok(value) => value,
            Err(e) => {
                self.validation_failure(
                    &format!("Transaction index conversion failed: {e}"),
                    validation_report,
                    decoded_metadata,
                );
                return None;
            },
        };
        // Extract the key hash from the output address
        if let Some(key) = extract_key_hash(output_address) {
            // Compare the key hash and return the result
            return Some(compare_key_hash(&[key], witness, idx).is_ok());
        }
        self.validation_failure(
            "Failed to extract payment key hash from address",
            validation_report,
            decoded_metadata,
        );
        None
    }
}
