//! WASM package metadata JSON

use chrono::{DateTime, Utc};

use crate::packaging::{
    resources::{bytes_resource::BytesResource, ResourceTrait},
    schema_validation::SchemaValidator,
};

/// WASM module package metadata reading error.
#[derive(thiserror::Error, Debug)]
#[error("WASM module metadata json file reading errors:\n{0}")]
pub(crate) struct MeatadataReadingError(String);

/// Metadata object
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Metadata {
    name: String,
    object: serde_json::Map<String, serde_json::Value>,
}

impl Metadata {
    /// WASM module metadata JSON schema.
    const METADATA_SCHEMA: &'static str =
        include_str!("../../../../schemas/hermes_module_metadata.schema.json");

    /// Create `Metadata` from `Resource`.
    pub(crate) fn from_resource(resource: &impl ResourceTrait) -> anyhow::Result<Self> {
        let manifest_reader = resource.get_reader()?;

        let schema_validator = SchemaValidator::from_str(Self::METADATA_SCHEMA)?;
        let object = schema_validator
            .deserialize_and_validate(manifest_reader)
            .map_err(|e| MeatadataReadingError(e.to_string()))?;
        Ok(Self {
            name: resource.name()?,
            object,
        })
    }

    /// Set `build_date` property.
    pub(crate) fn set_build_date(&mut self, build_date: DateTime<Utc>) {
        self.object
            .insert("build_date".to_string(), build_date.to_rfc3339().into());
    }

    /// Set `name` property.
    pub(crate) fn _set_name(&mut self, name: String) {
        self.object.insert("name".to_string(), name.into());
    }

    /// Build a `BytesResource`
    pub(crate) fn get_resource(&self) -> anyhow::Result<impl ResourceTrait> {
        let mut bytes = Vec::new();
        serde_json::to_writer(&mut bytes, &self.object)?;
        Ok(BytesResource::new(self.name.clone(), bytes))
    }
}
