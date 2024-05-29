//! JSON schema validation module.

use std::io::Read;

use jsonschema::{Draft, JSONSchema};
use serde::de::DeserializeOwned;

use crate::errors::Errors;

/// Invalid JSON schema error.
#[derive(thiserror::Error, Debug)]
#[error("Invalid draft 7 JSON schema, err: {0}")]
pub(crate) struct InvalidJsonSchema(String);

/// Json Schema Draft 7 Validator.
#[derive(Debug)]
pub(crate) struct SchemaValidator {
    /// JSON schema validator instance.
    schema: JSONSchema,
}

impl SchemaValidator {
    /// Create a new json schema validator from string.
    pub(crate) fn from_str(schema_str: &str) -> anyhow::Result<Self> {
        let schema =
            serde_json::from_str(schema_str).map_err(|err| InvalidJsonSchema(err.to_string()))?;

        let schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema)
            .map_err(|err| InvalidJsonSchema(err.to_string()))?;

        Ok(Self { schema })
    }

    /// Validate json instance against current schema.
    pub(crate) fn deserialize_and_validate<R: Read, T: DeserializeOwned>(
        &self, reader: R,
    ) -> anyhow::Result<T> {
        let json_val = serde_json::from_reader(reader)?;
        self.schema.validate(&json_val).map_err(|err| {
            let mut errors = Errors::new();
            for e in err {
                errors.add_err(anyhow::anyhow!("{e}"));
            }
            errors
        })?;
        let val = serde_json::from_value(json_val)?;
        Ok(val)
    }
}
