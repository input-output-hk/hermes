//! A generalized metadata object which is used an specified by the Hermes application
//! package or Hermes WASM module package.

use std::{
    fmt::{Debug, Display},
    io::Read,
};

use chrono::{DateTime, Utc};

use super::schema_validation::SchemaValidator;

/// Metadata object.
pub(crate) struct Metadata<T> {
    /// metadata JSON object.
    json: serde_json::Map<String, serde_json::Value>,
    /// `T` type marker.
    _phantom: std::marker::PhantomData<T>,
}

impl<T> PartialEq for Metadata<T> {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.json == other.json
    }
}
impl<T> Eq for Metadata<T> {}
impl<T> Display for Metadata<T> {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        self.json.fmt(f)
    }
}
impl<T> Debug for Metadata<T> {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        self.json.fmt(f)
    }
}

/// Traits defines a specific metadata schema specified for some type.
pub(crate) trait MetadataSchema {
    /// Metadata schema JSON string definition.
    const METADATA_SCHEMA: &'static str;
}

impl<T: MetadataSchema> Metadata<T> {
    /// Create `Metadata` from reader.
    pub(crate) fn from_reader(reader: impl Read) -> anyhow::Result<Self> {
        let schema_validator = SchemaValidator::from_str(T::METADATA_SCHEMA)?;
        let json = schema_validator.deserialize_and_validate(reader)?;
        Ok(Self {
            json,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Convert `Metadata` object to json bytes.
    pub(crate) fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let bytes = serde_json::to_vec(&self.json)?;
        Ok(bytes)
    }

    /// Get `name` property from the `Metadata` object.
    pub(crate) fn get_name(&self) -> anyhow::Result<String> {
        Ok(self
            .json
            .get("name")
            .ok_or(anyhow::anyhow!("No `name` field in the metadata object"))?
            .as_str()
            .ok_or(anyhow::anyhow!("Metadata `name` field is not a string"))?
            .to_string())
    }

    /// Set `build_date` property to the `Metadata` object.
    pub(crate) fn set_build_date(
        &mut self,
        date: DateTime<Utc>,
    ) {
        self.json
            .insert("build_date".to_string(), date.timestamp().into());
    }

    /// Set `name` property to the `Metadata` object.
    pub(crate) fn set_name(
        &mut self,
        name: &str,
    ) {
        self.json.insert("name".to_string(), name.into());
    }
}
