//! Raw Auxiliary Data Decoding

use std::sync::Arc;

use anyhow::bail;
use dashmap::DashMap;
use minicbor::{data::Type, Decoder};
use tracing::{error, warn};

/// What type of smart contract is this list.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, strum::Display, Hash)]
pub enum SmartContractType {
    /// Native smart contracts
    Native,
    /// Plutus smart contracts (with version number 1-x)
    Plutus(u64),
}

// We CAN NOT use the Pallas library metadata decoding because it does not preserve raw
// metadata values which are critical for performing operations like signature checks on
// data. So we have a bespoke metadata decoder here.
#[derive(Debug)]
pub(crate) struct RawAuxData {
    /// Metadata: key = label, value = raw metadata bytes
    metadata: DashMap<u64, Arc<Vec<u8>>>,
    /// Scripts: 1 = Native, 2 = Plutus V1, 3 = Plutus V2, 4 = Plutus V3
    scripts: DashMap<SmartContractType, Arc<Vec<Vec<u8>>>>,
}

impl RawAuxData {
    /// Create a new `RawDecodedMetadata`.
    pub(crate) fn new(aux_data: &[u8]) -> Self {
        let mut raw_decoded_data = Self {
            metadata: DashMap::new(),
            scripts: DashMap::new(),
        };

        let mut decoder = Decoder::new(aux_data);

        match decoder.datatype() {
            Ok(minicbor::data::Type::Map) => {
                if let Err(error) = Self::decode_shelley_map(&mut raw_decoded_data, &mut decoder) {
                    error!("Failed to Deserialize Shelley Metadata: {error}: {aux_data:02x?}");
                }
            },
            Ok(minicbor::data::Type::Array) => {
                if let Err(error) =
                    Self::decode_shelley_ma_array(&mut raw_decoded_data, &mut decoder)
                {
                    error!("Failed to Deserialize Shelley-MA Metadata: {error}: {aux_data:02x?}");
                }
            },
            Ok(minicbor::data::Type::Tag) => {
                if let Err(error) =
                    Self::decode_alonzo_plus_map(&mut raw_decoded_data, &mut decoder)
                {
                    error!("Failed to Deserialize Alonzo+ Metadata: {error}: {aux_data:02x?}");
                }
            },
            Ok(unexpected) => {
                error!("Unexpected datatype for Aux data: {unexpected}: {aux_data:02x?}");
            },
            Err(error) => {
                error!("Error decoding metadata: {error}: {aux_data:02x?}");
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
                // Sadly... Indefinite Maps are allowed in Cardano CBOR Encoding.
                u64::MAX
            },
            Err(error) => {
                bail!("Error decoding metadata: {error}");
            },
        };

        // debug!("Decoding shelley metadata map with {} entries", entries);

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

            // debug!("Decoded metadata key: {key}, value: {value:?}");

            let _unused = raw_decoded_data.metadata.insert(key, Arc::new(value));

            // Look for End Sentinel IF its an indefinite MAP (which we know because entries is
            // u64::MAX).
            if entries == u64::MAX {
                match decoder.datatype() {
                    Ok(Type::Break) => {
                        // Skip over the break token.
                        let _unused = decoder.skip();
                        break;
                    },
                    Ok(_) => (), // Not break, so do next loop, should be the next key.
                    Err(error) => {
                        bail!("Error checking indefinite metadata map end sentinel: {error}");
                    },
                }
            }
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

        let raw_metadata = decoder.input();

        for _entry in 0..entries {
            if contract_type == SmartContractType::Native {
                // Native Scripts are actually CBOR arrays, so capture their data as bytes for
                // later processing.
                let value_start = decoder.position();
                if let Err(error) = decoder.skip() {
                    bail!("Error decoding native script value:  {error}");
                }
                let value_end = decoder.position();
                let Some(value_slice) = raw_metadata.get(value_start..value_end) else {
                    bail!("Invalid metadata value found. Unable to extract native script slice.");
                };
                scripts.push(value_slice.to_vec());
            } else {
                let script = match decoder.bytes() {
                    Ok(script) => script,
                    Err(error) => bail!("Error decoding script data from metadata: {error}"),
                };
                scripts.push(script.to_vec());
            }
        }

        let _unused = raw_decoded_data
            .scripts
            .insert(contract_type, Arc::new(scripts));

        Ok(())
    }

    /// Get Raw metadata for a given metadata label, if it exists.
    pub(crate) fn get_metadata(&self, label: u64) -> Option<Arc<Vec<u8>>> {
        self.metadata.get(&label).map(|v| v.value().clone())
    }
}
