//! Cardano Improvement Proposal 509 (CIP-509) metadata module.
//! Doc Reference: <https://github.com/input-output-hk/catalyst-CIPs/tree/x509-envelope-metadata/CIP-XXXX>
//! CDDL Reference: <https://github.com/input-output-hk/catalyst-CIPs/blob/x509-envelope-metadata/CIP-XXXX/x509-envelope.cddl>

use der_parser::der::parse_der_sequence;
use tracing::warn;
use utf8_decode::Decoder as Utf8Decoder;

mod decode_helper;
mod rbac;
use x509_cert::{der::Decode as _, ext::pkix::ID_CE_SUBJECT_ALT_NAME};
mod x509_chunks;

use std::sync::Arc;

use decode_helper::{
    decode_array_len, decode_bytes, decode_i64, decode_map_len, decode_string, decode_u64,
    decode_u8,
};
use minicbor::{decode, Decode, Decoder};
use pallas::{
    codec::minicbor::{Encode, Encoder},
    ledger::traverse::MultiEraTx,
};
use strum::FromRepr;
use tracing::debug;
use x509_chunks::X509Chunks;

use super::{
    raw_aux_data::RawAuxData, DecodedMetadata, DecodedMetadataItem, DecodedMetadataValues,
    ValidationReport,
};
use crate::{
    utils::{blake2b_128, compare_key_hash, extract_cip19_hash},
    witness::TxWitness,
    Network,
};

/// CIP509 label.
pub const LABEL: u64 = 509;

/// Context-specific primitive type with tag number 6 (raw_tag 134) for
/// uniform resource identifier (URI) in the subject alternative name extension.
pub(crate) const URI: u8 = 134;

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
                        cip509_metadatum.validation_signature = validation_signature.to_vec();
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

        let cip509 = match Cip509::decode(&mut decoder, &mut ()) {
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
        cip509.validate_txn_inputs_hash(txn, &mut validation_report, decoded_metadata);

        // Validate the public key
        // FIXME - Remove this
        if _slot == 68906742 {
            if let Some(role_set) = &cip509.x509_chunks.0.role_set {
                // Only care about the role number 0
                if role_set.iter().any(|role| role.role_number == 0) {
                    cip509.validate_public_key(
                        txn,
                        &mut validation_report,
                        decoded_metadata,
                        txn_idx,
                    );
                }
            }
        }
    }

    // Handle validation failure.
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

    /// Validate the public key in the certificate with witness set in transaciton.
    fn validate_public_key(
        &self, txn: &MultiEraTx, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata, txn_idx: usize,
    ) -> Option<bool> {
        let mut pk_addrs = Vec::new();
        match txn {
            MultiEraTx::AlonzoCompatible(_, _) | MultiEraTx::Babbage(_) | MultiEraTx::Conway(_) => {
                if let Some(certs) = &self.x509_chunks.0.x509_certs {
                    for cert in certs {
                        // Attempt to decode the DER certificate
                        let der_cert = match x509_cert::Certificate::from_der(&cert.0) {
                            Ok(cert) => cert,
                            Err(e) => {
                                self.validation_failure(
                                    &format!("Failed to decode x509 certificate DER {e}"),
                                    validation_report,
                                    decoded_metadata,
                                );
                                return None;
                            },
                        };

                        // Check for extensions and look for the Subject Alternative Name extension
                        if let Some(san_ext) = der_cert
                            .tbs_certificate
                            .extensions
                            .as_ref()
                            .and_then(|exts| {
                                exts.iter()
                                    .find(|ext| ext.extn_id == ID_CE_SUBJECT_ALT_NAME)
                            })
                        {
                            // Parse the Subject Alternative Name extension
                            if let Ok(parsed_seq) =
                                parse_der_sequence(san_ext.extn_value.as_bytes())
                            {
                                for data in parsed_seq.1.ref_iter() {
                                    // Look for a Context-specific primitive type with tag number 6
                                    // (raw_tag 134)
                                    if data.header.raw_tag() == Some(&[URI]) {
                                        match data.content.as_slice() {
                                            Ok(content) => {
                                                // Decode the UTF-8 string
                                                let addr: String =
                                                    Utf8Decoder::new(content.iter().copied())
                                                        .filter_map(Result::ok)
                                                        .collect();
                                                // Extract the CIP19 hash and push into array.
                                                if let Some(h) = extract_cip19_hash(&addr) {
                                                    warn!("Extracted CIP19 hash: {:?}", h);
                                                    pk_addrs.push(h);
                                                }
                                            },
                                            Err(e) => {
                                                self.validation_failure(
                                                    &format!(
                                                        "Failed to process content for context-specific primitive type with raw tag 134 {e}"
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

        // TODO - Fix this clone array
        // Create TxWitness
        let witnesses = TxWitness::new(&[txn.clone()]).expect("Failed to create TxWitness");

        compare_key_hash(pk_addrs, witnesses, txn_idx as u8)
            .map_err(|e| {
                self.validation_failure(
                    &format!("Failed to compare public keys with witnesses {e}"),
                    validation_report,
                    decoded_metadata,
                );
            })
            .ok();

        Some(true)
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
                match e.array(inputs.len() as u64) {
                    Ok(_) => {},
                    Err(e) => {
                        self.validation_failure(
                        &format!(
                            "Failed to encode array of transaction input in validate_txn_inputs_hash {e}"
                        ),
                        validation_report,
                        decoded_metadata,
                    );
                        return None;
                    },
                }
                for input in inputs {
                    match input.encode(&mut e, &mut ()) {
                        Ok(_) => {},
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
            MultiEraTx::Babbage(tx) => {
                let inputs = tx.transaction_body.inputs.clone();
                match e.array(inputs.len() as u64) {
                    Ok(_) => {},
                    Err(e) => {
                        self.validation_failure(
                        &format!(
                            "Failed to encode array of transaction input in validate_txn_inputs_hash {e}"
                        ),
                        validation_report,
                        decoded_metadata,
                    );
                        return None;
                    },
                }
                for input in inputs {
                    match input.encode(&mut e, &mut ()) {
                        Ok(_) => {},
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
            MultiEraTx::Conway(tx) => {
                let inputs = tx.transaction_body.inputs.clone();
                match e.array(inputs.len() as u64) {
                    Ok(_) => {},
                    Err(e) => {
                        self.validation_failure(
                            &format!(
                            "Failed to encode array of transaction in validate_txn_inputs_hash {e}"
                        ),
                            validation_report,
                            decoded_metadata,
                        );
                        return None;
                    },
                }
                for input in &inputs {
                    match input.encode(&mut e, &mut ()) {
                        Ok(_) => {},
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
        let inputs_hash = match blake2b_128(&buffer.clone()) {
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
        debug!(
            "txn_inputs_hash {:?} {:?}",
            inputs_hash, self.txn_inputs_hash
        );
        Some(inputs_hash == self.txn_inputs_hash)
    }
}

/// Decode any in CDDL, only support basic datatype
pub(crate) fn decode_any(d: &mut Decoder) -> Result<Vec<u8>, decode::Error> {
    match d.datatype()? {
        minicbor::data::Type::Bytes => Ok(decode_bytes(d, "Any")?.to_vec()),
        minicbor::data::Type::String => Ok(decode_string(d, "Any")?.as_bytes().to_vec()),
        minicbor::data::Type::Array => {
            let arr_len = decode_array_len(d, "Any")?;
            let mut buffer = vec![];
            for _ in 0..arr_len {
                buffer.extend_from_slice(&decode_any(d)?);
            }
            Ok(buffer)
        },
        minicbor::data::Type::U8
        | minicbor::data::Type::U16
        | minicbor::data::Type::U32
        | minicbor::data::Type::U64 => Ok(decode_u64(d, "Any")?.to_be_bytes().to_vec()),
        minicbor::data::Type::I8
        | minicbor::data::Type::I16
        | minicbor::data::Type::I32
        | minicbor::data::Type::I64 => Ok(decode_i64(d, "Any")?.to_be_bytes().to_vec()),
        _ => Err(decode::Error::message("Data type not supported")),
    }
}
