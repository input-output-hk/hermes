//! WASM package metadata JSON

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::packaging::{resources::ResourceTrait, schema_validation::SchemaValidator};

/// WASM module package metadata reading error.
#[derive(thiserror::Error, Debug)]
#[error("WASM module metadata json file reading errors:\n{0}")]
pub(crate) struct MeatadataReadingError(String);

/// Metadata object
#[derive(Deserialize, Debug, PartialEq, Eq)]
pub(crate) struct Metadata(serde_json::Map<String, serde_json::Value>);

impl Metadata {
    /// WASM module metadata JSON schema.
    const METADATA_SCHEMA: &'static str =
        include_str!("../../../../schemas/hermes_module_metadata.schema.json");

    /// Create `Metadata` from `Resource`.
    pub(crate) fn from_resource(resource: &impl ResourceTrait) -> anyhow::Result<Self> {
        let manifest_reader = resource.get_reader()?;

        let schema_validator = SchemaValidator::from_str(Self::METADATA_SCHEMA)?;
        let metadata = schema_validator
            .deserialize_and_validate(manifest_reader)
            .map_err(|e| MeatadataReadingError(e.to_string()))?;
        Ok(Self(metadata))
    }

    /// Set `build_date` property
    pub(crate) fn set_build_date(&mut self, build_date: DateTime<Utc>) {
        self.0
            .insert("build_date".to_string(), build_date.to_rfc3339().into());
    }
}
