//! Cardano Improvement Proposal 509 (CIP-509) metadata module.
//! Doc Reference: <https://github.com/input-output-hk/catalyst-CIPs/tree/x509-envelope-metadata/CIP-XXXX>
//! CDDL Reference: <https://github.com/input-output-hk/catalyst-CIPs/blob/x509-envelope-metadata/CIP-XXXX/x509-envelope.cddl>

mod decode_helper;
mod rbac;
use x509_cert::{
    der::{oid::db::rfc4519::DOMAIN_COMPONENT, Decode as _},
    ext::pkix::ID_CE_SUBJECT_ALT_NAME,
};
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

/// DNS in subject alternative name.
const DNS_NAME: [u8; 10] = [9, 146, 38, 137, 147, 242, 44, 100, 4, 15];

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

/// Enum of x509 metadatum with its associated unsigned integer value.
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
        let mut x509_metadatum = Cip509::default();
        for _ in 0..map_len {
            // Use probe to peak
            let key = d.probe().u8()?;
            if let Some(key) = Cip509Int::from_repr(key) {
                // Consuming the int
                decode_u8(d, "CIP509")?;
                match key {
                    Cip509Int::Purpose => {
                        x509_metadatum.purpose = decode_bytes(d, "CIP509 purpose")?
                            .try_into()
                            .map_err(|_| decode::Error::message("Invalid data size of Purpose"))?;
                    },
                    Cip509Int::TxInputsHash => {
                        x509_metadatum.txn_inputs_hash = decode_bytes(d, "CIP509 txn inputs hash")?
                            .try_into()
                            .map_err(|_| {
                                decode::Error::message("Invalid data size of TxInputsHash")
                            })?;
                    },
                    Cip509Int::PreviousTxId => {
                        x509_metadatum.prv_tx_id = Some(
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
                        x509_metadatum.validation_signature = validation_signature.to_vec();
                    },
                }
            } else {
                // Handle the x509 chunks 10 11 12
                let x509_chunks = X509Chunks::decode(d, ctx)?;
                x509_metadatum.x509_chunks = x509_chunks;
            }
        }
        Ok(x509_metadatum)
    }
}

#[allow(clippy::module_name_repetitions)]
impl Cip509 {
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
        let cip509_metadatum = match Cip509::decode(&mut decoder, &mut ()) {
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

        // Validate the transaction inputs hash
        println!(
            "Validate {:?}",
            cip509.validate_txn_inputs_hash(txn, &mut validation_report, decoded_metadata)
        );

        // Validate the public key
        println!("slot: {:?}", _slot);
        if let Some(role_set) = cip509_metadatum.x509_chunks.0.role_set {
            for role in role_set {
                println!("role: {:?}", role.role_number);
                cip509.validate_required_signer(txn, &mut validation_report, decoded_metadata);
            }
        }
    }

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

    fn validate_required_signer(
        &self, txn: &MultiEraTx, validation_report: &mut ValidationReport,
        decoded_metadata: &DecodedMetadata,
    ) {
        match txn {
            MultiEraTx::AlonzoCompatible(tx, _) => {
                if let Some(certs) = &self.x509_chunks.0.x509_certs {
                    for cert in certs {
                        // Get the DER certificate
                        let der_cert = match x509_cert::Certificate::from_der(&cert.0) {
                            Ok(cert) => cert,
                            Err(e) => {
                                self.validation_failure(
                                    &format!("Failed to decode x509 certificate DER {e}"),
                                    validation_report,
                                    decoded_metadata,
                                );
                                return;
                            },
                        };

                        // Check the subject alternative name DNS name
                        if let Some(exts) = der_cert.tbs_certificate.extensions {
                            for ext in exts {
                                // Subject Alternative Name
                                if ext.extn_id == ID_CE_SUBJECT_ALT_NAME {
                                    let ext_bytes = ext.extn_value.as_bytes();
                                    // Check if the extension value starts with the domain name bytes
                                    if ext_bytes.starts_with(&DNS_NAME) {
                                        println!("DNS Domain: {:?}", &ext_bytes[DNS_NAME.len()..]);
                                    }
                                }
                            }
                        }
                        // Check the subject domain component
                        for rdn in der_cert.tbs_certificate.subject.0 {
                            rdn.0.iter().for_each(|attr| {
                                if attr.oid == DOMAIN_COMPONENT {
                                    println!("Domain Component: {:?}", attr.value);
                                }
                            });
                        }
                    }
                }
                println!("require {:?}", tx.transaction_body.required_signers)
            },
            MultiEraTx::Babbage(tx) => {
                println!("require {:?}", tx.transaction_body.required_signers)
            },
            MultiEraTx::Conway(tx) => {
                println!("require {:?}", tx.transaction_body.required_signers)
            },
            _ => todo!(),
        }
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
