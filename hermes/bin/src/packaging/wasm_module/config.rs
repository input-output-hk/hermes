//! WASM package config JSON

use std::io::Read;

use crate::{packaging::schema_validation::SchemaValidator, sign::hash::Blake2b256};

/// Config schema object.
#[derive(Debug)]
pub(crate) struct ConfigSchema {
    /// config schema JSON object.
    json: serde_json::Map<String, serde_json::Value>,
    /// JSON schema validator.
    validator: SchemaValidator,
}

impl PartialEq for ConfigSchema {
    fn eq(&self, other: &Self) -> bool {
        self.json.eq(&other.json)
    }
}
impl Eq for ConfigSchema {}

impl ConfigSchema {
    /// Create `ConfigSchema` from reader.
    pub(crate) fn from_reader(reader: impl Read) -> anyhow::Result<Self> {
        let json: serde_json::Map<_, _> = serde_json::from_reader(reader)?;
        let validator = SchemaValidator::from_json(&serde_json::Value::Object(json.clone()))?;
        Ok(Self { json, validator })
    }

    /// Calculates a `Hash` value of the `ConfigSchema` object.
    pub(crate) fn hash(&self) -> anyhow::Result<Blake2b256> {
        let bytes = self.to_bytes()?;
        Ok(Blake2b256::hash(&bytes))
    }

    /// Convert `ConfigSchema` object to json bytes
    pub(crate) fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let bytes = serde_json::to_vec(&self.json)?;
        Ok(bytes)
    }

    /// Get JSON schema validator.
    pub(crate) fn validator(&self) -> &SchemaValidator {
        &self.validator
    }
}

/// Config object.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Config {
    /// config JSON object.
    json: serde_json::Map<String, serde_json::Value>,
}

impl Config {
    /// Create `Config` from reader.
    pub(crate) fn from_reader(
        reader: impl Read, validator: &SchemaValidator,
    ) -> anyhow::Result<Self> {
        let json = validator.deserialize_and_validate(reader)?;
        Ok(Self { json })
    }

    /// Calculates a `Hash` value of the `Config` object.
    pub(crate) fn hash(&self) -> anyhow::Result<Blake2b256> {
        let bytes = self.to_bytes()?;
        Ok(Blake2b256::hash(&bytes))
    }

    /// Convert `Config` object to json bytes
    pub(crate) fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let bytes = serde_json::to_vec(&self.json)?;
        Ok(bytes)
    }
}
