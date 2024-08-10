//! Metadata decoding and validating.

use std::sync::Arc;

use anyhow::bail;
use crossbeam_skiplist::SkipMap;
use minicbor::Decoder;
use pallas::ledger::traverse::MultiEraBlock;
use tracing::{debug, error, warn};

mod cip36;

/// List of all validation errors (as strings) Metadata is considered Valid if this list is empty.
pub type ValidationReport = Vec<String>;

/// Possible Decoded Metadata Values.
/// Must match the key they relate too, but the consumer needs to check this.
#[derive(Debug)]
pub enum DecodedMetadataValues {
    /// Json Metadata
    Json(serde_json::Value),
    /// CIP-36/CIP-15 Catalyst Registration metadata.
    Cip36(Option<()>),
}

/// An individual decoded metadata item.
#[derive(Debug)]
pub struct DecodedMetadataItem {
    /// The decoded metadata itself.
    value: DecodedMetadataValues,
    /// Validation report for this metadata item.
    report: ValidationReport,
}

/// What type of smart contract is this list.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, strum::Display)]
pub enum SmartContractType {
    /// Native smart contracts
    Native,
    /// Plutus smart contracts (with version number 1-x)
    Plutus(u64),
}

// We CAN NOT use the Pallas library metadata decoding because it does not preserve raw metadata
// values which are critical for performing operations like signature checks on data.
// So we have a bespoke metadata decoder here.
#[derive(Debug)]
struct RawAuxData {
    /// Metadata: key = label, value = raw metadata bytes
    metadata: SkipMap<u64, Arc<Vec<u8>>>,
    /// Scripts: 1 = Native, 2 = Plutus V1, 3 = Plutus V2, 4 = Plutus V3
    scripts: SkipMap<SmartContractType, Arc<Vec<Vec<u8>>>>,
}

impl RawAuxData {
    /// Create a new `RawDecodedMetadata`.
    fn new(aux_data: &[u8]) -> Self {
        let mut raw_decoded_data = Self {
            metadata: SkipMap::new(),
            scripts: SkipMap::new(),
        };

        let mut decoder = Decoder::new(aux_data);

        match decoder.datatype() {
            Ok(minicbor::data::Type::Map) => {
                if let Err(error) = Self::decode_shelley_map(&mut raw_decoded_data, &mut decoder) {
                    error!("Failed to Deserialize Shelley Metadata: {error}");
                }
            },
            Ok(minicbor::data::Type::Array) => {
                if let Err(error) =
                    Self::decode_shelley_ma_array(&mut raw_decoded_data, &mut decoder)
                {
                    error!("Failed to Deserialize Shelley-MA Metadata: {error}");
                }
            },
            Ok(minicbor::data::Type::Tag) => {
                if let Err(error) =
                    Self::decode_alonzo_plus_map(&mut raw_decoded_data, &mut decoder)
                {
                    error!("Failed to Deserialize Alonzo+ Metadata: {error}");
                }
            },
            Ok(unexpected) => {
                error!("Unexpected datatype for Aux data: {unexpected}");
            },
            Err(error) => {
                error!("Error decoding metadata: {error}");
            },
        }

        raw_decoded_data
    }

    /// Decode the Shelley map of metadata.
    fn decode_shelley_map(
        raw_decoded_data: &mut Self, decoder: &mut minicbor::Decoder,
    ) -> anyhow::Result<()> {
        let entries = match decoder.map() {
            Ok(Some(entries)) => entries,
            Ok(None) => {
                bail!("Indefinite Map found decoding Metadata. Invalid.");
            },
            Err(error) => {
                bail!("Error decoding metadata: {error}");
            },
        };

        debug!("Decoding shelley metadata map with {} entries", entries);

        let raw_metadata = decoder.input();

        for _ in 0..entries {
            let key = match decoder.u64() {
                Ok(key) => key,
                Err(error) => {
                    bail!("Error decoding metadata key: {error}");
                },
            };
            let value_start = decoder.position();
            if let Err(error) = decoder.skip() {
                bail!("Error decoding metadata value:  {error}");
            }
            let value_end = decoder.position();
            let Some(value_slice) = raw_metadata.get(value_start..value_end) else {
                bail!("Invalid metadata value found. Unable to extract raw value slice.");
            };
            let value = value_slice.to_vec();

            debug!("Decoded metadata key: {key}, value: {value:?}");

            let _unused = raw_decoded_data.metadata.insert(key, Arc::new(value));
        }

        Ok(())
    }

    /// Decode a Shelley-MA Auxiliary Data Array
    fn decode_shelley_ma_array(
        raw_decoded_data: &mut Self, decoder: &mut minicbor::Decoder,
    ) -> anyhow::Result<()> {
        match decoder.array() {
            Ok(Some(entries)) => {
                if entries != 2 {
                    bail!(
                        "Invalid number of entries in Metadata Array. Expected 2, found {entries}."
                    );
                }
            },
            Ok(None) => {
                bail!("Indefinite Array found decoding Metadata. Invalid.");
            },
            Err(error) => {
                bail!("Error decoding metadata: {error}");
            },
        };

        // First entry is the metadata map, so just decode that now.
        Self::decode_shelley_map(raw_decoded_data, decoder)?;
        // Second entry is an array of native scripts.
        Self::decode_script_array(raw_decoded_data, decoder, SmartContractType::Native)?;

        Ok(())
    }

    /// Decode a Shelley-MA Auxiliary Data Array
    fn decode_alonzo_plus_map(
        raw_decoded_data: &mut Self, decoder: &mut minicbor::Decoder,
    ) -> anyhow::Result<()> {
        match decoder.tag() {
            Ok(tag) => {
                if tag.as_u64() != 259 {
                    bail!("Invalid tag for alonzo+ aux data. Expected 259, found {tag}.");
                }
            },
            Err(error) => {
                bail!("Error decoding tag for alonzo+ aux data: {error}");
            },
        }

        let entries = match decoder.map() {
            Ok(Some(entries)) => entries,
            Ok(None) => bail!("Indefinite Map found decoding Alonzo+ Metadata. Invalid."),
            Err(error) => bail!("Error decoding Alonzo+ Metadata: {error}"),
        };

        // iterate the map
        for _ in 0..entries {
            let aux_type_key = match decoder.u64() {
                Ok(key) => key,
                Err(error) => {
                    bail!("Error decoding Alonzo+ Metadata Aux Data Type Key: {error}");
                },
            };

            let contract_type = match aux_type_key {
                0 => {
                    if raw_decoded_data.metadata.is_empty() {
                        Self::decode_shelley_map(raw_decoded_data, decoder)?;
                        continue;
                    }
                    bail!("Multiple Alonzo+ Metadata entries found. Invalid.");
                },
                1 => SmartContractType::Native,
                _ => {
                    if aux_type_key > 4 {
                        warn!(
                            "Auxiliary Type Key > 4 detected, assuming its a plutus script > V3."
                        );
                    }
                    SmartContractType::Plutus(aux_type_key - 1)
                },
            };

            if raw_decoded_data.scripts.contains_key(&contract_type) {
                bail!("Multiple Alonzo+ Scripts of type {contract_type} found. Invalid.");
            }

            Self::decode_script_array(raw_decoded_data, decoder, contract_type)?;
        }
        Ok(())
    }

    /// Decode an array of smart contract scripts
    fn decode_script_array(
        raw_decoded_data: &mut Self, decoder: &mut minicbor::Decoder,
        contract_type: SmartContractType,
    ) -> anyhow::Result<()> {
        let mut scripts: Vec<Vec<u8>> = Vec::new();

        let entries = match decoder.array() {
            Ok(Some(entries)) => entries,
            Ok(None) => {
                bail!("Indefinite Script Array found decoding Metadata. Invalid.");
            },
            Err(error) => {
                bail!("Error decoding metadata: {error}");
            },
        };

        for _entry in 0..entries {
            let script = match decoder.bytes() {
                Ok(script) => script,
                Err(error) => bail!("Error decoding script data from metadata: {error}"),
            };
            scripts.push(script.to_vec());
        }

        let _unused = raw_decoded_data
            .scripts
            .insert(contract_type, Arc::new(scripts));

        Ok(())
    }
}

/// Decoded Metadata for a single transaction.
/// The key is the Primary Label of the Metadata.  
/// For example, CIP15/36 uses labels 61284 & 61285,
/// 61284 is the primary label, so decoded metadata
/// will be under that label.
#[derive(Debug)]
struct DecodedMetadata(SkipMap<u64, Arc<DecodedMetadataItem>>);

/// Decoded Metadata for a all transactions in a block.
/// The Key for both entries is the Transaction offset in the block.
#[derive(Debug)]
pub struct DecodedTransactionMetadata {
    /// The Raw Auxiliary Data for each transaction in the block.
    raw: SkipMap<i16, RawAuxData>,
    /// The Decoded Metadata for each transaction in the block.
    decoded: SkipMap<i16, DecodedMetadata>,
}

/// Convert a u32 to an i16. (saturate if out of range.)
fn i16_from_u32(value: u32) -> i16 {
    match value.try_into() {
        Ok(value) => value,
        Err(_) => i16::MAX,
    }
}

impl DecodedTransactionMetadata {
    /// Create a new `DecodedTransactionMetadata`.
    pub fn new(block: &MultiEraBlock) -> Self {
        let raw_aux_data = SkipMap::new();
        let decoded_metadata = SkipMap::new();

        if let Some(_metadata) = block.as_byron() {
            // Nothing to do here.
        } else if let Some(alonzo_block) = block.as_alonzo() {
            alonzo_block
                .auxiliary_data_set
                .iter()
                .for_each(|(txn_idx, metadata)| {
                    let data = metadata.raw_cbor();
                    debug!("Decoded Alonzo Metadata {txn_idx}:");
                    let txn_raw_aux_data = RawAuxData::new(data);
                    raw_aux_data.insert(i16_from_u32(*txn_idx), txn_raw_aux_data);
                });
        } else if let Some(babbage_block) = block.as_babbage() {
            babbage_block
                .auxiliary_data_set
                .iter()
                .for_each(|(txn_idx, metadata)| {
                    let data = metadata.raw_cbor();
                    debug!("Decoded Babbage Metadata {txn_idx}:");
                    let txn_raw_aux_data = RawAuxData::new(data);
                    raw_aux_data.insert(i16_from_u32(*txn_idx), txn_raw_aux_data);
                });
        } else if let Some(conway_block) = block.as_conway() {
            conway_block
                .auxiliary_data_set
                .iter()
                .for_each(|(txn_idx, metadata)| {
                    let data = metadata.raw_cbor();
                    debug!("Decoded Conway Metadata {txn_idx}:");
                    let txn_raw_aux_data = RawAuxData::new(data);
                    raw_aux_data.insert(i16_from_u32(*txn_idx), txn_raw_aux_data);
                });
        } else {
            error!("Undecodable metadata, unknown Era");
        };

        Self {
            raw: raw_aux_data,
            decoded: decoded_metadata,
        }
    }
}
