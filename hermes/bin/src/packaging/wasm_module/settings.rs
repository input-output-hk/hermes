//! WASM package settings JSON

use std::io::Read;

use crate::{packaging::schema_validation::SchemaValidator, sign::hash::Blake2b256};

/// Settings schema object.
#[derive(Debug)]
pub(crate) struct SettingsSchema {
    /// settings schema JSON object.
    json: serde_json::Map<String, serde_json::Value>,
    /// JSON schema validator.
    validator: SchemaValidator,
}

impl PartialEq for SettingsSchema {
    fn eq(&self, other: &Self) -> bool {
        self.json.eq(&other.json)
    }
}
impl Eq for SettingsSchema {}

impl SettingsSchema {
    /// Create `SettingsSchema` from reader.
    pub(crate) fn from_reader(reader: impl Read) -> anyhow::Result<Self> {
        let json: serde_json::Map<_, _> = serde_json::from_reader(reader)?;
        let validator = SchemaValidator::from_json(&serde_json::Value::Object(json.clone()))?;
        Ok(Self { json, validator })
    }

    /// Calculates a `Hash` value of the `SettingsSchema` object.
    pub(crate) fn hash(&self) -> anyhow::Result<Blake2b256> {
        let bytes = self.to_bytes()?;
        Ok(Blake2b256::hash(&bytes))
    }

    /// Convert `SettingsSchema` object to json bytes
    pub(crate) fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let bytes = serde_json::to_vec(&self.json)?;
        Ok(bytes)
    }

    /// Get JSON schema validator.
    #[allow(dead_code)]
    pub(crate) fn validator(&self) -> &SchemaValidator {
        &self.validator
    }
}
