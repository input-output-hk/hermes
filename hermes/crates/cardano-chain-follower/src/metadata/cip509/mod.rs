//! Cardano Improvement Proposal 509 (CIP-509) metadata module.
//! Doc Reference: <https://github.com/input-output-hk/catalyst-CIPs/tree/x509-envelope-metadata/CIP-XXXX>
//! CDDL Reference: <https://github.com/input-output-hk/catalyst-CIPs/blob/x509-envelope-metadata/CIP-XXXX/x509-envelope.cddl>

mod decode_helper;
mod rbac;
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
use crate::{utils::blake2b_128, Network};

/// CIP509 label.
pub const LABEL: u64 = 509;

/// CIP509 metadatum.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Cip509 {
    /// `UUIDv4` Purpose .
    purpose: [u8; 16], // (bytes .size 16)
    /// Transaction inputs hash.
    txn_inputs_hash: [u8; 16], // bytes .size 16
    /// Optional previous transaction ID.
    prv_tx_id: Option<[u8; 32]>, // bytes .size 32
    /// x509 chunks.
    x509_chunks: X509Chunks, // chunk_type => [ + x509_chunk ]
    /// Validation signature.
    validation_signature: Vec<u8>, // bytes size (1..64)
}

#[allow(clippy::module_name_repetitions)]
impl Cip509 {
    /// Create a new instance of `Cip509`.
    // fn new() -> Self {
    //     Self {
    //         purpose: [0; 16],
    //         txn_inputs_hash: [0; 16],
    //         prv_tx_id: None,
    //         x509_chunks: X509Chunks::new(CompressionAlgorithm::Raw, X509RbacMetadata::new()),
    //         validation_signature: vec![],
    //     }
    // }

    /// Set the purpose.
    fn set_purpose(&mut self, purpose: [u8; 16]) {
        self.purpose = purpose;
    }

    /// Set the transaction inputs hash.
    fn set_txn_inputs_hash(&mut self, txn_inputs_hash: [u8; 16]) {
        self.txn_inputs_hash = txn_inputs_hash;
    }

    /// Set the previous transaction ID.
    fn set_prv_tx_id(&mut self, prv_tx_id: [u8; 32]) {
        self.prv_tx_id = Some(prv_tx_id);
    }

    /// Set the x509 chunks.
    fn set_x509_chunks(&mut self, x509_chunks: X509Chunks) {
        self.x509_chunks = x509_chunks;
    }

    /// Set the validation signature.
    fn set_validation_signature(&mut self, validation_signature: Vec<u8>) {
        self.validation_signature = validation_signature;
    }

    #[allow(dead_code)]
    #[allow(clippy::too_many_lines)]
    pub(crate) fn decode_and_validate(
        decoded_metadata: &DecodedMetadata, _slot: u64, txn: &MultiEraTx,
        raw_aux_data: &RawAuxData, _chain: Network,
    ) {
        // Get the CIP509 metadata if possible
        let Some(k509) = raw_aux_data.get_metadata(LABEL) else {
            return;
        };

        let cip509 = Cip509::default();

        let mut validation_report = ValidationReport::new();

        let cip509_slice = k509.as_slice();

        // println!("cip509_slice: {:?}", cip509_slice);
        let mut decoder = Decoder::new(cip509_slice);
        // if slot == 67865376 {
        let _cip509_metadatum = match Cip509::decode(&mut decoder, &mut ()) {
            Ok(metadata) => metadata,
            Err(e) => {
                cip509.validation_failure(
                    &format!("Failed to decode CIP509 metadata {e}"),
                    &mut validation_report,
                    decoded_metadata,
                );
                return;
            },
        };

        // println!("cip509_metadatum: {:?}", cip509_metadatum.clone());

        println!(
            "Validate {:?}",
            cip509.validate_txn_inputs_hash(txn, &mut validation_report, decoded_metadata)
        );
    }

    // }

    #[allow(dead_code)]
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
                match e.array(inputs.len() as u64) {
                    Ok(_) => {},
                    Err(e) => self.validation_failure(
                        &format!(
                            "Failed to encode array of transaction input in Alonzo validate_txn_inputs_hash {e}"
                        ),
                        validation_report,
                        decoded_metadata,
                    ),
                }
                for input in inputs {
                    match input.encode(&mut e, &mut ()) {
                        Ok(_) => {},
                        Err(e) => {
                            self.validation_failure(
                            &format!(
                                "Failed to encode transaction input in Alonzo validate_txn_inputs_hash {e}"
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
                            "Failed to encode array of transaction input in Babbage validate_txn_inputs_hash {e}"
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
                                "Failed to encode transaction input in Babbage validate_txn_inputs_hash {e}"
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
                            "Failed to encode array of transaction in Conway validate_txn_inputs_hash {e}"
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
                                "Failed to encode transaction input in Conway validate_txn_inputs_hash {e}"
                            ),
                            validation_report,
                            decoded_metadata,
                        );
                            return None;
                        },
                    }
                }
            },
            _ => {},
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
        debug!("txn_inputs_hash {:?}", hex::encode(inputs_hash));
        Some(inputs_hash != self.txn_inputs_hash)
    }
}

/// Enum of x509 metadatum with its associated unsigned integer value.
#[allow(clippy::module_name_repetitions)]
#[derive(FromRepr, Debug, PartialEq)]
#[repr(u8)]
pub enum Cip509Int {
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
        let mut x509_metadatum = Cip509::default();
        for _ in 0..map_len {
            // Use probe to peak
            let key = d.probe().u8()?;
            if let Some(key) = Cip509Int::from_repr(key) {
                // Consuming the int
                decode_u8(d, "CIP509")?;
                match key {
                    Cip509Int::Purpose => {
                        x509_metadatum.set_purpose(
                            decode_bytes(d, "CIP509 purpose")?.try_into().map_err(|_| {
                                decode::Error::message("Invalid data size of Purpose")
                            })?,
                        );
                    },
                    Cip509Int::TxInputsHash => {
                        x509_metadatum.set_txn_inputs_hash(
                            decode_bytes(d, "CIP509 txn inputs hash")?
                                .try_into()
                                .map_err(|_| {
                                    decode::Error::message("Invalid data size of TxInputsHash")
                                })?,
                        );
                    },
                    Cip509Int::PreviousTxId => {
                        x509_metadatum.set_prv_tx_id(
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
                        x509_metadatum.set_validation_signature(validation_signature.to_vec());
                    },
                }
            } else {
                // Handle the x509 chunks 10 11 12
                let x509_chunks = X509Chunks::decode(d, ctx)?;
                x509_metadatum.set_x509_chunks(x509_chunks);
            }
        }
        Ok(x509_metadatum)
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
