//! WASM package metadata JSON

use std::io::Read;

use crate::packaging::schema_validation::SchemaValidator;

/// WASM module package metadata reading error.
#[derive(thiserror::Error, Debug)]
#[error("WASM module metadata json file reading errors:\n{0}")]
pub(crate) struct MeatadataReadingError(String);

/// Metadata object
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Metadata {
    /// metadata JSON object
    object: serde_json::Map<String, serde_json::Value>,
}

impl Metadata {
    /// WASM module metadata JSON schema.
    const METADATA_SCHEMA: &'static str =
        include_str!("../../../../schemas/hermes_module_metadata.schema.json");

    /// Create `Metadata` from reader.
    pub(crate) fn from_reader(reader: impl Read) -> anyhow::Result<Self> {
        let schema_validator = SchemaValidator::from_str(Self::METADATA_SCHEMA)?;
        let object = schema_validator
            .deserialize_and_validate(reader)
            .map_err(|e| MeatadataReadingError(e.to_string()))?;
        Ok(Self { object })
    }
}
