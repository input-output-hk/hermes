//! WASM package metadata JSON

use std::io::Read;

use chrono::{DateTime, Utc};

use crate::packaging::schema_validation::SchemaValidator;

/// Metadata object.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Metadata {
    /// metadata JSON object.
    json: serde_json::Map<String, serde_json::Value>,
}

impl Metadata {
    /// WASM module metadata JSON schema.
    const METADATA_SCHEMA: &'static str =
        include_str!("../../../../schemas/hermes_module_metadata.schema.json");

    /// Create `Metadata` from reader.
    pub(crate) fn from_reader(reader: impl Read) -> anyhow::Result<Self> {
        let schema_validator = SchemaValidator::from_str(Self::METADATA_SCHEMA)?;
        let json = schema_validator.deserialize_and_validate(reader)?;
        Ok(Self { json })
    }

    /// Convert `Metadata` object to json bytes.
    pub(crate) fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let bytes = serde_json::to_vec(&self.json)?;
        Ok(bytes)
    }

    /// Set `build_date` property to the `Metadata` object.
    pub(crate) fn set_build_date(&mut self, date: DateTime<Utc>) {
        self.json
            .insert("build_date".to_string(), date.timestamp().into());
    }

    /// Set `name` property to the `Metadata` object.
    pub(crate) fn set_name(&mut self, name: &str) {
        self.json.insert("name".to_string(), name.into());
    }
}
