//! Cardano Improvement Proposal 509 (CIP-509) metadata module.
//! Doc Reference: <https://github.com/input-output-hk/catalyst-CIPs/tree/x509-envelope-metadata/CIP-XXXX>
//! CDDL Reference: <https://github.com/input-output-hk/catalyst-CIPs/blob/x509-envelope-metadata/CIP-XXXX/x509-envelope.cddl>

mod rbac;
mod x509_chunks;

use std::sync::Arc;

use minicbor::{decode, Decode, Decoder};
use pallas::ledger::traverse::MultiEraTx;
use strum::FromRepr;
use x509_chunks::X509Chunks;

use crate::Network;

use super::{
    raw_aux_data::RawAuxData, DecodedMetadata, DecodedMetadataItem, DecodedMetadataValues,
    ValidationReport,
};

#[allow(dead_code)]
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
        decoded_metadata: &DecodedMetadata, slot: u64, _txn: &MultiEraTx,
        raw_aux_data: &RawAuxData, _chain: Network,
    ) {
        let Some(k509) = raw_aux_data.get_metadata(LABEL) else {
            return;
        };

        let mut _validation_report = ValidationReport::new();


        let cip509_slice = k509.as_slice();

        println!("cip509_slice: {:?}", cip509_slice);
        let mut decoder = Decoder::new(cip509_slice);
        if slot == 67865376 {
            let x509_metadatum = Cip509::decode(&mut decoder, &mut ());
            println!("x509_metadatum: {:?}", x509_metadatum);
            decoded_metadata.0.insert(
                LABEL,
                Arc::new(DecodedMetadataItem {
                    value: DecodedMetadataValues::Cip509(Arc::new(x509_metadatum.unwrap()).clone()),
                    report: _validation_report.clone(),
                }),
            );
        }
    }

    /// Decoding of the CIP509 metadata failed, and can not continue.
    #[allow(dead_code)]
    fn decoding_failed(
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
        let map_len = d.map().expect("ka1 ").unwrap();
        // .ok_or(decode::Error::message("Error indefinite array in Cip509"))?;
        let mut x509_metadatum = Cip509::default();
        for _ in 0..map_len {
            // Use probe to peak
            let key = d.probe().u8()?;
            if let Some(key) = Cip509Int::from_repr(key) {
                // Consuming the int
                d.u8()?;
                match key {
                    Cip509Int::Purpose => {
                        x509_metadatum.set_purpose(
                            d.bytes()?.try_into().map_err(|_| {
                                decode::Error::message("Invalid data size of Purpose")
                            })?,
                        );
                    },
                    Cip509Int::TxInputsHash => {
                        x509_metadatum.set_txn_inputs_hash(d.bytes()?.try_into().map_err(
                            |_| decode::Error::message("Invalid data size of TxInputsHash"),
                        )?);
                    },
                    Cip509Int::PreviousTxId => {
                        x509_metadatum.set_prv_tx_id(d.bytes()?.try_into().map_err(|_| {
                            decode::Error::message("Invalid data size of PreviousTxId")
                        })?);
                    },
                    Cip509Int::ValidationSignature => {
                        let validation_signature = d.bytes()?;
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
        minicbor::data::Type::Bytes => Ok(d.bytes()?.to_vec()),
        minicbor::data::Type::String => Ok(d.str()?.as_bytes().to_vec()),
        minicbor::data::Type::Array => {
            let arr_len = d.array()?.ok_or(decode::Error::message(
                "Error indefinite length in decoding any",
            ))?;
            let mut buffer = vec![];
            for _ in 0..arr_len {
                buffer.extend_from_slice(&decode_any(d)?);
            }
            Ok(buffer)
        },
        minicbor::data::Type::U8
        | minicbor::data::Type::U16
        | minicbor::data::Type::U32
        | minicbor::data::Type::U64 => Ok(d.u64()?.to_be_bytes().to_vec()),
        minicbor::data::Type::I8
        | minicbor::data::Type::I16
        | minicbor::data::Type::I32
        | minicbor::data::Type::I64 => Ok(d.i64()?.to_be_bytes().to_vec()),
        _ => Err(decode::Error::message("Data type not supported")),
    }
}
