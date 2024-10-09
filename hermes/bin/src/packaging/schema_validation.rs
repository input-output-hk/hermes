//! JSON schema validation module.

use std::io::Read;

use jsonschema::{Draft, Validator};
use serde::de::DeserializeOwned;

use crate::errors::Errors;

/// JSON Schema Draft 7 Validator.
#[derive(Debug)]
pub(crate) struct SchemaValidator {
    /// JSON schema validator instance.
    schema: Validator,
}

impl SchemaValidator {
    /// Create a new json schema validator from reader.
    #[allow(dead_code)]
    pub(crate) fn from_reader<R: Read>(reader: R) -> anyhow::Result<Self> {
        let schema = serde_json::from_reader(reader)?;
        Self::from_json(&schema)
    }

    /// Create a new json schema validator from string.
    pub(crate) fn from_str(str: &str) -> anyhow::Result<Self> {
        let schema = serde_json::from_str(str)?;
        Self::from_json(&schema)
    }

    /// Create a new json schema validator from JSON value.
    pub(crate) fn from_json(json: &serde_json::Value) -> anyhow::Result<Self> {
        let schema = Validator::options()
            .with_draft(Draft::Draft7)
            .build(json)
            .map_err(|err| anyhow::anyhow!("Invalid draft 7 JSON schema:\n {err}"))?;

        Ok(Self { schema })
    }

    /// Validate JSON value against current schema.
    pub(crate) fn validate(&self, json: &serde_json::Value) -> anyhow::Result<()> {
        self.schema.validate(json).map_err(|err| {
            let mut errors = Errors::new();
            for e in err {
                errors.add_err(anyhow::anyhow!("{e}"));
            }
            errors
        })?;

        Ok(())
    }

    /// Validate and deserialize JSON value from reader against current schema.
    pub(crate) fn deserialize_and_validate<R: Read, T: DeserializeOwned>(
        &self, reader: R,
    ) -> anyhow::Result<T> {
        let json_val = serde_json::from_reader(reader)?;
        self.validate(&json_val)?;
        let val = serde_json::from_value(json_val)?;
        Ok(val)
    }
}
